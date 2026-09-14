use crate::types::{
    APPLICATION_CONTRACT_VERSION, AgentCall, AgentReply, AppError, AppEvent, ContextSnapshot,
    SearchSpec,
};
use jueming_kernel::{KernelError, KernelService};
use jueming_protocol::{
    AlignmentId, CreateProjectRequest, ProjectId, ProjectSnapshot, ReplaceApplyRequest,
    ReplacePreviewRequest, ReplacePreviewResponse, RevisionId, SearchSegmentsRequest,
    SearchSegmentsResponse, SegmentId,
};
use jueming_storage::write_bytes_atomic;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

mod native;
mod pipeline;
mod pool;
mod research;

const MAX_SEARCH_PAGE: usize = 200;
const MAX_RETRY_CACHE: usize = 256;
const MAX_SEARCH_SESSIONS: usize = 32;
const MAX_UI_ACTIONS: usize = 64;
const MAX_BINDINGS: usize = 128;

#[cfg(test)]
static JOURNAL_FAIL_ON_WRITE: AtomicUsize = AtomicUsize::new(usize::MAX);

#[derive(Clone)]
struct OpenProject {
    writer_lock: Arc<std::fs::File>,
    path: PathBuf,
    snapshot: ProjectSnapshot,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ReplaceProposal {
    proposal_id: String,
    canonical_operation_id: String,
    status: ProposalStatus,
    preview: ReplacePreviewResponse,
    request: ReplaceApplyRequest,
    #[serde(default)]
    applied_revision_id: Option<String>,
    #[serde(default)]
    applied_operation_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ProposalStatus {
    Pending,
    Applying,
    Approved,
    Rejected,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ProposalJournal {
    project_id: String,
    proposals: Vec<ReplaceProposal>,
}

#[derive(Clone)]
struct SearchSession {
    session_id: String,
    revision_id: RevisionId,
    response: SearchSegmentsResponse,
}

#[derive(Clone)]
struct CachedReply {
    fingerprint: String,
    reply: AgentReply,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PendingUiAction {
    operation_id: String,
    request_id: String,
    project_id: Option<ProjectId>,
    revision_id: Option<RevisionId>,
    /// The initiating transport binding is audit metadata, never an ACK
    /// capability: external and native clients have distinct bindings.
    binding_id: Option<String>,
    kind: String,
    payload: Value,
    status: String,
}

#[derive(Clone)]
struct OperationOwner {
    binding_id: Option<String>,
    project_id: Option<ProjectId>,
}

#[derive(Default)]
struct HostState {
    current: Option<OpenProject>,
    sequence: u64,
    bindings: HashMap<String, Binding>,
    binding_order: VecDeque<String>,
    native_context: Option<ContextSnapshot>,
    search_spec: SearchSpec,
    searches: HashMap<String, SearchSession>,
    latest_search: Option<String>,
    proposals: Vec<ReplaceProposal>,
    operations: HashMap<String, String>,
    operation_owners: HashMap<String, OperationOwner>,
    pending_ui_actions: HashMap<String, PendingUiAction>,
    preview_request_claims: HashMap<String, (String, String)>,
    preview_claim_order: VecDeque<String>,
    completed_requests: HashMap<String, CachedReply>,
    retry_order: VecDeque<String>,
}

struct Binding {
    project_id: ProjectId,
    revision_id: RevisionId,
}

/// Serializes every application-visible project operation and owns the only
/// in-memory project snapshot.  The host is intentionally synchronous: Tauri
/// and external transports place it on a blocking worker rather than creating
/// competing snapshot holders.
pub struct LocalAppHost {
    feature: Arc<crate::feature::FeatureManager>,
    research: Arc<crate::research::ResearchService>,
    graph: Arc<crate::graph::GraphService>,
    kernel: KernelService,
    state: Mutex<HostState>,
    events: broadcast::Sender<AppEvent>,
}

impl Default for LocalAppHost {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalAppHost {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            feature: Arc::new(crate::feature::FeatureManager::default()),
            research: Arc::new(crate::research::ResearchService::default()),
            graph: Arc::new(crate::graph::GraphService::default()),
            kernel: KernelService,
            state: Mutex::new(HostState::default()),
            events,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.events.subscribe()
    }

    /// External / MCP dispatch. Canonical proposal approval is rejected here.
    pub fn dispatch(&self, call: AgentCall) -> Result<AgentReply, AppError> {
        self.dispatch_as(call, false)
    }

    /// Trusted native UI dispatch. Only this boundary can approve proposals.
    pub fn dispatch_native(&self, call: AgentCall) -> Result<AgentReply, AppError> {
        self.dispatch_as(call, true)
    }

    pub fn projection(&self) -> Result<AgentReply, AppError> {
        self.projection_for(None, "projection".into())
    }

    fn projection_for(
        &self,
        binding_id: Option<&str>,
        request_id: String,
    ) -> Result<AgentReply, AppError> {
        let state = self.lock()?;
        if binding_id.is_some() {
            self.ensure_binding(&state, binding_id)?;
        }
        let project = state
            .current
            .as_ref()
            .map(|current| json!({"project": current.snapshot.project}));
        let search_results = state
            .latest_search
            .as_ref()
            .and_then(|id| state.searches.get(id))
            .map(|session| session.response.clone());
        Ok(AgentReply {
            request_id,
            sequence: state.sequence.to_string(),
            data: json!({ "binding_id": binding_id, "project": project, "context": state.native_context, "search_spec": state.search_spec, "search_results": search_results, "proposals": state.proposals, "pending_ui_actions": state.pending_ui_actions.values().filter(|action| action.status == "accepted").collect::<Vec<_>>() }),
        })
    }

    pub fn create_project(
        &self,
        request: &CreateProjectRequest,
    ) -> Result<ProjectSnapshot, AppError> {
        self.load_project(Path::new(&request.project_path), Some(request))
    }

    pub fn open_project(
        &self,
        project_path: impl AsRef<Path>,
    ) -> Result<ProjectSnapshot, AppError> {
        self.load_project(project_path.as_ref(), None)
    }

    fn load_project(
        &self,
        path: &Path,
        create: Option<&CreateProjectRequest>,
    ) -> Result<ProjectSnapshot, AppError> {
        let mut state = self.lock()?;
        if create.is_none() && !path.join("project.json").is_file() {
            return Err(AppError::new("not_found", "Project does not exist."));
        }
        let layout = jueming_storage::ProjectLayout::new(path)
            .map_err(|e| AppError::new("io_error", e.to_string()))?;
        layout
            .ensure()
            .map_err(|e| AppError::new("io_error", e.to_string()))?;
        let path =
            std::fs::canonicalize(path).map_err(|e| AppError::new("io_error", e.to_string()))?;
        let writer_lock = if let Some(current) = state
            .current
            .as_ref()
            .filter(|current| current.path == path)
        {
            current.writer_lock.clone()
        } else {
            Arc::new(
                layout
                    .acquire_writer_lock()
                    .map_err(|e| AppError::new("project_locked", e.to_string()))?,
            )
        };
        if create.is_some() && path.join("project.json").exists() {
            return Err(AppError::new(
                "project_exists",
                "An existing project cannot be overwritten.",
            ));
        }
        let snapshot = match create {
            Some(request) => self.kernel.create_project(request),
            None => self.kernel.open_project(&path),
        }
        .map_err(kernel_error)?;
        self.install_project(&mut state, path, snapshot.clone(), writer_lock, "native")?;
        Ok(snapshot)
    }

    pub fn current_snapshot(&self) -> Result<ProjectSnapshot, AppError> {
        self.lock()?
            .current
            .as_ref()
            .map(|current| current.snapshot.clone())
            .ok_or_else(AppError::unavailable)
    }

    pub fn current_path(&self) -> Result<PathBuf, AppError> {
        self.lock()?
            .current
            .as_ref()
            .map(|current| current.path.clone())
            .ok_or_else(AppError::unavailable)
    }

    pub fn read<T>(
        &self,
        operation: impl FnOnce(&KernelService, &ProjectSnapshot, &Path) -> Result<T, KernelError>,
    ) -> Result<T, AppError> {
        let state = self.lock()?;
        let current = state.current.as_ref().ok_or_else(AppError::unavailable)?;
        operation(&self.kernel, &current.snapshot, &current.path).map_err(kernel_error)
    }

    /// Runs a native canonical mutation while holding the sole project lock.
    /// A revision event is emitted only after Kernel persistence succeeds and
    /// the in-memory snapshot has advanced.
    pub fn mutate(
        &self,
        origin: &str,
        operation: impl FnOnce(
            &KernelService,
            &ProjectSnapshot,
            &Path,
        ) -> Result<ProjectSnapshot, KernelError>,
    ) -> Result<ProjectSnapshot, AppError> {
        let mut state = self.lock()?;
        self.mutate_locked(&mut state, origin, operation)
    }

    fn mutate_locked(
        &self,
        state: &mut HostState,
        origin: &str,
        operation: impl FnOnce(
            &KernelService,
            &ProjectSnapshot,
            &Path,
        ) -> Result<ProjectSnapshot, KernelError>,
    ) -> Result<ProjectSnapshot, AppError> {
        let current = state
            .current
            .as_ref()
            .ok_or_else(AppError::unavailable)?
            .clone();
        let next =
            operation(&self.kernel, &current.snapshot, &current.path).map_err(kernel_error)?;
        if next.project.project_id != current.snapshot.project.project_id {
            return Err(AppError::new(
                "project_mismatch",
                "A mutation returned a different project.",
            ));
        }
        let revised =
            next.project.current_revision_id != current.snapshot.project.current_revision_id;
        state.current = Some(OpenProject {
            writer_lock: current.writer_lock,
            path: current.path,
            snapshot: next.clone(),
        });
        if revised {
            for binding in state.bindings.values_mut() {
                binding.revision_id = next.project.current_revision_id;
            }
            if let Some(context) = state.native_context.as_mut() {
                context.revision_id = Some(next.project.current_revision_id.to_string());
                context.segment_ids.clear();
                context.alignment_ids.clear();
                context.selected_text.clear();
                context.text_range = None;
                context.window_focused = false;
                context.focused_control = None;
            }
            self.publish_locked(state, "revision_advanced", None, origin, json!({"project_id": next.project.project_id, "revision_id": next.project.current_revision_id}));
        }
        Ok(next)
    }

    pub fn flush(&self) -> Result<(), AppError> {
        self.read(|kernel, snapshot, path| kernel.save_project(path, snapshot))
    }
    pub fn clear_cache(&self) -> Result<u64, AppError> {
        self.read(|kernel, _, path| kernel.clear_cache(path))
    }

    fn install_project(
        &self,
        state: &mut HostState,
        path: PathBuf,
        snapshot: ProjectSnapshot,
        writer_lock: Arc<std::fs::File>,
        origin: &str,
    ) -> Result<(), AppError> {
        let mut proposals = read_journal(&path, snapshot.project.project_id)?;
        if reconcile_applying_proposals(&snapshot, &mut proposals) {
            persist_journal(
                &OpenProject {
                    writer_lock: writer_lock.clone(),
                    path: path.clone(),
                    snapshot: snapshot.clone(),
                },
                &proposals,
            )?;
        }
        state.current = Some(OpenProject {
            writer_lock,
            path,
            snapshot: snapshot.clone(),
        });
        state.bindings.clear();
        state.binding_order.clear();
        state.native_context = None;
        state.searches.clear();
        state.latest_search = None;
        state.proposals = proposals;
        state.completed_requests.clear();
        state.retry_order.clear();
        state.pending_ui_actions.clear();
        state.preview_request_claims.clear();
        state.preview_claim_order.clear();
        state.operations.clear();
        state.operation_owners.clear();
        self.publish_locked(state, "project_changed", None, origin, json!({"project_id": snapshot.project.project_id, "revision_id": snapshot.project.current_revision_id}));
        Ok(())
    }

    fn dispatch_as(&self, call: AgentCall, trusted_native: bool) -> Result<AgentReply, AppError> {
        if !trusted_native && crate::research_method(&call.method).is_some_and(|d| d.native_only) {
            return Err(AppError::new(
                "native_ui_required",
                "This operation is available only to the trusted native client",
            ));
        }
        if !trusted_native
            && matches!(
                call.method.as_str(),
                "ui.publish_context" | "ui.ack" | "app.get_projection"
            )
        {
            return Err(AppError::new(
                "native_ui_required",
                "This UI synchronization method is only available to the trusted native client.",
            ));
        }
        let cache_key = format!(
            "{}:{}",
            if trusted_native { "native" } else { "external" },
            call.request_id
        );
        if scoped_method(&call.method) {
            let state = self.lock()?;
            self.ensure_binding(&state, call.binding_id.as_deref())?;
        }
        let fingerprint = request_fingerprint(&call, trusted_native)?;
        let retryable = retryable_method(&call.method);
        if retryable && let Some(cached) = self.lock()?.completed_requests.get(&cache_key).cloned()
        {
            if cached.fingerprint != fingerprint {
                return Err(AppError::new(
                    "request_id_conflict",
                    "A request ID may only be retried with identical method, binding, and parameters.",
                ));
            }
            return Ok(cached.reply);
        }
        let request_id = call.request_id.clone();
        let data = match call.method.as_str() {
            method if method.starts_with("pool.") => {
                self.pool_call(method, call.params, call.binding_id.as_deref())?
            }
            method if research::is_research_method(method) => self.research_call(
                method,
                Value::Object(call.params),
                call.binding_id.as_deref(),
                trusted_native,
                &request_id,
            )?,
            "app.describe" => {
                json!({"contract_version": APPLICATION_CONTRACT_VERSION, "approval": "native_ui_only"})
            }
            "app.bind_session" => self.bind_session(call.params)?,
            "app.get_projection" => {
                return self.projection_for(call.binding_id.as_deref(), request_id);
            }
            "ui.get_context" | "ui.get_selection" => {
                self.get_context(call.binding_id.as_deref())?
            }
            "ui.publish_context" => {
                self.publish_context(call.binding_id.as_deref(), call.params)?
            }
            "ui.navigate" => self.request_navigation(
                call.binding_id.as_deref(),
                call.params,
                "navigation_requested",
                &request_id,
            )?,
            "ui.reveal" => self.request_navigation(
                call.binding_id.as_deref(),
                call.params,
                "reveal_requested",
                &request_id,
            )?,
            "ui.ack" => self.ack_ui_action(call.binding_id.as_deref(), call.params)?,
            "search.validate_regex" => self.validate_regex(call.params)?,
            "search.set_spec" => self.set_search_spec(call.binding_id.as_deref(), call.params)?,
            "search.execute" => self.execute_search(call.binding_id.as_deref(), call.params)?,
            "search.get_results" => self.search_results(call.binding_id.as_deref(), call.params)?,
            "project.get_summary" => self
                .read(|kernel, snapshot, _| kernel.summarize(snapshot))
                .and_then(to_value)?,
            "segment.get" => self.get_segment(call.binding_id.as_deref(), call.params)?,
            "alignment.get" => self.get_alignment(call.binding_id.as_deref(), call.params)?,
            "proposal.preview_replace" => self.preview_replace(
                call.binding_id.as_deref(),
                call.params,
                &cache_key,
                &fingerprint,
            )?,
            "proposal.list" => to_value(self.lock()?.proposals.clone())?,
            "proposal.approve" if trusted_native => {
                self.approve_replace(call.binding_id.as_deref(), call.params)?
            }
            "proposal.approve" => {
                return Err(AppError::new(
                    "native_approval_required",
                    "Replace proposals may only be approved by trusted native UI.",
                ));
            }
            "proposal.reject" => self.reject_replace(call.binding_id.as_deref(), call.params)?,
            "operation.get" => self.operation(call.binding_id.as_deref(), call.params, false)?,
            "operation.cancel" => self.operation(call.binding_id.as_deref(), call.params, true)?,
            method if method.starts_with("pipeline.") => self.pipeline_call(
                method,
                call.params,
                call.binding_id.as_deref(),
                trusted_native,
            )?,
            _ => {
                return Err(AppError::new(
                    "unknown_method",
                    format!("Unsupported application method: {}", call.method),
                ));
            }
        };
        let sequence = self.lock()?.sequence.to_string();
        let reply = AgentReply {
            request_id,
            data,
            sequence,
        };
        if retryable {
            self.cache_reply(cache_key, fingerprint, reply)
        } else {
            Ok(reply)
        }
    }

    fn cache_reply(
        &self,
        cache_key: String,
        fingerprint: String,
        reply: AgentReply,
    ) -> Result<AgentReply, AppError> {
        let mut state = self.lock()?;
        state.completed_requests.insert(
            cache_key.clone(),
            CachedReply {
                fingerprint,
                reply: reply.clone(),
            },
        );
        state.retry_order.push_back(cache_key);
        while state.retry_order.len() > MAX_RETRY_CACHE {
            if let Some(expired) = state.retry_order.pop_front() {
                state.completed_requests.remove(&expired);
            }
        }
        Ok(reply)
    }

    fn bind_session(&self, params: serde_json::Map<String, Value>) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Expected {
            #[serde(default)]
            project_id: Option<ProjectId>,
            #[serde(default)]
            revision_id: Option<RevisionId>,
        }
        let expected: Expected = from_params(params)?;
        let mut state = self.lock()?;
        let current = state.current.as_ref().ok_or_else(AppError::unavailable)?;
        let id = jueming_protocol::OperationId::new().to_string();
        let project_id = current.snapshot.project.project_id;
        let revision_id = current.snapshot.project.current_revision_id;
        if expected.project_id.is_some_and(|value| value != project_id)
            || expected
                .revision_id
                .is_some_and(|value| value != revision_id)
        {
            return Err(AppError::new(
                "stale_revision",
                "The project changed before this session could be bound; reload the projection.",
            ));
        }
        state.bindings.insert(
            id.clone(),
            Binding {
                project_id,
                revision_id,
            },
        );
        state.binding_order.push_back(id.clone());
        while state.binding_order.len() > MAX_BINDINGS {
            if let Some(expired) = state.binding_order.pop_front() {
                state.bindings.remove(&expired);
            }
        }
        Ok(json!({"binding_id": id, "project_id": project_id, "revision_id": revision_id}))
    }

    fn ensure_binding<'a>(
        &self,
        state: &'a HostState,
        binding_id: Option<&str>,
    ) -> Result<&'a Binding, AppError> {
        let id = binding_id.ok_or_else(|| {
            AppError::new(
                "binding_required",
                "This operation requires a current project binding.",
            )
        })?;
        let binding = state.bindings.get(id).ok_or_else(|| {
            AppError::new("stale_binding", "The project changed; bind a new session.")
        })?;
        let current = state.current.as_ref().ok_or_else(AppError::unavailable)?;
        if binding.project_id != current.snapshot.project.project_id
            || binding.revision_id != current.snapshot.project.current_revision_id
        {
            return Err(AppError::new(
                "stale_revision",
                "The bound revision is no longer current; rebind and review the latest projection.",
            ));
        }
        Ok(binding)
    }

    fn get_context(&self, binding_id: Option<&str>) -> Result<Value, AppError> {
        let state = self.lock()?;
        if state.current.is_some() {
            self.ensure_binding(&state, binding_id)?;
        }
        let context = state.native_context.clone().ok_or_else(|| {
            AppError::new(
                "ui_context_unavailable",
                "The native UI has not published a context for this project.",
            )
        })?;
        to_value(context)
    }

    fn publish_context(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        let mut context: ContextSnapshot = from_params(params)?;
        let mut state = self.lock()?;
        if state.current.is_none() {
            if binding_id.is_some() || context.project_id.is_some() || context.revision_id.is_some()
            {
                return Err(AppError::new(
                    "app_context_invalid",
                    "Loading-state context must have no binding, project ID, or revision ID.",
                ));
            }
            context.binding_id = None;
            context.captured_at = chrono::Utc::now().to_rfc3339();
            state.native_context = Some(context.clone());
            self.publish_locked(
                &mut state,
                "context_changed",
                None,
                "ui",
                to_value(context.clone())?,
            );
            return to_value(context);
        }
        self.ensure_binding(&state, binding_id)?;
        let current = state.current.as_ref().expect("binding requires current");
        for id in &context.segment_ids {
            let id = SegmentId::from_str(id)
                .map_err(|_| AppError::invalid("context segment_ids must contain stable UUIDs"))?;
            if !current
                .snapshot
                .segments
                .iter()
                .any(|segment| segment.segment_id == id)
            {
                return Err(AppError::new(
                    "segment_not_found",
                    "Native context references a segment outside the current project.",
                ));
            }
        }
        for id in &context.alignment_ids {
            let id = AlignmentId::from_str(id).map_err(|_| {
                AppError::invalid("context alignment_ids must contain stable UUIDs")
            })?;
            if !current
                .snapshot
                .alignments
                .iter()
                .any(|alignment| alignment.alignment_id == id)
            {
                return Err(AppError::new(
                    "alignment_not_found",
                    "Native context references an alignment outside the current project.",
                ));
            }
        }
        context.binding_id = binding_id.map(str::to_owned);
        context.project_id = Some(current.snapshot.project.project_id.to_string());
        context.revision_id = Some(current.snapshot.project.current_revision_id.to_string());
        context.captured_at = chrono::Utc::now().to_rfc3339();
        state.native_context = Some(context.clone());
        self.publish_locked(
            &mut state,
            "context_changed",
            binding_id,
            "ui",
            to_value(context.clone())?,
        );
        to_value(context)
    }

    fn request_navigation(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
        kind: &str,
        request_id: &str,
    ) -> Result<Value, AppError> {
        let mut state = self.lock()?;
        if state.current.is_some() {
            self.ensure_binding(&state, binding_id)?;
        }
        let current = state.current.as_ref();
        let action_binding = binding_id.map(str::to_owned);
        let action_project_id = current.map(|current| current.snapshot.project.project_id);
        let action_revision_id =
            current.map(|current| current.snapshot.project.current_revision_id);
        let mut payload = params;
        if kind == "navigation_requested" {
            let tab = payload
                .get("tab")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::invalid("ui.navigate requires tab"))?;
            if !matches!(
                tab,
                "project"
                    | "review"
                    | "edit"
                    | "order"
                    | "history"
                    | "search"
                    | "pipeline"
                    | "settings"
            ) {
                return Err(AppError::invalid("ui.navigate tab is not supported"));
            }
            if current.is_none() && !matches!(tab, "project" | "settings") {
                return Err(AppError::new(
                    "project_unavailable",
                    "Only project and settings navigation is available before a project opens.",
                ));
            }
        } else {
            let current = current.ok_or_else(AppError::unavailable)?;
            let segment_id = payload.get("segment_id").and_then(Value::as_str);
            let alignment_id = payload.get("alignment_id").and_then(Value::as_str);
            let node_id = payload.get("node_id").and_then(Value::as_str);
            if segment_id.is_none() && alignment_id.is_none() && node_id.is_none() {
                return Err(AppError::invalid(
                    "ui.reveal requires segment_id, alignment_id, or node_id",
                ));
            }
            if let Some(id) = segment_id {
                let id = SegmentId::from_str(id)
                    .map_err(|_| AppError::invalid("segment_id must be a stable UUID"))?;
                if !current
                    .snapshot
                    .segments
                    .iter()
                    .any(|segment| segment.segment_id == id)
                {
                    return Err(AppError::new(
                        "segment_not_found",
                        "Segment does not exist in the current project.",
                    ));
                }
            }
            if let Some(id) = alignment_id {
                let id = AlignmentId::from_str(id)
                    .map_err(|_| AppError::invalid("alignment_id must be a stable UUID"))?;
                if !current
                    .snapshot
                    .alignments
                    .iter()
                    .any(|alignment| alignment.alignment_id == id)
                {
                    return Err(AppError::new(
                        "alignment_not_found",
                        "Alignment does not exist in the current project.",
                    ));
                }
            }
            if let Some(id) = node_id {
                uuid::Uuid::parse_str(id)
                    .map_err(|_| AppError::invalid("node_id must be a stable UUID"))?;
            }
        }
        payload.insert("request_id".into(), Value::String(request_id.into()));
        payload.insert(
            "project_id".into(),
            action_project_id
                .map(|id| Value::String(id.to_string()))
                .unwrap_or(Value::Null),
        );
        payload.insert(
            "revision_id".into(),
            action_revision_id
                .map(|id| Value::String(id.to_string()))
                .unwrap_or(Value::Null),
        );
        let operation_id = jueming_protocol::OperationId::new().to_string();
        payload.insert("operation_id".into(), Value::String(operation_id.clone()));
        let payload = Value::Object(payload);
        if state.pending_ui_actions.len() >= MAX_UI_ACTIONS {
            return Err(AppError::new(
                "operation_backpressure",
                "Too many pending UI actions; acknowledge or cancel an earlier action.",
            ));
        }
        state
            .operations
            .insert(operation_id.clone(), "accepted".into());
        state.operation_owners.insert(
            operation_id.clone(),
            OperationOwner {
                binding_id: action_binding.clone(),
                project_id: action_project_id,
            },
        );
        state.pending_ui_actions.insert(
            operation_id.clone(),
            PendingUiAction {
                operation_id: operation_id.clone(),
                request_id: request_id.into(),
                project_id: action_project_id,
                revision_id: action_revision_id,
                binding_id: action_binding,
                kind: kind.into(),
                payload: payload.clone(),
                status: "accepted".into(),
            },
        );
        self.publish_locked(&mut state, kind, binding_id, "agent", payload.clone());
        self.publish_locked(
            &mut state,
            "operation_changed",
            binding_id,
            "agent",
            json!({"operation_id": operation_id, "status":"accepted"}),
        );
        Ok(json!({"operation_id": operation_id, "status":"accepted", "payload": payload}))
    }

