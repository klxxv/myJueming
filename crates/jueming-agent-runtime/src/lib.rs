//! Local-first embedded agent runtime.
//!
//! The runtime owns model configuration and non-canonical conversation history.
//! It never owns a project: all tools enter `LocalAppHost::dispatch`, and native
//! proposal approval remains outside this crate.

use chrono::Utc;
use jueming_application::{AgentCall, ContextSnapshot, LocalAppHost};
use keyring::Entry;
use reqwest::Url;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    future::Future,
    path::Path,
    pin::Pin,
    sync::{Arc, Mutex},
};
use thiserror::Error;
use tokio::sync::broadcast;
use uuid::Uuid;

const SERVICE_NAME: &str = "com.jueming.aligner.agent-runtime";
const MAX_TOOL_STEPS: u32 = 12;
const MAX_MODEL_TURNS: u32 = 16;
const MAX_OUTPUT_BYTES: usize = 96 * 1024;
const MAX_PROVIDER_RESPONSE_BYTES: usize = 512 * 1024;
const MAX_HISTORY_MESSAGES: usize = 32;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("invalid runtime configuration: {0}")]
    InvalidConfiguration(String),
    #[error("runtime is not configured")]
    NotConfigured,
    #[error("runtime run was not found: {0}")]
    RunNotFound(String),
    #[error("runtime database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("runtime provider request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("runtime serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("application tool failed: {0}")]
    Tool(String),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Loopback,
    Https,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecretStorage {
    Keyring,
    SessionMemory,
    None,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct RuntimeConfigurationInput {
    pub provider_kind: ProviderKind,
    pub endpoint: String,
    pub model: String,
    /// Native IPC accepts this value once. It is deliberately absent from every
    /// response type and from the SQLite configuration row.
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RuntimeConfiguration {
    #[serde(default)]
    pub configured: bool,
    pub provider_kind: ProviderKind,
    pub endpoint: String,
    pub model: String,
    pub secret_storage: SecretStorage,
    pub api_key_configured: bool,
}

impl Default for RuntimeConfiguration {
    fn default() -> Self {
        Self {
            provider_kind: ProviderKind::Loopback,
            configured: false,
            endpoint: String::new(),
            model: String::new(),
            secret_storage: SecretStorage::None,
            api_key_configured: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RuntimeStatus {
    pub configured: bool,
    pub provider_kind: ProviderKind,
    pub endpoint: String,
    pub model: String,
    pub secret_storage: SecretStorage,
    pub api_key_configured: bool,
    pub active_run_ids: Vec<String>,
    pub recovered_interrupted_runs: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RunRequest {
    pub project_id: String,
    pub prompt: String,
    /// Frozen by the frontend when Send is pressed. Native code validates its
    /// project, revision, binding, stable references, and selected-text range
    /// against the host before any model request is made.
    pub context: ContextSnapshot,
    #[serde(default)]
    pub session_id: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RunHandle {
    pub run_id: String,
    pub session_id: String,
    pub state: RunState,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Running,
    AwaitingApproval,
    Completed,
    Cancelled,
    Failed,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HistoryRequest {
    pub project_id: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default = "default_history_limit")]
    pub limit: usize,
}
fn default_history_limit() -> usize {
    80
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionHistory {
    pub session_id: String,
    pub project_id: String,
    pub messages: Vec<StoredMessage>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub project_id: String,
    pub last_message_at: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StoredMessage {
    pub message_id: String,
    pub role: MessageRole,
    pub content: String,
    pub context: Option<ContextSnapshot>,
    pub tool_call_id: Option<String>,
    pub created_at: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RuntimeEvent {
    pub run_id: String,
    pub session_id: String,
    pub project_id: String,
    pub kind: RuntimeEventKind,
    pub state: RunState,
    pub payload: Value,
    pub emitted_at: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEventKind {
    RunStarted,
    ToolStarted,
    ToolFinished,
    AwaitingApproval,
    RunCompleted,
    RunCancelled,
    RunFailed,
}

#[derive(Clone)]
pub struct ModelRequest {
    pub endpoint: Url,
    pub model: String,
    pub api_key: Option<String>,
    pub messages: Vec<ProviderMessage>,
    pub tools: Vec<ToolSchema>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProviderMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ModelToolCall>>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModelToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}
#[derive(Clone, Debug)]
pub struct ModelCompletion {
    pub content: String,
    pub tool_calls: Vec<ModelToolCall>,
}

pub trait ModelProvider: Send + Sync {
    fn complete<'a>(
        &'a self,
        request: ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelCompletion, RuntimeError>> + Send + 'a>>;
}

#[derive(Clone)]
pub struct OpenAiCompatibleProvider {
    client: reqwest::Client,
}
impl Default for OpenAiCompatibleProvider {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}
impl ModelProvider for OpenAiCompatibleProvider {
    fn complete<'a>(
        &'a self,
        request: ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelCompletion, RuntimeError>> + Send + 'a>> {
        Box::pin(async move {
            let url = request
                .endpoint
                .join("chat/completions")
                .map_err(|error| RuntimeError::InvalidConfiguration(error.to_string()))?;
            let tools: Vec<Value> = request
                .tools
                .iter()
                .map(|tool| {
                    Ok(json!({"type":"function","function":{"name":provider_tool_name(tool.name)?,"description":tool.description,"parameters":tool.parameters}}))
                })
                .collect::<Result<_, RuntimeError>>()?;
            let messages: Vec<Value> = request
                .messages
                .iter()
                .map(|message| {
                    let mut wire = json!({"role": message.role, "content": message.content});
                    if let Some(tool_call_id) = &message.tool_call_id {
                        wire["tool_call_id"] = Value::String(tool_call_id.clone());
                    }
                    if let Some(tool_calls) = &message.tool_calls {
                        wire["tool_calls"] = Value::Array(
                            tool_calls
                                .iter()
                                .map(|call| {
                                    Ok(json!({
                                        "id": call.id,
                                        "type": "function",
                                        "function": {
                                            "name": provider_tool_name(&call.name)?,
                                            "arguments": call.arguments.to_string(),
                                        }
                                    }))
                                })
                                .collect::<Result<_, RuntimeError>>()?,
                        );
                    }
                    Ok(wire)
                })
                .collect::<Result<_, RuntimeError>>()?;
            let mut call = self.client.post(url).json(&json!({"model":request.model,"messages":messages,"tools":tools,"tool_choice":"auto","max_tokens":2048}));
            if let Some(key) = request.api_key.as_deref().filter(|key| !key.is_empty()) {
                call = call.bearer_auth(key);
            }
            let mut response = call.send().await?.error_for_status()?;
            let mut body_bytes = Vec::new();
            while let Some(chunk) = response.chunk().await? {
                if body_bytes.len().saturating_add(chunk.len()) > MAX_PROVIDER_RESPONSE_BYTES {
                    return Err(RuntimeError::InvalidConfiguration(
                        "provider response exceeded runtime byte limit".into(),
                    ));
                }
                body_bytes.extend_from_slice(&chunk);
            }
            let body: Value = serde_json::from_slice(&body_bytes)?;
            let message = body
                .pointer("/choices/0/message")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let content = message
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let tool_calls = message
                .get("tool_calls")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|tool| {
                    let arguments = tool
                        .pointer("/function/arguments")
                        .and_then(Value::as_str)
                        .unwrap_or("{}");
                    Ok(ModelToolCall {
                        id: tool
                            .get("id")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        name: host_tool_method(
                            tool.pointer("/function/name")
                                .and_then(Value::as_str)
                                .unwrap_or_default(),
                        )?
                        .to_owned(),
                        arguments: serde_json::from_str(arguments)?,
                    })
                })
                .collect::<Result<Vec<_>, RuntimeError>>()?;
            Ok(ModelCompletion {
                content,
                tool_calls,
            })
        })
    }
}

fn provider_tool_name(host_method: &str) -> Result<&'static str, RuntimeError> {
    match host_method {
        "project.get_summary" => Ok("jueming_project_get_summary"),
        "segment.get" => Ok("jueming_segment_get"),
        "alignment.get" => Ok("jueming_alignment_get"),
        "search.execute" => Ok("jueming_search_execute"),
        "search.get_results" => Ok("jueming_search_get_results"),
        "proposal.preview_replace" => Ok("jueming_proposal_preview_replace"),
        "proposal.list" => Ok("jueming_proposal_list"),
        "proposal.reject" => Ok("jueming_proposal_reject"),
        "ui.navigate" => Ok("jueming_ui_navigate"),
        "ui.reveal" => Ok("jueming_ui_reveal"),
        "pipeline.list" => Ok("jueming_pipeline_list"),
        "pipeline.get" => Ok("jueming_pipeline_get"),
        "pipeline.preview_update" => Ok("jueming_pipeline_preview_update"),
        "pipeline.list_proposals" => Ok("jueming_pipeline_list_proposals"),
        "pipeline.execute" => Ok("jueming_pipeline_execute"),
        _ => Err(RuntimeError::InvalidConfiguration(format!(
            "no OpenAI-compatible function alias for host method {host_method}"
        ))),
    }
}

fn host_tool_method(provider_name: &str) -> Result<&'static str, RuntimeError> {
    match provider_name {
        "jueming_project_get_summary" => Ok("project.get_summary"),
        "jueming_segment_get" => Ok("segment.get"),
        "jueming_alignment_get" => Ok("alignment.get"),
        "jueming_search_execute" => Ok("search.execute"),
        "jueming_search_get_results" => Ok("search.get_results"),
        "jueming_proposal_preview_replace" => Ok("proposal.preview_replace"),
        "jueming_proposal_list" => Ok("proposal.list"),
        "jueming_proposal_reject" => Ok("proposal.reject"),
        "jueming_ui_navigate" => Ok("ui.navigate"),
        "jueming_ui_reveal" => Ok("ui.reveal"),
        "jueming_pipeline_list" => Ok("pipeline.list"),
        "jueming_pipeline_get" => Ok("pipeline.get"),
        "jueming_pipeline_preview_update" => Ok("pipeline.preview_update"),
        "jueming_pipeline_list_proposals" => Ok("pipeline.list_proposals"),
        "jueming_pipeline_execute" => Ok("pipeline.execute"),
        _ => Err(RuntimeError::InvalidConfiguration(
            "provider returned an unknown function name".into(),
        )),
    }
}

#[derive(Clone, Debug)]
pub struct ToolSchema {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value,
}
fn tools() -> Vec<ToolSchema> {
    let object =
        |properties| json!({"type":"object","properties":properties,"additionalProperties":false});
    vec![
        ToolSchema {
            name: "project.get_summary",
            description: "Read the current local project summary.",
            parameters: object(json!({})),
        },
        ToolSchema {
            name: "segment.get",
            description: "Read one stable segment UUID.",
            parameters: object(json!({"segment_id":{"type":"string"}})),
        },
        ToolSchema {
            name: "alignment.get",
            description: "Read one stable alignment UUID.",
            parameters: object(json!({"alignment_id":{"type":"string"}})),
        },
        ToolSchema {
            name: "search.execute",
            description: "Execute one atomic local search against the expected revision.",
            parameters: json!({
                "type":"object",
                "properties": {
                    "spec":{"type":"object","properties":{"query":{"type":"string"},"regex":{"type":"boolean"},"case_sensitive":{"type":"boolean"},"language_id":{"type":["string","null"]}},"required":["query","regex","case_sensitive"],"additionalProperties":false},
                    "expected_revision_id":{"type":"string"},
                    "page_size":{"type":"integer","minimum":1,"maximum":200}
                },
                "required":["spec","expected_revision_id"],
                "additionalProperties":false
            }),
        },
        ToolSchema {
            name: "search.get_results",
            description: "Read current local search results.",
            parameters: json!({"type":"object","properties":{"session_id":{"type":"string"},"cursor":{"type":"integer","minimum":0},"page_size":{"type":"integer","minimum":1,"maximum":200}},"required":["session_id"],"additionalProperties":false}),
        },
        ToolSchema {
            name: "proposal.preview_replace",
            description: "Stage a reviewable replacement proposal; it requires native approval.",
            parameters: json!({"type":"object","properties":{"query":{"type":"string"},"replacement":{"type":"string"},"regex":{"type":"boolean"},"case_sensitive":{"type":"boolean"},"language_id":{"type":["string","null"]},"selected_segment_ids":{"type":"array","items":{"type":"string"}}},"required":["query","replacement","regex","case_sensitive"],"additionalProperties":false}),
        },
        ToolSchema {
            name: "proposal.list",
            description: "List staged replacement proposals.",
            parameters: object(json!({})),
        },
        ToolSchema {
            name: "proposal.reject",
            description: "Reject a staged proposal.",
            parameters: object(json!({"proposal_id":{"type":"string"}})),
        },
        ToolSchema {
            name: "ui.navigate",
            description: "Request trusted UI navigation.",
            parameters: object(json!({"tab":{"type":"string"},"request_id":{"type":"string"}})),
        },
        ToolSchema {
            name: "ui.reveal",
            description: "Request trusted UI reveal by stable ID.",
            parameters: json!({"type":"object","properties":{"segment_id":{"type":"string"},"alignment_id":{"type":"string"},"node_id":{"type":"string"},"request_id":{"type":"string"}},"required":["request_id"],"additionalProperties":false}),
        },
        ToolSchema {
            name: "pipeline.list",
            description: "List local pipeline methods available to the current project binding.",
            parameters: object(json!({})),
        },
        ToolSchema {
            name: "pipeline.get",
            description: "Read one pipeline method by its stable method_id.",
            parameters: object(json!({"method_id":{"type":"string"}})),
        },
        ToolSchema {
            name: "pipeline.preview_update",
            description: "Stage a durable, reviewable pipeline update proposal; native UI owns approval.",
            parameters: object(json!({"request":{"type":"object"}})),
        },
        ToolSchema {
            name: "pipeline.list_proposals",
            description: "List durable pipeline update proposals for the current project binding.",
            parameters: object(json!({})),
        },
        ToolSchema {
            name: "pipeline.execute",
            description: "Execute a selected method to create a non-canonical derived artifact.",
            parameters: object(
                json!({"method_id":{"type":"string"},"method_revision_id":{"type":["string","null"]},"segment_id":{"type":"string"},"base_revision_id":{"type":"string"}}),
            ),
        },
    ]
}
fn allowed_tool(name: &str) -> bool {
    tools().iter().any(|tool| tool.name == name)
}
fn isolated_tool_arguments(run_id: &str, call: &ModelToolCall) -> Value {
    let mut arguments = call.arguments.clone();
    if call.name == "pipeline.execute" {
        if !arguments.is_object() {
            arguments = json!({});
        }
        let object = arguments.as_object_mut().expect("object just assigned");
        // The model never owns operation IDs. A unique run-scoped value prevents
        // colliding concurrent pipeline invocations from separate sessions.
        object.insert(
            "operation_id".into(),
            Value::String(format!("agent-runtime:{run_id}:{}", Uuid::now_v7())),
        );
    }
    arguments
}

pub trait AgentRuntimePort: Send + Sync {
    fn status(&self) -> Result<RuntimeStatus, RuntimeError>;
    fn configure(
        &self,
        input: RuntimeConfigurationInput,
    ) -> Result<RuntimeConfiguration, RuntimeError>;
    fn start(&self, request: RunRequest) -> Result<RunHandle, RuntimeError>;
    fn cancel(&self, run_id: &str) -> Result<(), RuntimeError>;
    fn history(&self, request: HistoryRequest) -> Result<SessionHistory, RuntimeError>;
    fn sessions(&self, project_id: &str) -> Result<Vec<SessionSummary>, RuntimeError>;
    fn subscribe(&self) -> broadcast::Receiver<RuntimeEvent>;
}

pub struct AgentRuntime {
    host: Arc<LocalAppHost>,
    provider: Arc<dyn ModelProvider>,
    store: Mutex<ConversationStore>,
    config: Mutex<RuntimeConfiguration>,
    vault: SecretVault,
    events: broadcast::Sender<RuntimeEvent>,
    active: Mutex<HashMap<String, ActiveRun>>,
    pending: Mutex<HashMap<String, PendingRun>>,
    recovered_interrupted_runs: u64,
}
struct ActiveRun {
    project_id: String,
    session_id: String,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
    cancellation_reason: Arc<Mutex<Option<String>>>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum PendingApprovalKind {
    Replace,
    PipelineMethod,
}

struct PendingRun {
    approval_kind: PendingApprovalKind,
    request: RunRequest,
    binding_id: String,
    context: ContextSnapshot,
    messages: Vec<ProviderMessage>,
    model_turns: u32,
    tool_steps: u32,
    output_bytes: usize,
    proposal_id: String,
}
struct RunLoop {
    run_id: String,
    request: RunRequest,
    binding_id: String,
    context: ContextSnapshot,
    messages: Vec<ProviderMessage>,
    model_turns: u32,
    tool_steps: u32,
    output_bytes: usize,
}

pub fn initialize(
    host: Arc<LocalAppHost>,
    app_data: impl AsRef<Path>,
) -> Result<Arc<AgentRuntime>, RuntimeError> {
    initialize_with_provider(
        host,
        app_data,
        Arc::new(OpenAiCompatibleProvider::default()),
    )
}
pub fn initialize_with_provider(
    host: Arc<LocalAppHost>,
    app_data: impl AsRef<Path>,
    provider: Arc<dyn ModelProvider>,
) -> Result<Arc<AgentRuntime>, RuntimeError> {
    let mut store = ConversationStore::open(app_data.as_ref())?;
    let config = store.load_config()?.unwrap_or_default();
    let recovered_interrupted_runs = store.recover_interrupted_runs()?;
    let (events, _) = broadcast::channel(256);
    let runtime = Arc::new(AgentRuntime {
        host: host.clone(),
        provider,
        store: Mutex::new(store),
        config: Mutex::new(config),
        vault: SecretVault::default(),
        events,
        active: Mutex::new(HashMap::new()),
        pending: Mutex::new(HashMap::new()),
        recovered_interrupted_runs,
    });
    AgentRuntime::relay_approval_events(&runtime, host.subscribe());
    Ok(runtime)
}

impl AgentRuntime {
    fn relay_approval_events(
        this: &Arc<Self>,
        mut receiver: broadcast::Receiver<jueming_application::AppEvent>,
    ) {
        let runtime = Arc::clone(this);
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                while let Ok(event) = receiver.recv().await {
                    match event.kind.as_str() {
                        "proposal_changed" => {
                            let proposal_id =
                                event.payload.get("proposal_id").and_then(Value::as_str);
                            match (
                                proposal_id,
                                event.payload.get("status").and_then(Value::as_str),
                            ) {
                                (Some(id), Some("approved")) => {
                                    runtime
                                        .resume_pending(id, PendingApprovalKind::Replace)
                                        .await;
                                }
                                (Some(id), Some("rejected")) => runtime.cancel_pending_proposal(
                                    id,
                                    PendingApprovalKind::Replace,
                                    "proposal_rejected",
                                ),
                                _ => {}
                            }
                        }
                        "pipeline_changed" => runtime.resolve_pipeline_proposals().await,
                        "project_changed" => {
                            runtime.cancel_for_project_change(
                                event.payload.get("project_id").and_then(Value::as_str),
                            );
                        }
                        _ => {}
                    }
                }
            });
        }
    }
    fn cancel_pending_proposal(
        &self,
        proposal_id: &str,
        approval_kind: PendingApprovalKind,
        reason: &str,
    ) {
        let run_id =
            self.pending
                .lock()
                .expect("pending lock")
                .iter()
                .find_map(|(run_id, pending)| {
                    (pending.proposal_id == proposal_id && pending.approval_kind == approval_kind)
                        .then(|| run_id.clone())
                });
        if let Some(run_id) = run_id {
            self.cancel_pending_run(&run_id, reason);
        }
    }
    fn cancel_for_project_change(&self, current_project_id: Option<&str>) {
        let pending_run_ids: Vec<String> = self
            .pending
            .lock()
            .expect("pending lock")
            .iter()
            .filter(|(_, pending)| Some(pending.request.project_id.as_str()) != current_project_id)
            .map(|(run_id, _)| run_id.clone())
            .collect();
        for run_id in pending_run_ids {
            self.cancel_pending_run(&run_id, "project_changed");
        }

        let active_runs: Vec<ActiveRun> = self
            .active
            .lock()
            .expect("active lock")
            .values()
            .filter(|run| Some(run.project_id.as_str()) != current_project_id)
            .map(|run| ActiveRun {
                project_id: run.project_id.clone(),
                session_id: run.session_id.clone(),
                cancelled: Arc::clone(&run.cancelled),
                cancellation_reason: Arc::clone(&run.cancellation_reason),
            })
            .collect();
        for run in active_runs {
            run.cancelled
                .store(true, std::sync::atomic::Ordering::Relaxed);
            *run.cancellation_reason.lock().expect("cancellation lock") =
                Some("project_changed".into());
        }
    }
    fn cancel_pending_run(&self, run_id: &str, reason: &str) -> bool {
        let pending = self.pending.lock().expect("pending lock").remove(run_id);
        if let Some(pending) = pending {
            let session_id = pending.request.session_id.as_deref().unwrap_or_default();
            self.finish(
                run_id,
                session_id,
                &pending.request.project_id,
                RunState::Cancelled,
                RuntimeEventKind::RunCancelled,
                json!({"reason": reason, "proposal_id": pending.proposal_id}),
            );
            true
        } else {
            false
        }
    }
    fn emit(
        &self,
        run_id: &str,
        session_id: &str,
        project_id: &str,
        kind: RuntimeEventKind,
        state: RunState,
        payload: Value,
    ) {
        let _ = self.events.send(RuntimeEvent {
            run_id: run_id.into(),
            session_id: session_id.into(),
            project_id: project_id.into(),
            kind,
            state,
            payload,
            emitted_at: Utc::now().to_rfc3339(),
        });
    }
    async fn resolve_pipeline_proposals(self: &Arc<Self>) {
        let proposal_ids: Vec<String> = self
            .pending
            .lock()
            .expect("pending lock")
            .values()
            .filter(|pending| pending.approval_kind == PendingApprovalKind::PipelineMethod)
            .map(|pending| pending.proposal_id.clone())
            .collect();
        for proposal_id in proposal_ids {
            self.resume_pending(&proposal_id, PendingApprovalKind::PipelineMethod)
                .await;
        }
    }
    async fn resume_pending(
        self: &Arc<Self>,
        proposal_id: &str,
        approval_kind: PendingApprovalKind,
    ) {
        let pending_id =
            self.pending
                .lock()
                .expect("pending lock")
                .iter()
                .find_map(|(id, pending)| {
                    (pending.proposal_id == proposal_id && pending.approval_kind == approval_kind)
                        .then(|| id.clone())
                });
        let Some(run_id) = pending_id else {
            return;
        };
        let pending = self.pending.lock().expect("pending lock").remove(&run_id);
        let Some(mut pending) = pending else {
            return;
        };
        let host = self.host.clone();
        let expected_id = pending.proposal_id.clone();
        let verification = tokio::task::spawn_blocking(move || {
            let binding = host
                .dispatch(AgentCall {
                    request_id: Uuid::now_v7().to_string(),
                    method: "app.bind_session".into(),
                    params: Default::default(),
                    binding_id: None,
                })
                .ok()?;
            let binding_id = binding.data.get("binding_id")?.as_str()?.to_owned();
            let method = match approval_kind {
                PendingApprovalKind::Replace => "proposal.list",
                PendingApprovalKind::PipelineMethod => "pipeline.list_proposals",
            };
            let proposals = host
                .dispatch(AgentCall {
                    request_id: Uuid::now_v7().to_string(),
                    method: method.into(),
                    params: Default::default(),
                    binding_id: Some(binding_id.clone()),
                })
                .ok()?;
            let status = proposals.data.as_array()?.iter().find_map(|proposal| {
                (proposal.get("proposal_id").and_then(Value::as_str) == Some(expected_id.as_str()))
                    .then(|| {
                        proposal
                            .get("status")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    })
                    .flatten()
            })?;
            Some((binding_id, status))
        })
        .await
        .ok()
        .flatten();
        if self.cancellation_reason(&run_id).is_some() {
            self.finish_cancelled(&run_id, &pending.request, &pending.proposal_id);
            return;
        }
        match verification.as_ref().map(|(_, status)| status.as_str()) {
            Some("approved") => {
                pending.binding_id = verification.expect("checked").0;
                pending.messages.push(ProviderMessage {
                    role: "tool".into(),
                    content: json!({
                        "proposal_id": pending.proposal_id,
                        "status": "approved",
                        "kind": match approval_kind {
                            PendingApprovalKind::Replace => "replace",
                            PendingApprovalKind::PipelineMethod => "pipeline_method",
                        },
                    })
                    .to_string(),
                    tool_call_id: None,
                    tool_calls: None,
                });
                self.spawn_loop(RunLoop {
                    run_id,
                    request: pending.request,
                    binding_id: pending.binding_id,
                    context: pending.context,
                    messages: pending.messages,
                    model_turns: pending.model_turns,
                    tool_steps: pending.tool_steps,
                    output_bytes: pending.output_bytes,
                });
            }
            Some("rejected") => self.finish(
                &run_id,
                pending.request.session_id.as_deref().unwrap_or_default(),
                &pending.request.project_id,
                RunState::Cancelled,
                RuntimeEventKind::RunCancelled,
                json!({"reason": "proposal_rejected", "proposal_id": pending.proposal_id}),
            ),
            Some("pending") | Some("applying") => {
                self.pending.lock().expect("pending lock").insert(run_id, pending);
            }
            _ => self.finish(
                &run_id,
                pending.request.session_id.as_deref().unwrap_or_default(),
                &pending.request.project_id,
                RunState::Failed,
                RuntimeEventKind::RunFailed,
                json!({"error": "proposal could not be verified against the current local project", "proposal_id": pending.proposal_id}),
            ),
        }
    }
    fn spawn_loop(self: &Arc<Self>, input: RunLoop) {
        let runtime = Arc::clone(self);
        tokio::spawn(async move {
            runtime.run_loop(input).await;
        });
    }
    async fn run_loop(self: Arc<Self>, input: RunLoop) {
        let RunLoop {
            run_id,
            request,
            binding_id,
            context,
            mut messages,
            mut model_turns,
            mut tool_steps,
            mut output_bytes,
        } = input;
        let session_id = request.session_id.clone().expect("session id assigned");
        loop {
            if let Some(reason) = self.cancellation_reason(&run_id) {
                self.finish(
                    &run_id,
                    &session_id,
                    &request.project_id,
                    RunState::Cancelled,
                    RuntimeEventKind::RunCancelled,
                    json!({"reason": reason}),
                );
                return;
            }
            if model_turns >= MAX_MODEL_TURNS
                || tool_steps >= MAX_TOOL_STEPS
                || output_bytes >= MAX_OUTPUT_BYTES
            {
                self.finish(
                    &run_id,
                    &session_id,
                    &request.project_id,
                    RunState::Failed,
                    RuntimeEventKind::RunFailed,
                    json!({"error":"runtime budget exceeded"}),
                );
                return;
            }
            model_turns += 1;
            let configuration = self.config.lock().expect("config lock").clone();
            let endpoint = match Url::parse(&configuration.endpoint) {
                Ok(value) => value,
                Err(error) => {
                    self.finish(
                        &run_id,
                        &session_id,
                        &request.project_id,
                        RunState::Failed,
                        RuntimeEventKind::RunFailed,
                        json!({"error":error.to_string()}),
                    );
                    return;
                }
            };
            let completion = self
                .provider
                .complete(ModelRequest {
                    endpoint,
                    model: configuration.model.clone(),
                    api_key: self.vault.get(&secret_scope(&configuration)),
                    messages: messages.clone(),
                    tools: tools(),
                })
                .await;
            let completion = match completion {
                Ok(value) => value,
                Err(error) => {
                    self.finish(
                        &run_id,
                        &session_id,
                        &request.project_id,
                        RunState::Failed,
                        RuntimeEventKind::RunFailed,
                        json!({"error":self.vault.redact(&secret_scope(&configuration), &error.to_string())}),
                    );
                    return;
                }
            };
            output_bytes += completion.content.len();
            let assistant = ProviderMessage {
                role: "assistant".into(),
                content: completion.content.clone(),
                tool_call_id: None,
                tool_calls: (!completion.tool_calls.is_empty())
                    .then_some(completion.tool_calls.clone()),
            };
            if let Err(error) = self.store_message(
                &session_id,
                &request.project_id,
                MessageRole::Assistant,
                &completion.content,
                None,
                None,
            ) {
                self.finish(
                    &run_id,
                    &session_id,
                    &request.project_id,
                    RunState::Failed,
                    RuntimeEventKind::RunFailed,
                    json!({"error":error.to_string()}),
                );
                return;
            }
            messages.push(assistant);
            if completion.tool_calls.is_empty() {
                self.finish(
                    &run_id,
                    &session_id,
                    &request.project_id,
                    RunState::Completed,
                    RuntimeEventKind::RunCompleted,
                    json!({"content":completion.content}),
                );
                return;
            }
            for call in completion.tool_calls {
                if !allowed_tool(&call.name) {
                    self.finish(
                        &run_id,
                        &session_id,
                        &request.project_id,
                        RunState::Failed,
                        RuntimeEventKind::RunFailed,
                        json!({"error":"model requested disallowed tool"}),
                    );
                    return;
                }
                tool_steps += 1;
                self.emit(
                    &run_id,
                    &session_id,
                    &request.project_id,
                    RuntimeEventKind::ToolStarted,
                    RunState::Running,
                    json!({"tool":call.name,"tool_call_id":call.id}),
                );
                let host = self.host.clone();
                let binding = binding_id.clone();
                let name = call.name.clone();
                let arguments = isolated_tool_arguments(&run_id, &call);
                let result = tokio::task::spawn_blocking(move || {
                    host.dispatch(AgentCall {
                        request_id: Uuid::now_v7().to_string(),
                        method: name,
                        params: arguments.as_object().cloned().unwrap_or_default(),
                        binding_id: Some(binding),
                    })
                })
                .await;
                let payload = match result {
                    Ok(Ok(reply)) => reply.data,
                    Ok(Err(error)) => json!({"error":{"code":error.code,"message":error.message}}),
                    Err(error) => {
                        json!({"error":{"code":"tool_join_error","message":error.to_string()}})
                    }
                };
                let encoded = match serde_json::to_string(&payload) {
                    Ok(value) => value,
                    Err(error) => error.to_string(),
                };
                let _ = self.store_message(
                    &session_id,
                    &request.project_id,
                    MessageRole::Tool,
                    &encoded,
                    None,
                    Some(&call.id),
                );
                self.emit(
                    &run_id,
                    &session_id,
                    &request.project_id,
                    RuntimeEventKind::ToolFinished,
                    RunState::Running,
                    json!({"tool":call.name,"tool_call_id":call.id,"result":payload}),
                );
                messages.push(ProviderMessage {
                    role: "tool".into(),
                    content: encoded,
                    tool_call_id: Some(call.id.clone()),
                    tool_calls: None,
                });
                if matches!(
                    call.name.as_str(),
                    "proposal.preview_replace" | "pipeline.preview_update"
                ) {
                    let proposal_id = payload
                        .get("proposal_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    if !proposal_id.is_empty() {
                        self.pending.lock().expect("pending lock").insert(
                            run_id.clone(),
                            PendingRun {
                                approval_kind: if call.name == "proposal.preview_replace" {
                                    PendingApprovalKind::Replace
                                } else {
                                    PendingApprovalKind::PipelineMethod
                                },
                                request: request.clone(),
                                binding_id,
                                context,
                                messages,
                                model_turns,
                                tool_steps,
                                output_bytes,
                                proposal_id: proposal_id.clone(),
                            },
                        );
                        self.set_run_state(&run_id, RunState::AwaitingApproval).ok();
                        self.emit(
                            &run_id,
                            &session_id,
                            &request.project_id,
                            RuntimeEventKind::AwaitingApproval,
                            RunState::AwaitingApproval,
                            json!({"proposal_id":proposal_id}),
                        );
                        return;
                    }
                }
            }
        }
    }
    fn cancellation_reason(&self, run_id: &str) -> Option<String> {
        let active = self.active.lock().expect("active lock");
        let run = active.get(run_id)?;
        run.cancelled
            .load(std::sync::atomic::Ordering::Relaxed)
            .then(|| {
                run.cancellation_reason
                    .lock()
                    .expect("cancellation lock")
                    .clone()
                    .unwrap_or_else(|| "cancelled".into())
            })
    }
    fn finish_cancelled(&self, run_id: &str, request: &RunRequest, proposal_id: &str) {
        let reason = self
            .cancellation_reason(run_id)
            .unwrap_or_else(|| "cancelled".into());
        self.finish(
            run_id,
            request.session_id.as_deref().unwrap_or_default(),
            &request.project_id,
            RunState::Cancelled,
            RuntimeEventKind::RunCancelled,
            json!({"reason": reason, "proposal_id": proposal_id}),
        );
    }
    fn finish(
        &self,
        run_id: &str,
        session_id: &str,
        project_id: &str,
        state: RunState,
        kind: RuntimeEventKind,
        payload: Value,
    ) {
        let _ = self.set_run_state(run_id, state.clone());
        self.active.lock().expect("active lock").remove(run_id);
        self.emit(run_id, session_id, project_id, kind, state, payload);
    }
    fn store_message(
        &self,
        session_id: &str,
        project_id: &str,
        role: MessageRole,
        content: &str,
        context: Option<&ContextSnapshot>,
        tool_call_id: Option<&str>,
    ) -> Result<(), RuntimeError> {
        self.store.lock().expect("store lock").append_message(
            session_id,
            project_id,
            role,
            content,
            context,
            tool_call_id,
        )
    }
    fn set_run_state(&self, run_id: &str, state: RunState) -> Result<(), RuntimeError> {
        self.store
            .lock()
            .expect("store lock")
            .set_run_state(run_id, state)
    }
}

impl AgentRuntimePort for Arc<AgentRuntime> {
    fn status(&self) -> Result<RuntimeStatus, RuntimeError> {
        let configuration = self.config.lock().expect("config lock").clone();
        Ok(RuntimeStatus {
            configured: configuration.configured,
            provider_kind: configuration.provider_kind,
            endpoint: configuration.endpoint,
            model: configuration.model,
            secret_storage: configuration.secret_storage,
            api_key_configured: configuration.api_key_configured,
            active_run_ids: self
                .active
                .lock()
                .expect("active lock")
                .keys()
                .cloned()
                .collect(),
            recovered_interrupted_runs: self.recovered_interrupted_runs,
        })
    }
    fn configure(
        &self,
        input: RuntimeConfigurationInput,
    ) -> Result<RuntimeConfiguration, RuntimeError> {
        validate_configuration(&input)?;
        let mut config = RuntimeConfiguration {
            configured: true,
            provider_kind: input.provider_kind,
            endpoint: input.endpoint.trim_end_matches('/').to_owned() + "/",
            model: input.model,
            secret_storage: SecretStorage::None,
            api_key_configured: false,
        };
        let scope = secret_scope(&config);
        if let Some(key) = input.api_key.filter(|key| !key.trim().is_empty()) {
            config.secret_storage = self.vault.set(&scope, key);
            config.api_key_configured = true;
        } else if self.vault.get(&scope).is_some() {
            config.secret_storage = self.vault.status(&scope);
            config.api_key_configured = true;
        }
        self.store
            .lock()
            .expect("store lock")
            .save_config(&config)?;
        *self.config.lock().expect("config lock") = config.clone();
        Ok(config)
    }
    fn start(&self, mut request: RunRequest) -> Result<RunHandle, RuntimeError> {
        if request.prompt.trim().is_empty() {
            return Err(RuntimeError::InvalidConfiguration(
                "prompt must not be empty".into(),
            ));
        }
        let configuration = self.config.lock().expect("config lock").clone();
        if !configuration.configured
            || configuration.endpoint.is_empty()
            || configuration.model.is_empty()
        {
            return Err(RuntimeError::NotConfigured);
        }
        if configuration.provider_kind == ProviderKind::Https && !configuration.api_key_configured {
            return Err(RuntimeError::NotConfigured);
        }
        let binding_id =
            request.context.binding_id.clone().ok_or_else(|| {
                RuntimeError::Tool("send-time context is missing binding_id".into())
            })?;
        let context_reply = self
            .host
            .dispatch(AgentCall {
                request_id: Uuid::now_v7().to_string(),
                method: "ui.get_context".into(),
                params: Default::default(),
                binding_id: Some(binding_id.clone()),
            })
            .map_err(|error| RuntimeError::Tool(error.to_string()))?;
        let validated_context: ContextSnapshot = serde_json::from_value(context_reply.data)?;
        if request.context.project_id.as_deref() != Some(request.project_id.as_str())
            || request.context.revision_id != validated_context.revision_id
            || request.context.binding_id != validated_context.binding_id
            || validated_context.project_id.as_deref() != Some(request.project_id.as_str())
        {
            return Err(RuntimeError::Tool(
                "send-time context is stale or does not match the current bound local project"
                    .into(),
            ));
        }
        if let Some(range) = &request.context.text_range
            && (range.start_utf16 > range.end_utf16
                || request.context.selected_text.encode_utf16().count() as u64
                    > range.end_utf16 - range.start_utf16)
        {
            return Err(RuntimeError::Tool(
                "send-time selected text range is invalid".into(),
            ));
        }
        for segment_id in &request.context.segment_ids {
            self.host
                .dispatch(AgentCall {
                    request_id: Uuid::now_v7().to_string(),
                    method: "segment.get".into(),
                    params: json!({"segment_id":segment_id})
                        .as_object()
                        .cloned()
                        .unwrap_or_default(),
                    binding_id: Some(binding_id.clone()),
                })
                .map_err(|error| RuntimeError::Tool(error.to_string()))?;
        }
        for alignment_id in &request.context.alignment_ids {
            self.host
                .dispatch(AgentCall {
                    request_id: Uuid::now_v7().to_string(),
                    method: "alignment.get".into(),
                    params: json!({"alignment_id":alignment_id})
                        .as_object()
                        .cloned()
                        .unwrap_or_default(),
                    binding_id: Some(binding_id.clone()),
                })
                .map_err(|error| RuntimeError::Tool(error.to_string()))?;
        }
        let context = request.context.clone();
        let session_id = request
            .session_id
            .take()
            .unwrap_or_else(|| Uuid::now_v7().to_string());
        request.session_id = Some(session_id.clone());
        let run_id = Uuid::now_v7().to_string();
        self.store_message(
            &session_id,
            &request.project_id,
            MessageRole::User,
            &request.prompt,
            Some(&context),
            None,
        )?;
        self.store.lock().expect("store lock").create_run(
            &run_id,
            &session_id,
            &request.project_id,
        )?;
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.active.lock().expect("active lock").insert(
            run_id.clone(),
            ActiveRun {
                project_id: request.project_id.clone(),
                session_id: session_id.clone(),
                cancelled,
                cancellation_reason: Arc::new(Mutex::new(None)),
            },
        );
        self.emit(
            &run_id,
            &session_id,
            &request.project_id,
            RuntimeEventKind::RunStarted,
            RunState::Running,
            json!({"context":context}),
        );
        let history = self.history(HistoryRequest {
            project_id: request.project_id.clone(),
            session_id: Some(session_id.clone()),
            limit: MAX_HISTORY_MESSAGES,
        })?;
        let mut messages=vec![ProviderMessage{role:"system".into(),content:"You are the local Jueming Aligner assistant. Use tools only for the open project. Never claim approval is complete; replacement proposals require native approval.".into(),tool_call_id:None,tool_calls:None}];
        messages.extend(history.messages.into_iter().map(|message| {
            ProviderMessage {
                role: match message.role {
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                }
                .into(),
                content: message.content,
                tool_call_id: message.tool_call_id,
                tool_calls: None,
            }
        }));
        self.spawn_loop(RunLoop {
            run_id: run_id.clone(),
            request,
            binding_id,
            context,
            messages,
            model_turns: 0,
            tool_steps: 0,
            output_bytes: 0,
        });
        Ok(RunHandle {
            run_id,
            session_id,
            state: RunState::Running,
        })
    }
    fn cancel(&self, run_id: &str) -> Result<(), RuntimeError> {
        if self.cancel_pending_run(run_id, "cancelled") {
            return Ok(());
        }
        if let Some(active) = self.active.lock().expect("active lock").get(run_id) {
            active
                .cancelled
                .store(true, std::sync::atomic::Ordering::Relaxed);
            *active
                .cancellation_reason
                .lock()
                .expect("cancellation lock") = Some("cancelled".into());
            return Ok(());
        }
        Err(RuntimeError::RunNotFound(run_id.into()))
    }
    fn history(&self, request: HistoryRequest) -> Result<SessionHistory, RuntimeError> {
        self.store.lock().expect("store lock").history(&request)
    }
    fn sessions(&self, project_id: &str) -> Result<Vec<SessionSummary>, RuntimeError> {
        self.store.lock().expect("store lock").sessions(project_id)
    }
    fn subscribe(&self) -> broadcast::Receiver<RuntimeEvent> {
        self.events.subscribe()
    }
}

fn validate_configuration(input: &RuntimeConfigurationInput) -> Result<(), RuntimeError> {
    if input.model.trim().is_empty() {
        return Err(RuntimeError::InvalidConfiguration(
            "model must not be empty".into(),
        ));
    }
    let endpoint = Url::parse(&input.endpoint)
        .map_err(|error| RuntimeError::InvalidConfiguration(error.to_string()))?;
    match input.provider_kind {
        ProviderKind::Loopback => {
            if endpoint.scheme() != "http" && endpoint.scheme() != "https" {
                return Err(RuntimeError::InvalidConfiguration(
                    "loopback endpoint must use http or https".into(),
                ));
            }
            let host = endpoint.host_str().unwrap_or_default();
            if host != "localhost" && host != "127.0.0.1" && host != "::1" {
                return Err(RuntimeError::InvalidConfiguration(
                    "loopback provider must use localhost".into(),
                ));
            }
        }
        ProviderKind::Https => {
            if endpoint.scheme() != "https" {
                return Err(RuntimeError::InvalidConfiguration(
                    "remote provider must use https".into(),
                ));
            }
        }
    };
    Ok(())
}

#[derive(Default)]
struct SecretVault {
    memory: Mutex<HashMap<String, String>>,
    storage: Mutex<HashMap<String, SecretStorage>>,
}
impl SecretVault {
    fn set(&self, scope: &str, key: String) -> SecretStorage {
        let saved = Entry::new(SERVICE_NAME, &secret_username(scope))
            .and_then(|entry| entry.set_password(&key))
            .is_ok();
        if saved {
            self.storage
                .lock()
                .expect("vault lock")
                .insert(scope.into(), SecretStorage::Keyring);
            self.memory.lock().expect("vault lock").remove(scope);
            SecretStorage::Keyring
        } else {
            self.memory
                .lock()
                .expect("vault lock")
                .insert(scope.into(), key);
            self.storage
                .lock()
                .expect("vault lock")
                .insert(scope.into(), SecretStorage::SessionMemory);
            SecretStorage::SessionMemory
        }
    }
    fn get(&self, scope: &str) -> Option<String> {
        if let Some(key) = self.memory.lock().expect("vault lock").get(scope).cloned() {
            return Some(key);
        }
        Entry::new(SERVICE_NAME, &secret_username(scope))
            .ok()
            .and_then(|entry| entry.get_password().ok())
            .filter(|key| !key.is_empty())
    }
    fn status(&self, scope: &str) -> SecretStorage {
        self.storage
            .lock()
            .expect("vault lock")
            .get(scope)
            .cloned()
            .unwrap_or(SecretStorage::None)
    }
    fn redact(&self, scope: &str, input: &str) -> String {
        self.get(scope)
            .filter(|key| !key.is_empty())
            .map_or_else(|| input.into(), |key| input.replace(&key, "[redacted]"))
    }
}

fn secret_scope(configuration: &RuntimeConfiguration) -> String {
    format!(
        "{:?}:{}",
        configuration.provider_kind, configuration.endpoint
    )
}

fn secret_username(scope: &str) -> String {
    let digest = Sha256::digest(scope.as_bytes());
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("model-api-key-{hex}")
}

struct ConversationStore {
    connection: Connection,
}
impl ConversationStore {
    fn open(app_data: &Path) -> Result<Self, RuntimeError> {
        std::fs::create_dir_all(app_data)
            .map_err(|error| RuntimeError::InvalidConfiguration(error.to_string()))?;
        let connection = Connection::open(app_data.join("agent-runtime.sqlite3"))?;
        connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK (id=1), json TEXT NOT NULL); CREATE TABLE IF NOT EXISTS runs (run_id TEXT PRIMARY KEY, session_id TEXT NOT NULL, project_id TEXT NOT NULL, state TEXT NOT NULL, created_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS messages (message_id TEXT PRIMARY KEY, session_id TEXT NOT NULL, project_id TEXT NOT NULL, role TEXT NOT NULL, content TEXT NOT NULL, context_json TEXT, tool_call_id TEXT, created_at TEXT NOT NULL);")?;
        Ok(Self { connection })
    }
    fn load_config(&mut self) -> Result<Option<RuntimeConfiguration>, RuntimeError> {
        let row: Option<String> = self
            .connection
            .query_row("SELECT json FROM settings WHERE id=1", [], |row| row.get(0))
            .optional()?;
        row.map(|json| serde_json::from_str(&json).map_err(RuntimeError::from))
            .transpose()
    }
    fn save_config(&mut self, config: &RuntimeConfiguration) -> Result<(), RuntimeError> {
        self.connection.execute("INSERT INTO settings(id,json) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET json=excluded.json",params![serde_json::to_string(config)?])?;
        Ok(())
    }
    fn create_run(&mut self, run: &str, session: &str, project: &str) -> Result<(), RuntimeError> {
        self.connection.execute("INSERT INTO runs(run_id,session_id,project_id,state,created_at) VALUES(?1,?2,?3,'running',?4)",params![run,session,project,Utc::now().to_rfc3339()])?;
        Ok(())
    }
    fn set_run_state(&mut self, run: &str, state: RunState) -> Result<(), RuntimeError> {
        self.connection.execute(
            "UPDATE runs SET state=?1 WHERE run_id=?2",
            params![
                serde_json::to_value(state)?.as_str().unwrap_or("failed"),
                run
            ],
        )?;
        Ok(())
    }
    fn recover_interrupted_runs(&mut self) -> Result<u64, RuntimeError> {
        Ok(self.connection.execute(
            "UPDATE runs SET state='failed' WHERE state IN ('running', 'awaiting_approval')",
            [],
        )? as u64)
    }
    fn append_message(
        &mut self,
        session: &str,
        project: &str,
        role: MessageRole,
        content: &str,
        context: Option<&ContextSnapshot>,
        tool_call_id: Option<&str>,
    ) -> Result<(), RuntimeError> {
        let role = serde_json::to_value(role)?
            .as_str()
            .unwrap_or("assistant")
            .to_owned();
        let context = context.map(serde_json::to_string).transpose()?;
        self.connection.execute("INSERT INTO messages(message_id,session_id,project_id,role,content,context_json,tool_call_id,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![Uuid::now_v7().to_string(),session,project,role,content,context,tool_call_id,Utc::now().to_rfc3339()])?;
        Ok(())
    }
    fn history(&mut self, request: &HistoryRequest) -> Result<SessionHistory, RuntimeError> {
        let session = match request.session_id.clone() {
            Some(session) => session,
            None => self.connection.query_row(
                "SELECT session_id FROM messages WHERE project_id=?1 ORDER BY created_at DESC LIMIT 1",
                params![request.project_id],
                |row| row.get(0),
            ).optional()?.unwrap_or_default(),
        };
        if session.is_empty() {
            return Ok(SessionHistory {
                session_id: session,
                project_id: request.project_id.clone(),
                messages: Vec::new(),
            });
        }
        let mut statement=self.connection.prepare("SELECT message_id,role,content,context_json,tool_call_id,created_at FROM messages WHERE project_id=?1 AND session_id=?2 ORDER BY created_at DESC LIMIT ?3")?;
        let rows = statement.query_map(
            params![request.project_id, session, request.limit.min(200) as i64],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )?;
        let mut messages = Vec::new();
        for row in rows {
            let (id, role, content, context, tool_call_id, created_at) = row?;
            messages.push(StoredMessage {
                message_id: id,
                role: serde_json::from_value(Value::String(role))?,
                content,
                context: context.map(|raw| serde_json::from_str(&raw)).transpose()?,
                tool_call_id,
                created_at,
            });
        }
        messages.reverse();
        Ok(SessionHistory {
            session_id: session,
            project_id: request.project_id.clone(),
            messages,
        })
    }
    fn sessions(&mut self, project_id: &str) -> Result<Vec<SessionSummary>, RuntimeError> {
        let mut statement = self.connection.prepare(
            "SELECT session_id, project_id, MAX(created_at) FROM messages WHERE project_id=?1 GROUP BY session_id, project_id ORDER BY MAX(created_at) DESC",
        )?;
        let rows = statement.query_map(params![project_id], |row| {
            Ok(SessionSummary {
                session_id: row.get(0)?,
                project_id: row.get(1)?,
                last_message_at: row.get(2)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(RuntimeError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jueming_protocol::{
        CreateProjectRequest, Encoding, ImportProfile, ImportSideRequest, SegmentationMode,
        TextInput,
    };
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    struct MockPreviewProvider;
    impl ModelProvider for MockPreviewProvider {
        fn complete<'a>(
            &'a self,
            _request: ModelRequest,
        ) -> Pin<Box<dyn Future<Output = Result<ModelCompletion, RuntimeError>> + Send + 'a>>
        {
            Box::pin(async move {
                Ok(ModelCompletion {
                    content: "I prepared a replacement for review.".into(),
                    tool_calls: vec![ModelToolCall {
                        id: "preview-1".into(),
                        name: "proposal.preview_replace".into(),
                        arguments: json!({"query":"甲","replacement":"乙"}),
                    }],
                })
            })
        }
    }

    struct MockPipelinePreviewProvider {
        update_request: Value,
        calls: AtomicUsize,
    }
    impl ModelProvider for MockPipelinePreviewProvider {
        fn complete<'a>(
            &'a self,
            _request: ModelRequest,
        ) -> Pin<Box<dyn Future<Output = Result<ModelCompletion, RuntimeError>> + Send + 'a>>
        {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            let update_request = self.update_request.clone();
            Box::pin(async move {
                if call == 0 {
                    Ok(ModelCompletion {
                        content: "Pipeline change staged for approval.".into(),
                        tool_calls: vec![ModelToolCall {
                            id: "pipeline-preview-1".into(),
                            name: "pipeline.preview_update".into(),
                            arguments: json!({"request": update_request}),
                        }],
                    })
                } else {
                    Ok(ModelCompletion {
                        content: "Pipeline approval completed.".into(),
                        tool_calls: Vec::new(),
                    })
                }
            })
        }
    }

    struct MockToolProvider(AtomicUsize);
    impl ModelProvider for MockToolProvider {
        fn complete<'a>(
            &'a self,
            _request: ModelRequest,
        ) -> Pin<Box<dyn Future<Output = Result<ModelCompletion, RuntimeError>> + Send + 'a>>
        {
            let call = self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                if call == 0 {
                    Ok(ModelCompletion {
                        content: String::new(),
                        tool_calls: vec![ModelToolCall {
                            id: "tool-1".into(),
                            name: "project.get_summary".into(),
                            arguments: json!({}),
                        }],
                    })
                } else {
                    Ok(ModelCompletion {
                        content: "summary complete".into(),
                        tool_calls: Vec::new(),
                    })
                }
            })
        }
    }

    fn test_project(path: std::path::PathBuf) -> CreateProjectRequest {
        let profile = ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine);
        CreateProjectRequest {
            project_path: path.to_string_lossy().into_owned(),
            name: "runtime".into(),
            source: ImportSideRequest {
                language_id: "zh-CN".into(),
                title: "source".into(),
                input: TextInput::Paste {
                    label: "source".into(),
                    text: "甲".into(),
                },
                profile: profile.clone(),
            },
            target: ImportSideRequest {
                language_id: "en".into(),
                title: "target".into(),
                input: TextInput::Paste {
                    label: "target".into(),
                    text: "one".into(),
                },
                profile,
            },
        }
    }

    fn object(value: Value) -> serde_json::Map<String, Value> {
        value.as_object().cloned().unwrap()
    }
    #[test]
    fn remote_requires_https() {
        let result = validate_configuration(&RuntimeConfigurationInput {
            provider_kind: ProviderKind::Https,
            endpoint: "http://example.test/v1".into(),
            model: "x".into(),
            api_key: None,
        });
        assert!(result.is_err());
    }
    #[test]
    fn loopback_rejects_remote() {
        let result = validate_configuration(&RuntimeConfigurationInput {
            provider_kind: ProviderKind::Loopback,
            endpoint: "https://example.test/v1".into(),
            model: "x".into(),
            api_key: None,
        });
        assert!(result.is_err());
    }
    #[test]
    fn durable_history_does_not_store_secret() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = ConversationStore::open(directory.path()).unwrap();
        store
            .append_message("s", "p", MessageRole::User, "hello", None, None)
            .unwrap();
        let bytes = std::fs::read(directory.path().join("agent-runtime.sqlite3")).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("api-key"));
    }
    #[test]
    fn reopening_marks_interrupted_approval_run_failed() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = ConversationStore::open(directory.path()).unwrap();
        store.create_run("run", "session", "project").unwrap();
        store
            .set_run_state("run", RunState::AwaitingApproval)
            .unwrap();
        drop(store);
        let mut reopened = ConversationStore::open(directory.path()).unwrap();
        assert_eq!(reopened.recover_interrupted_runs().unwrap(), 1);
        let state: String = reopened
            .connection
            .query_row("SELECT state FROM runs WHERE run_id='run'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(state, "failed");
    }
    #[test]
    fn endpoint_scopes_do_not_collide() {
        let first = RuntimeConfiguration {
            configured: true,
            provider_kind: ProviderKind::Loopback,
            endpoint: "http://127.0.0.1:11434/v1/".into(),
            model: "one".into(),
            secret_storage: SecretStorage::None,
            api_key_configured: false,
        };
        let second = RuntimeConfiguration {
            endpoint: "http://127.0.0.1:11435/v1/".into(),
            ..first.clone()
        };
        assert_ne!(secret_scope(&first), secret_scope(&second));
        assert_ne!(
            secret_username(&secret_scope(&first)),
            secret_username(&secret_scope(&second))
        );
    }
    fn published_context(host: &Arc<LocalAppHost>) -> (String, ContextSnapshot) {
        let binding = host
            .dispatch_native(AgentCall {
                request_id: "bind".into(),
                method: "app.bind_session".into(),
                params: Default::default(),
                binding_id: None,
            })
            .unwrap()
            .data["binding_id"]
            .as_str()
            .unwrap()
            .to_owned();
        host.dispatch_native(AgentCall { request_id: "context".into(), method: "ui.publish_context".into(), params: object(json!({"tab":"review","mode":null,"project_id":null,"revision_id":null,"binding_id":null,"window_focused":true,"focused_control":null,"selected_text":"","segment_ids":[],"alignment_ids":[],"text_range":null,"captured_at":"send-time"})), binding_id: Some(binding.clone()) }).unwrap();
        let context = serde_json::from_value(
            host.dispatch(AgentCall {
                request_id: "read-context".into(),
                method: "ui.get_context".into(),
                params: Default::default(),
                binding_id: Some(binding.clone()),
            })
            .unwrap()
            .data,
        )
        .unwrap();
        (binding, context)
    }

    fn configured_preview_runtime(
        directory: &tempfile::TempDir,
    ) -> (
        Arc<AgentRuntime>,
        Arc<LocalAppHost>,
        String,
        String,
        broadcast::Receiver<RuntimeEvent>,
    ) {
        let host = Arc::new(LocalAppHost::new());
        let snapshot = host
            .create_project(&test_project(directory.path().join("runtime.jm")))
            .unwrap();
        let (binding, context) = published_context(&host);
        let runtime = initialize_with_provider(
            Arc::clone(&host),
            directory.path().join("app-data"),
            Arc::new(MockPreviewProvider),
        )
        .unwrap();
        runtime
            .configure(RuntimeConfigurationInput {
                provider_kind: ProviderKind::Loopback,
                endpoint: "http://127.0.0.1:11434/v1".into(),
                model: "mock".into(),
                api_key: None,
            })
            .unwrap();
        let events = runtime.subscribe();
        let handle = runtime
            .start(RunRequest {
                project_id: snapshot.project.project_id.to_string(),
                prompt: "replace".into(),
                context,
                session_id: None,
            })
            .unwrap();
        (runtime, host, binding, handle.run_id, events)
    }

    async fn next_state(
        events: &mut broadcast::Receiver<RuntimeEvent>,
        state: RunState,
    ) -> RuntimeEvent {
        loop {
            let event = tokio::time::timeout(Duration::from_secs(2), events.recv())
                .await
                .expect("runtime event timed out")
                .expect("runtime event channel closed");
            if event.state == state {
                return event;
            }
        }
    }

    #[tokio::test]
    async fn mock_provider_executes_real_host_tool_and_completes() {
        let directory = tempfile::tempdir().unwrap();
        let host = Arc::new(LocalAppHost::new());
        let snapshot = host
            .create_project(&test_project(directory.path().join("runtime.jm")))
            .unwrap();
        let binding = host
            .dispatch_native(AgentCall {
                request_id: "bind".into(),
                method: "app.bind_session".into(),
                params: Default::default(),
                binding_id: None,
            })
            .unwrap()
            .data["binding_id"]
            .as_str()
            .unwrap()
            .to_owned();
        host.dispatch_native(AgentCall { request_id: "context".into(), method: "ui.publish_context".into(), params: object(json!({"tab":"review","mode":null,"project_id":null,"revision_id":null,"binding_id":null,"window_focused":true,"focused_control":null,"selected_text":"","segment_ids":[],"alignment_ids":[],"text_range":null,"captured_at":"send-time"})), binding_id: Some(binding.clone()) }).unwrap();
        let context: ContextSnapshot = serde_json::from_value(
            host.dispatch(AgentCall {
                request_id: "read-context".into(),
                method: "ui.get_context".into(),
                params: Default::default(),
                binding_id: Some(binding),
            })
            .unwrap()
            .data,
        )
        .unwrap();
        let runtime = initialize_with_provider(
            host,
            directory.path().join("app-data"),
            Arc::new(MockToolProvider(AtomicUsize::new(0))),
        )
        .unwrap();
        runtime
            .configure(RuntimeConfigurationInput {
                provider_kind: ProviderKind::Loopback,
                endpoint: "http://127.0.0.1:11434/v1".into(),
                model: "mock".into(),
                api_key: None,
            })
            .unwrap();
        let mut events = runtime.subscribe();
        runtime
            .start(RunRequest {
                project_id: snapshot.project.project_id.to_string(),
                prompt: "summarize".into(),
                context,
                session_id: None,
            })
            .unwrap();
        let mut completed = false;
        for _ in 0..4 {
            let event = tokio::time::timeout(Duration::from_secs(2), events.recv())
                .await
                .unwrap()
                .unwrap();
            assert_ne!(event.state, RunState::AwaitingApproval);
            completed |= event.state == RunState::Completed;
            if completed {
                break;
            }
        }
        assert!(completed, "mock tool loop did not complete");
    }

    #[tokio::test]
    async fn rejected_native_proposal_cancels_awaiting_mock_run() {
        let directory = tempfile::tempdir().unwrap();
        let (runtime, host, binding, run_id, mut events) = configured_preview_runtime(&directory);
        let awaiting = next_state(&mut events, RunState::AwaitingApproval).await;
        let proposal_id = awaiting.payload["proposal_id"].as_str().unwrap().to_owned();
        host.dispatch_native(AgentCall {
            request_id: "reject".into(),
            method: "proposal.reject".into(),
            params: object(json!({"proposal_id": proposal_id})),
            binding_id: Some(binding),
        })
        .unwrap();
        let cancelled = next_state(&mut events, RunState::Cancelled).await;
        assert_eq!(cancelled.run_id, run_id);
        assert_eq!(cancelled.payload["reason"], "proposal_rejected");
        assert!(!runtime.status().unwrap().active_run_ids.contains(&run_id));
    }

    #[tokio::test]
    async fn project_change_cancels_awaiting_mock_run() {
        let directory = tempfile::tempdir().unwrap();
        let (runtime, host, _binding, run_id, mut events) = configured_preview_runtime(&directory);
        next_state(&mut events, RunState::AwaitingApproval).await;
        host.create_project(&test_project(directory.path().join("other.jm")))
            .unwrap();
        let cancelled = next_state(&mut events, RunState::Cancelled).await;
        assert_eq!(cancelled.run_id, run_id);
        assert_eq!(cancelled.payload["reason"], "project_changed");
        assert!(!runtime.status().unwrap().active_run_ids.contains(&run_id));
    }

    #[tokio::test]
    async fn explicit_cancel_cancels_awaiting_mock_run() {
        let directory = tempfile::tempdir().unwrap();
        let (runtime, _host, _binding, run_id, mut events) = configured_preview_runtime(&directory);
        next_state(&mut events, RunState::AwaitingApproval).await;
        runtime.cancel(&run_id).unwrap();
        let cancelled = next_state(&mut events, RunState::Cancelled).await;
        assert_eq!(cancelled.run_id, run_id);
        assert_eq!(cancelled.payload["reason"], "cancelled");
        assert!(!runtime.status().unwrap().active_run_ids.contains(&run_id));
    }

    #[test]
    fn pipeline_execution_operation_ids_are_runtime_scoped() {
        let call = ModelToolCall {
            id: "model-reused-call".into(),
            name: "pipeline.execute".into(),
            arguments: json!({"method_id":"method","segment_id":"segment","base_revision_id":"revision","operation_id":"model-value"}),
        };
        let first = isolated_tool_arguments("run-a", &call);
        let second = isolated_tool_arguments("run-b", &call);
        assert_ne!(first["operation_id"], second["operation_id"]);
        assert!(
            first["operation_id"]
                .as_str()
                .unwrap()
                .starts_with("agent-runtime:run-a:")
        );
        assert_ne!(first["operation_id"], "model-value");
    }

    #[tokio::test]
    async fn openai_wire_uses_aliases_and_maps_response_back_to_host_method() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (request_sender, request_receiver) = mpsc::channel();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            let header_end = loop {
                let read = stream.read(&mut buffer).unwrap();
                request.extend_from_slice(&buffer[..read]);
                if let Some(end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                    break end + 4;
                }
            };
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    line.split_once(':').and_then(|(name, value)| {
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                })
                .unwrap();
            while request.len() < header_end + content_length {
                let read = stream.read(&mut buffer).unwrap();
                request.extend_from_slice(&buffer[..read]);
            }
            request_sender
                .send(serde_json::from_slice::<Value>(&request[header_end..]).unwrap())
                .unwrap();
            let body = json!({"choices":[{"message":{"content":"", "tool_calls":[{
                "id":"wire-call-1", "type":"function", "function":{
                    "name":"jueming_project_get_summary", "arguments":"{}"
                }
            }]}}]})
            .to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });

        let completion = OpenAiCompatibleProvider::default()
            .complete(ModelRequest {
                endpoint: Url::parse(&format!("http://{address}/v1/")).unwrap(),
                model: "wire-test".into(),
                api_key: None,
                messages: vec![ProviderMessage {
                    role: "assistant".into(),
                    content: "calling summary".into(),
                    tool_call_id: None,
                    tool_calls: Some(vec![ModelToolCall {
                        id: "historic-call".into(),
                        name: "project.get_summary".into(),
                        arguments: json!({}),
                    }]),
                }],
                tools: tools(),
            })
            .await
            .unwrap();
        let request = request_receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        server.join().unwrap();
        let tool_names: Vec<&str> = request["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["function"]["name"].as_str().unwrap())
            .collect();
        assert!(tool_names.iter().all(|name| !name.contains('.')));
        assert!(tool_names.contains(&"jueming_project_get_summary"));
        assert_eq!(
            request["messages"][0]["tool_calls"][0]["function"]["name"],
            "jueming_project_get_summary"
        );
        assert_eq!(completion.tool_calls[0].name, "project.get_summary");
    }

    #[tokio::test]
    async fn pipeline_method_proposal_waits_for_native_approval_then_resumes() {
        let directory = tempfile::tempdir().unwrap();
        let host = Arc::new(LocalAppHost::new());
        let snapshot = host
            .create_project(&test_project(directory.path().join("pipeline-runtime.jm")))
            .unwrap();
        let (binding, context) = published_context(&host);
        let method = host
            .dispatch_native(AgentCall {
                request_id: "create-pipeline-method".into(),
                method: "pipeline.create_default".into(),
                params: object(json!({"name":"runtime pipeline"})),
                binding_id: Some(binding.clone()),
            })
            .unwrap()
            .data;
        let update_request = json!({
            "method_id": method["method_id"],
            "base_method_revision_id": method["current"]["method_revision_id"],
            "name": "runtime pipeline approved",
            "plan": method["current"]["plan"],
        });
        let runtime = initialize_with_provider(
            Arc::clone(&host),
            directory.path().join("app-data"),
            Arc::new(MockPipelinePreviewProvider {
                update_request,
                calls: AtomicUsize::new(0),
            }),
        )
        .unwrap();
        runtime
            .configure(RuntimeConfigurationInput {
                provider_kind: ProviderKind::Loopback,
                endpoint: "http://127.0.0.1:11434/v1".into(),
                model: "mock".into(),
                api_key: None,
            })
            .unwrap();
        let mut events = runtime.subscribe();
        runtime
            .start(RunRequest {
                project_id: snapshot.project.project_id.to_string(),
                prompt: "rename the pipeline".into(),
                context,
                session_id: None,
            })
            .unwrap();
        let awaiting = next_state(&mut events, RunState::AwaitingApproval).await;
        let proposal_id = awaiting.payload["proposal_id"].as_str().unwrap().to_owned();
        host.dispatch_native(AgentCall {
            request_id: "approve-pipeline-method".into(),
            method: "pipeline.approve_update".into(),
            params: object(json!({"proposal_id": proposal_id})),
            binding_id: Some(binding),
        })
        .unwrap();
        let completed = next_state(&mut events, RunState::Completed).await;
        assert_eq!(completed.payload["content"], "Pipeline approval completed.");
    }
}