    fn ack_ui_action(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Input {
            operation_id: String,
            request_id: String,
            #[serde(default)]
            status: Option<String>,
        }
        let input: Input = from_params(params)?;
        let mut state = self.lock()?;
        let status = input.status.unwrap_or_else(|| "ui_applied".into());
        if !matches!(status.as_str(), "ui_applied" | "failed" | "cancelled") {
            return Err(AppError::invalid(
                "ui.ack status must be ui_applied, failed, or cancelled",
            ));
        }
        let mut action = state
            .pending_ui_actions
            .get(&input.operation_id)
            .cloned()
            .ok_or_else(|| {
                if state.operations.contains_key(&input.operation_id) {
                    AppError::new(
                        "operation_terminal",
                        "This UI operation has already reached a terminal state.",
                    )
                } else {
                    AppError::new(
                        "operation_not_found",
                        "No pending native UI operation has this ID.",
                    )
                }
            })?;
        if action.request_id != input.request_id {
            return Err(AppError::new(
                "ack_mismatch",
                "The acknowledgement request ID does not match the UI operation.",
            ));
        }
        match (action.project_id, action.revision_id) {
            (Some(project_id), Some(revision_id)) => {
                self.ensure_binding(&state, binding_id)?;
                let current = state
                    .current
                    .as_ref()
                    .expect("validated binding requires project");
                if current.snapshot.project.project_id != project_id
                    || current.snapshot.project.current_revision_id != revision_id
                {
                    return Err(AppError::new(
                        "stale_operation_epoch",
                        "The UI operation belongs to an earlier project or revision epoch.",
                    ));
                }
            }
            (None, None) if state.current.is_none() && binding_id.is_none() => {}
            _ => {
                return Err(AppError::new(
                    "stale_operation_epoch",
                    "The app-only UI operation no longer belongs to the current application epoch.",
                ));
            }
        }
        action.status = status.clone();
        state.pending_ui_actions.remove(&input.operation_id);
        state
            .operations
            .insert(input.operation_id.clone(), status.clone());
        self.publish_locked(
            &mut state,
            "operation_changed",
            binding_id,
            "ui",
            json!({"operation_id": input.operation_id, "status": status}),
        );
        Ok(json!({"acknowledged": true, "operation": action}))
    }

    fn validate_regex(&self, params: serde_json::Map<String, Value>) -> Result<Value, AppError> {
        let spec: SearchSpec = from_params(params)?;
        if spec.regex {
            regex::RegexBuilder::new(&spec.query)
                .case_insensitive(!spec.case_sensitive)
                .build()
                .map_err(|error| AppError::new("invalid_regex", error.to_string()))?;
        }
        Ok(json!({"valid": true}))
    }

    fn set_search_spec(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        let spec: SearchSpec = from_params(params)?;
        self.validate_regex(to_map(&spec)?)?;
        let mut state = self.lock()?;
        self.ensure_binding(&state, binding_id)?;
        state.search_spec = spec.clone();
        self.publish_locked(
            &mut state,
            "search_changed",
            binding_id,
            "agent",
            to_value(&spec)?,
        );
        to_value(spec)
    }

    fn execute_search(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Execute {
            #[serde(default)]
            page_size: Option<usize>,
            #[serde(default)]
            spec: Option<SearchSpec>,
            #[serde(default)]
            expected_revision_id: Option<RevisionId>,
        }
        let input: Execute = from_params(params)?;
        let mut state = self.lock()?;
        self.ensure_binding(&state, binding_id)?;
        let snapshot = state.current.as_ref().expect("checked").snapshot.clone();
        if input
            .expected_revision_id
            .is_some_and(|revision| revision != snapshot.project.current_revision_id)
        {
            return Err(AppError::new(
                "stale_revision",
                "Search execute expected an earlier project revision.",
            ));
        }
        let spec = input.spec.unwrap_or_else(|| state.search_spec.clone());
        self.validate_regex(to_map(&spec)?)?;
        let changed_spec = state.search_spec.query != spec.query
            || state.search_spec.regex != spec.regex
            || state.search_spec.case_sensitive != spec.case_sensitive
            || state.search_spec.language_id != spec.language_id
            || state.search_spec.document_ids != spec.document_ids;
        state.search_spec = spec.clone();
        let response = self
            .kernel
            .search_segments(
                &snapshot,
                &SearchSegmentsRequest {
                    document_ids: spec.document_ids.clone(),
                    project_id: snapshot.project.project_id,
                    query: spec.query.clone(),
                    regex: spec.regex,
                    case_sensitive: spec.case_sensitive,
                    language_id: spec.language_id.clone(),
                    base_revision_id: snapshot.project.current_revision_id,
                },
            )
            .map_err(kernel_error)?;
        let session_id = jueming_protocol::OperationId::new().to_string();
        let revision_id = response.revision_id;
        if state.searches.len() >= MAX_SEARCH_SESSIONS {
            state.searches.clear();
            state.latest_search = None;
        }
        state.searches.insert(
            session_id.clone(),
            SearchSession {
                session_id: session_id.clone(),
                revision_id,
                response,
            },
        );
        state.latest_search = Some(session_id.clone());
        if changed_spec {
            self.publish_locked(
                &mut state,
                "search_changed",
                binding_id,
                "agent",
                json!({"spec": spec}),
            );
        }
        self.publish_locked(
            &mut state,
            "search_changed",
            binding_id,
            "agent",
            json!({"session_id": session_id}),
        );
        search_page(
            &state.searches[&session_id],
            0,
            input.page_size.unwrap_or(50),
        )
    }

    fn search_results(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Get {
            session_id: String,
            #[serde(default)]
            cursor: Option<usize>,
            #[serde(default)]
            page_size: Option<usize>,
        }
        let input: Get = from_params(params)?;
        let state = self.lock()?;
        self.ensure_binding(&state, binding_id)?;
        let current = state.current.as_ref().expect("checked");
        let session = state
            .searches
            .get(&input.session_id)
            .ok_or_else(|| AppError::new("search_not_found", "Search session is unavailable."))?;
        if session.revision_id != current.snapshot.project.current_revision_id {
            return Err(AppError::new(
                "stale_revision",
                "Search results belong to an earlier revision; execute the search again.",
            ));
        }
        search_page(
            session,
            input.cursor.unwrap_or(0),
            input.page_size.unwrap_or(50),
        )
    }

    fn get_segment(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Get {
            segment_id: String,
        }
        let input: Get = from_params(params)?;
        let id = SegmentId::from_str(&input.segment_id)
            .map_err(|_| AppError::invalid("segment_id must be a stable UUID"))?;
        let state = self.lock()?;
        self.ensure_binding(&state, binding_id)?;
        to_value(
            state
                .current
                .as_ref()
                .expect("checked")
                .snapshot
                .segments
                .iter()
                .find(|segment| segment.segment_id == id)
                .cloned()
                .ok_or_else(|| {
                    AppError::new(
                        "segment_not_found",
                        "Segment does not exist in the current project.",
                    )
                })?,
        )
    }

    fn get_alignment(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Get {
            alignment_id: String,
        }
        let input: Get = from_params(params)?;
        let id = AlignmentId::from_str(&input.alignment_id)
            .map_err(|_| AppError::invalid("alignment_id must be a stable UUID"))?;
        let state = self.lock()?;
        self.ensure_binding(&state, binding_id)?;
        to_value(
            state
                .current
                .as_ref()
                .expect("checked")
                .snapshot
                .alignments
                .iter()
                .find(|alignment| alignment.alignment_id == id)
                .cloned()
                .ok_or_else(|| {
                    AppError::new(
                        "alignment_not_found",
                        "Alignment does not exist in the current project.",
                    )
                })?,
        )
    }

    fn preview_replace(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
        request_key: &str,
        fingerprint: &str,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Input {
            query: String,
            replacement: String,
            #[serde(default)]
            regex: bool,
            #[serde(default)]
            case_sensitive: bool,
            #[serde(default)]
            language_id: Option<String>,
            #[serde(default)]
            selected_segment_ids: Vec<SegmentId>,
        }
        let input: Input = from_params(params)?;
        let mut state = self.lock()?;
        self.ensure_binding(&state, binding_id)?;
        if let Some((claimed_fingerprint, proposal_id)) =
            state.preview_request_claims.get(request_key)
        {
            if claimed_fingerprint != fingerprint {
                return Err(AppError::new(
                    "request_id_conflict",
                    "A request ID may only be retried with identical method, binding, and parameters.",
                ));
            }
            let existing = state
                .proposals
                .iter()
                .find(|proposal| proposal.proposal_id == *proposal_id)
                .cloned()
                .ok_or_else(|| {
                    AppError::new(
                        "host_inconsistent",
                        "A completed preview request has no proposal record.",
                    )
                })?;
            return to_value(existing);
        }
        let current = state.current.as_ref().expect("checked");
        let preview_request = ReplacePreviewRequest {
            document_ids: None,
            project_id: current.snapshot.project.project_id,
            query: input.query,
            replacement: input.replacement,
            regex: input.regex,
            case_sensitive: input.case_sensitive,
            language_id: input.language_id,
            base_revision_id: current.snapshot.project.current_revision_id,
        };
        let preview = self
            .kernel
            .preview_replace(&current.snapshot, &preview_request)
            .map_err(kernel_error)?;
        let proposal = ReplaceProposal {
            proposal_id: jueming_protocol::OperationId::new().to_string(),
            canonical_operation_id: jueming_protocol::OperationId::new().to_string(),
            status: ProposalStatus::Pending,
            preview,
            request: ReplaceApplyRequest {
                preview: preview_request,
                selected_segment_ids: input.selected_segment_ids,
            },
            applied_revision_id: None,
            applied_operation_id: None,
        };
        let mut next_proposals = state.proposals.clone();
        next_proposals.push(proposal.clone());
        persist_journal(state.current.as_ref().expect("checked"), &next_proposals)?;
        state.proposals = next_proposals;
        state.preview_request_claims.insert(
            request_key.into(),
            (fingerprint.into(), proposal.proposal_id.clone()),
        );
        state.preview_claim_order.push_back(request_key.into());
        while state.preview_claim_order.len() > MAX_RETRY_CACHE {
            if let Some(expired) = state.preview_claim_order.pop_front() {
                state.preview_request_claims.remove(&expired);
            }
        }
        self.publish_locked(
            &mut state,
            "proposal_changed",
            binding_id,
            "agent",
            to_value(&proposal)?,
        );
        to_value(proposal)
    }

    fn approve_replace(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Input {
            proposal_id: String,
        }
        let input: Input = from_params(params)?;
        let mut state = self.lock()?;
        self.ensure_binding(&state, binding_id)?;
        let index = state
            .proposals
            .iter()
            .position(|proposal| proposal.proposal_id == input.proposal_id)
            .ok_or_else(|| {
                AppError::new("proposal_not_found", "Replace proposal is unavailable.")
            })?;
        if state.proposals[index].status == ProposalStatus::Approved {
            return to_value(state.proposals[index].clone());
        }
        if state.proposals[index].status != ProposalStatus::Pending {
            return Err(AppError::new(
                "proposal_not_pending",
                "Only a pending proposal can be approved.",
            ));
        }
        let current = state.current.as_ref().expect("checked").clone();
        if state.proposals[index].request.preview.base_revision_id
            != current.snapshot.project.current_revision_id
        {
            return Err(AppError::new(
                "stale_revision",
                "The proposal was previewed against an earlier revision.",
            ));
        }
        let mut applying = state.proposals.clone();
        applying[index].status = ProposalStatus::Applying;
        persist_journal(&current, &applying)?;
        state.proposals = applying;
        let operation_id =
            jueming_protocol::OperationId::from_str(&state.proposals[index].canonical_operation_id)
                .map_err(|_| {
                    AppError::new(
                        "journal_invalid",
                        "Replace proposal has an invalid canonical operation ID.",
                    )
                })?;
        let next = match self.kernel.apply_replace_with_operation_id(
            &current.path,
            &current.snapshot,
            &state.proposals[index].request,
            operation_id,
        ) {
            Ok(next) => next,
            Err(error) => {
                let mut pending = state.proposals.clone();
                pending[index].status = ProposalStatus::Pending;
                persist_journal(&current, &pending)?;
                state.proposals = pending;
                return Err(kernel_error(error));
            }
        };
        state.current = Some(OpenProject {
            writer_lock: current.writer_lock,
            path: current.path,
            snapshot: next.clone(),
        });
        let mut approved = state.proposals.clone();
        approved[index].status = ProposalStatus::Approved;
        approved[index].applied_revision_id = Some(next.project.current_revision_id.to_string());
        approved[index].applied_operation_id =
            Some(state.proposals[index].canonical_operation_id.clone());
        if let Err(error) = persist_journal(state.current.as_ref().expect("updated"), &approved) {
            for binding in state.bindings.values_mut() {
                binding.revision_id = next.project.current_revision_id;
            }
            let proposal_id = state.proposals[index].proposal_id.clone();
            self.publish_locked(&mut state, "revision_advanced", binding_id, "native", json!({"project_id": next.project.project_id, "revision_id": next.project.current_revision_id}));
            self.publish_locked(
                &mut state,
                "proposal_changed",
                binding_id,
                "native",
                json!({"proposal_id": proposal_id, "status":"applying", "recovery_needed":true}),
            );
            return Err(AppError::new(
                "committed_recovery_needed",
                format!(
                    "Canonical replacement committed at revision {}; proposal journal finalization must be recovered on reopen: {}",
                    next.project.current_revision_id, error.message
                ),
            ));
        }
        state.proposals = approved;
        for binding in state.bindings.values_mut() {
            binding.revision_id = next.project.current_revision_id;
        }
        let proposal = state.proposals[index].clone();
        self.publish_locked(&mut state, "revision_advanced", binding_id, "native", json!({"project_id": next.project.project_id, "revision_id": next.project.current_revision_id}));
        self.publish_locked(
            &mut state,
            "proposal_changed",
            binding_id,
            "native",
            to_value(&proposal)?,
        );
        to_value(proposal)
    }

    fn reject_replace(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Input {
            proposal_id: String,
        }
        let input: Input = from_params(params)?;
        let mut state = self.lock()?;
        self.ensure_binding(&state, binding_id)?;
        let index = state
            .proposals
            .iter()
            .position(|proposal| proposal.proposal_id == input.proposal_id)
            .ok_or_else(|| {
                AppError::new("proposal_not_found", "Replace proposal is unavailable.")
            })?;
        if state.proposals[index].status == ProposalStatus::Pending {
            let mut rejected = state.proposals.clone();
            rejected[index].status = ProposalStatus::Rejected;
            persist_journal(state.current.as_ref().expect("checked"), &rejected)?;
            state.proposals = rejected;
        }
        let proposal = state.proposals[index].clone();
        self.publish_locked(
            &mut state,
            "proposal_changed",
            binding_id,
            "agent",
            to_value(&proposal)?,
        );
        to_value(proposal)
    }

    fn operation(
        &self,
        binding_id: Option<&str>,
        params: serde_json::Map<String, Value>,
        cancel: bool,
    ) -> Result<Value, AppError> {
        #[derive(Deserialize)]
        struct Input {
            operation_id: String,
        }
        let input: Input = from_params(params)?;
        let mut state = self.lock()?;
        let owner = state
            .operation_owners
            .get(&input.operation_id)
            .cloned()
            .ok_or_else(|| {
                AppError::new(
                    "operation_not_found",
                    "No host UI operation has this ID in the current project.",
                )
            })?;
        match owner.project_id {
            Some(project_id) => {
                self.ensure_binding(&state, binding_id)?;
                let current = state
                    .current
                    .as_ref()
                    .expect("validated binding requires project");
                if current.snapshot.project.project_id != project_id
                    || owner.binding_id.as_deref() != binding_id
                {
                    return Err(AppError::new(
                        "operation_forbidden",
                        "This operation belongs to another client binding or project.",
                    ));
                }
            }
            None if state.current.is_none() && owner.binding_id.as_deref() == binding_id => {}
            None => {
                return Err(AppError::new(
                    "operation_forbidden",
                    "This app-only operation belongs to another client binding or epoch.",
                ));
            }
        }
        let current_status = state
            .operations
            .get(&input.operation_id)
            .cloned()
            .ok_or_else(|| {
                AppError::new(
                    "operation_not_found",
                    "No operation has this ID in the current project.",
                )
            })?;
        if cancel {
            let action = state
                .pending_ui_actions
                .get(&input.operation_id)
                .cloned()
                .ok_or_else(|| {
                    AppError::new(
                        "operation_not_cancellable",
                        "Only a pending native UI operation can be cancelled here.",
                    )
                })?;
            if action.status != "accepted" {
                return Err(AppError::new(
                    "operation_not_cancellable",
                    "This UI operation is no longer pending for the current binding.",
                ));
            }
            match (action.project_id, action.revision_id) {
                (Some(project_id), Some(revision_id)) => {
                    self.ensure_binding(&state, binding_id)?;
                    let current = state
                        .current
                        .as_ref()
                        .expect("validated binding requires project");
                    if current.snapshot.project.project_id != project_id
                        || current.snapshot.project.current_revision_id != revision_id
                    {
                        return Err(AppError::new(
                            "stale_operation_epoch",
                            "The UI operation belongs to an earlier project or revision epoch.",
                        ));
                    }
                }
                (None, None) if state.current.is_none() && binding_id.is_none() => {}
                _ => {
                    return Err(AppError::new(
                        "stale_operation_epoch",
                        "The app-only UI operation no longer belongs to the current application epoch.",
                    ));
                }
            }
            state.pending_ui_actions.remove(&input.operation_id);
            state
                .operations
                .insert(input.operation_id.clone(), "cancelled".into());
            self.publish_locked(
                &mut state,
                "operation_changed",
                None,
                "agent",
                json!({"operation_id": input.operation_id, "status":"cancelled"}),
            );
        }
        Ok(
            json!({"operation_id": input.operation_id, "status": if cancel { "cancelled".to_owned() } else { current_status }}),
        )
    }

    fn publish_locked(
        &self,
        state: &mut HostState,
        kind: &str,
        binding_id: Option<&str>,
        origin: &str,
        payload: Value,
    ) {
        state.sequence += 1;
        let _ = self.events.send(AppEvent {
            contract_version: APPLICATION_CONTRACT_VERSION.into(),
            sequence: state.sequence.to_string(),
            kind: kind.into(),
            binding_id: binding_id.map(str::to_owned),
            origin: origin.into(),
            payload,
        });
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, HostState>, AppError> {
        self.state.lock().map_err(|_| {
            AppError::new(
                "host_unavailable",
                "The LocalAppHost state is unavailable; restart Jueming Aligner.",
            )
        })
    }
}

fn search_page(
    session: &SearchSession,
    cursor: usize,
    requested: usize,
) -> Result<Value, AppError> {
    let size = requested.clamp(1, MAX_SEARCH_PAGE);
    let hits = &session.response.hits;
    let end = cursor.saturating_add(size).min(hits.len());
    Ok(
        json!({"session_id": session.session_id, "revision_id": session.revision_id, "hits": &hits[cursor.min(hits.len())..end], "next_cursor": (end < hits.len()).then_some(end), "total": hits.len()}),
    )
}
fn to_value<T: Serialize>(value: T) -> Result<Value, AppError> {
    serde_json::to_value(value)
        .map_err(|error| AppError::new("serialization_failed", error.to_string()))
}
fn from_params<T: for<'de> Deserialize<'de>>(
    params: serde_json::Map<String, Value>,
) -> Result<T, AppError> {
    serde_json::from_value(Value::Object(params))
        .map_err(|error| AppError::invalid(error.to_string()))
}
fn to_map<T: Serialize>(value: &T) -> Result<serde_json::Map<String, Value>, AppError> {
    match to_value(value)? {
        Value::Object(value) => Ok(value),
        _ => Err(AppError::invalid("expected object")),
    }
}
fn scoped_method(method: &str) -> bool {
    if let Some(descriptor) = crate::research_method(method) {
        return descriptor.requires_binding;
    }
    if method == "research.call" {
        return false;
    }
    if matches!(
        method,
        "capabilities.get" | "slots.list" | "schemas.list" | "schemas.get" | "operators.list"
    ) || method.starts_with("features.")
    {
        return false;
    }
    !matches!(
        method,
        "app.describe"
            | "app.bind_session"
            | "search.validate_regex"
            | "ui.get_context"
            | "ui.get_selection"
            | "ui.publish_context"
            | "ui.navigate"
            | "ui.ack"
            | "operation.get"
            | "operation.cancel"
    )
}
fn retryable_method(method: &str) -> bool {
    if matches!(
        method,
        "research.start"
            | "research.confirm"
            | "research.merge_groups"
            | "pipeline.start"
            | "pipeline.save_method_v2"
    ) {
        return true;
    }
    matches!(
        method,
        "ui.navigate"
            | "ui.reveal"
            | "ui.ack"
            | "search.set_spec"
            | "proposal.preview_replace"
            | "proposal.approve"
            | "proposal.reject"
    )
}
fn request_fingerprint(call: &AgentCall, trusted_native: bool) -> Result<String, AppError> {
    serde_json::to_string(&json!({"trusted_native": trusted_native, "method": call.method, "binding_id": call.binding_id, "params": call.params}))
        .map_err(|error| AppError::new("serialization_failed", error.to_string()))
}
fn kernel_error(error: KernelError) -> AppError {
    let code = match error {
        KernelError::StaleRevision { .. } => "stale_revision",
        KernelError::InvalidSearchRegex(_) => "invalid_regex",
        KernelError::ProjectMismatch => "project_mismatch",
        _ => "kernel_error",
    };
    AppError::new(code, error.to_string())
}
fn journal_path(project: &Path) -> PathBuf {
    project
        .join("extensions")
        .join("jueming-application")
        .join("replace-proposals.json")
}
fn read_journal(project: &Path, project_id: ProjectId) -> Result<Vec<ReplaceProposal>, AppError> {
    let path = journal_path(project);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(&path)
        .map_err(|error| AppError::new("journal_read_failed", error.to_string()))?;
    let journal: ProposalJournal = serde_json::from_slice(&bytes)
        .map_err(|error| AppError::new("journal_invalid", error.to_string()))?;
    if journal.project_id != project_id.to_string() {
        return Err(AppError::new(
            "journal_project_mismatch",
            "The extension proposal journal belongs to another project.",
        ));
    }
    Ok(journal.proposals)
}

/// Recover an in-flight proposal only when its preallocated canonical
/// operation ID appears in history. Revision order and change-set labels are
/// deliberately not evidence of proposal ownership.
fn reconcile_applying_proposals(
    snapshot: &ProjectSnapshot,
    proposals: &mut [ReplaceProposal],
) -> bool {
    let mut changed = false;
    for proposal in proposals
        .iter_mut()
        .filter(|proposal| proposal.status == ProposalStatus::Applying)
    {
        if let Some(revision) = snapshot
            .revisions
            .iter()
            .find(|revision| revision.operation_id.to_string() == proposal.canonical_operation_id)
        {
            proposal.status = ProposalStatus::Approved;
            proposal.applied_revision_id = Some(revision.revision_id.to_string());
            proposal.applied_operation_id = Some(proposal.canonical_operation_id.clone());
            changed = true;
        } else if snapshot.project.current_revision_id == proposal.request.preview.base_revision_id
        {
            proposal.status = ProposalStatus::Pending;
            changed = true;
        }
    }
    changed
}

fn persist_journal(current: &OpenProject, proposals: &[ReplaceProposal]) -> Result<(), AppError> {
    #[cfg(test)]
    if JOURNAL_FAIL_ON_WRITE
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
            remaining.checked_sub(1)
        })
        .ok()
        == Some(1)
    {
        return Err(AppError::new(
            "journal_write_failed",
            "test failpoint: journal write interrupted",
        ));
    }
    let bytes = serde_json::to_vec_pretty(&ProposalJournal {
        project_id: current.snapshot.project.project_id.to_string(),
        proposals: proposals.to_vec(),
    })
    .map_err(|error| AppError::new("journal_serialize_failed", error.to_string()))?;
    write_bytes_atomic(&journal_path(&current.path), &bytes)
        .map_err(|error| AppError::new("journal_write_failed", error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jueming_protocol::{
        CreateProjectRequest, Encoding, ImportProfile, ImportSideRequest, SegmentationMode,
        TextInput,
    };
    use serde_json::Map;

    fn request(path: &Path) -> CreateProjectRequest {
        let profile = ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine);
        CreateProjectRequest {
            additional_targets: Vec::new(),
            project_path: path.to_string_lossy().into_owned(),
            name: "recovery".into(),
            source: ImportSideRequest {
                expected_sha256: None,
                language_id: "zh-CN".into(),
                title: "source".into(),
                input: TextInput::Paste {
                    label: "source".into(),
                    text: "old".into(),
                },
                profile: profile.clone(),
            },
            target: ImportSideRequest {
                expected_sha256: None,
                language_id: "en".into(),
                title: "target".into(),
                input: TextInput::Paste {
                    label: "target".into(),
                    text: "old".into(),
                },
                profile,
            },
        }
    }

    fn call(
        request_id: &str,
        method: &str,
        params: Value,
        binding_id: Option<String>,
    ) -> AgentCall {
        AgentCall {
            request_id: request_id.into(),
            method: method.into(),
            params: params.as_object().cloned().unwrap_or_else(Map::new),
            binding_id,
        }
    }

    #[test]
    fn journal_finalization_failure_keeps_committed_revision_and_recovers_by_operation_id() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("recovery.jm");
        let host = LocalAppHost::new();
        host.create_project(&request(&path)).unwrap();
        let binding = host
            .dispatch_native(call("bind", "app.bind_session", json!({}), None))
            .unwrap()
            .data["binding_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let proposal = host
            .dispatch_native(call(
                "preview",
                "proposal.preview_replace",
                json!({"query":"old", "replacement":"new"}),
                Some(binding.clone()),
            ))
            .unwrap();
        let proposal_id = proposal.data["proposal_id"].as_str().unwrap().to_owned();
        JOURNAL_FAIL_ON_WRITE.store(2, Ordering::SeqCst);
        let error = host
            .dispatch_native(call(
                "approve",
                "proposal.approve",
                json!({"proposal_id":proposal_id}),
                Some(binding),
            ))
            .unwrap_err();
        assert_eq!(error.code, "committed_recovery_needed");
        assert_eq!(
            host.current_snapshot().unwrap().project.current_revision_id,
            RevisionId::new(2)
        );
        host.open_project(&path).unwrap();
        let projection = host.projection().unwrap();
        assert_eq!(projection.data["proposals"][0]["status"], "approved");
        assert_eq!(projection.data["proposals"][0]["applied_revision_id"], "2");
        JOURNAL_FAIL_ON_WRITE.store(usize::MAX, Ordering::SeqCst);
    }
}
