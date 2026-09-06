//! Stdio MCP sidecar for an already-running local Jueming application.
//!
//! The sidecar has no project loader and no direct storage dependency. It forwards only its
//! explicit allowlist to the authenticated local application bridge.

use std::{
    collections::HashMap,
    net::IpAddr,
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};

use futures_util::StreamExt;
use jueming_application::{AgentCall, AgentReply};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, ContentBlock, ListResourceTemplatesResult, ListResourcesResult,
        ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult, Resource,
        ResourceContents, ResourceTemplate, ServerCapabilities, ServerInfo,
    },
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use thiserror::Error;
use url::Url;
use uuid::Uuid;

const CALL_PATH: &str = "v1/agent/call";
const MAX_RESPONSE_BYTES: usize = 2 * 1_024 * 1_024;

/// Sidecar configuration. The token is sourced from the sidecar environment, never arguments.
#[derive(Clone)]
pub struct SidecarConfig {
    endpoint: Url,
    token: String,
    request_timeout: Duration,
}

impl SidecarConfig {
    /// Builds a config after verifying that the bridge is a local HTTP endpoint.
    pub fn new(endpoint: impl AsRef<str>, token: impl Into<String>) -> Result<Self, BridgeError> {
        let endpoint = Url::parse(endpoint.as_ref())
            .map_err(|_| BridgeError::InvalidConfig("endpoint must be an absolute URL".into()))?;
        if endpoint.scheme() != "http" || !is_loopback_url(&endpoint) {
            return Err(BridgeError::InvalidConfig(
                "the MCP sidecar accepts only an http loopback bridge endpoint".into(),
            ));
        }
        let token = token.into();
        if token.trim().is_empty() {
            return Err(BridgeError::InvalidConfig(
                "JUEMING_AGENT_TOKEN must not be empty".into(),
            ));
        }
        Ok(Self {
            endpoint,
            token,
            request_timeout: Duration::from_secs(10),
        })
    }

    /// Reads the two values passed by the Desktop sidecar launcher.
    pub fn from_environment() -> Result<Self, BridgeError> {
        let endpoint = std::env::var("JUEMING_AGENT_ENDPOINT")
            .map_err(|_| BridgeError::InvalidConfig("JUEMING_AGENT_ENDPOINT is required".into()))?;
        let token = std::env::var("JUEMING_AGENT_TOKEN")
            .map_err(|_| BridgeError::InvalidConfig("JUEMING_AGENT_TOKEN is required".into()))?;
        Self::new(endpoint, token)
    }
}

fn is_loopback_url(url: &Url) -> bool {
    match url.host_str() {
        Some("localhost") => true,
        Some(host) => IpAddr::from_str(host).is_ok_and(|address| address.is_loopback()),
        None => false,
    }
}

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("invalid bridge configuration: {0}")]
    InvalidConfig(String),
    #[error("local bridge request failed")]
    Request(#[source] reqwest::Error),
    #[error("local bridge response exceeded its size limit")]
    ResponseTooLarge,
    #[error("local bridge returned invalid JSON")]
    InvalidResponse(#[source] serde_json::Error),
    #[error("{code}: {message}")]
    Rejected { code: String, message: String },
}

#[derive(Clone)]
struct HttpBridge {
    client: reqwest::Client,
    endpoint: Url,
    token: Arc<str>,
}

impl HttpBridge {
    fn new(config: SidecarConfig) -> Result<Self, BridgeError> {
        let client = reqwest::Client::builder()
            .timeout(config.request_timeout)
            .build()
            .map_err(BridgeError::Request)?;
        Ok(Self {
            client,
            endpoint: config.endpoint,
            token: Arc::from(config.token),
        })
    }

    async fn call(
        &self,
        method: impl Into<String>,
        params: Map<String, Value>,
        binding_id: Option<String>,
    ) -> Result<AgentReply, BridgeError> {
        self.call_with_request_id(Uuid::now_v7().to_string(), method, params, binding_id)
            .await
    }

    async fn call_with_request_id(
        &self,
        request_id: String,
        method: impl Into<String>,
        params: Map<String, Value>,
        binding_id: Option<String>,
    ) -> Result<AgentReply, BridgeError> {
        let url = self
            .endpoint
            .join(CALL_PATH)
            .map_err(|_| BridgeError::InvalidConfig("invalid bridge endpoint path".into()))?;
        let call = AgentCall {
            request_id,
            method: method.into(),
            params,
            binding_id,
        };
        let response = self
            .client
            .post(url)
            .bearer_auth(self.token.as_ref())
            .json(&call)
            .send()
            .await
            .map_err(BridgeError::Request)?;
        let status = response.status();
        let bytes = limited_body(response).await?;
        if !status.is_success() {
            let error: BridgeWireError =
                serde_json::from_slice(&bytes).map_err(BridgeError::InvalidResponse)?;
            return Err(BridgeError::Rejected {
                code: error.code,
                message: error.message,
            });
        }
        serde_json::from_slice(&bytes).map_err(BridgeError::InvalidResponse)
    }

    async fn checked_call(
        &self,
        method: impl Into<String>,
        params: Map<String, Value>,
        binding_id: String,
    ) -> Result<AgentReply, BridgeError> {
        // project.get_summary does not itself consume a binding in the host today. This explicit
        // validation prevents an MCP caller from using an old binding to read a new project.
        self.call("ui.get_context", Map::new(), Some(binding_id.clone()))
            .await?;
        self.call(method, params, Some(binding_id)).await
    }

    async fn checked_call_with_request_id(
        &self,
        request_id: String,
        method: impl Into<String>,
        params: Map<String, Value>,
        binding_id: String,
    ) -> Result<AgentReply, BridgeError> {
        self.call("ui.get_context", Map::new(), Some(binding_id.clone()))
            .await?;
        self.call_with_request_id(request_id, method, params, Some(binding_id))
            .await
    }
}

async fn limited_body(response: reqwest::Response) -> Result<Vec<u8>, BridgeError> {
    if response
        .content_length()
        .is_some_and(|size| size > MAX_RESPONSE_BYTES as u64)
    {
        return Err(BridgeError::ResponseTooLarge);
    }
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(BridgeError::Request)?;
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(BridgeError::ResponseTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeWireError {
    code: String,
    message: String,
}

#[derive(Clone)]
struct BindingInfo {
    project_id: String,
    revision_id: String,
}

/// The official rmcp server. Its only state is the explicit binding metadata needed to validate
/// resource URIs; canonical project state remains in `LocalAppHost`.
pub struct McpServer {
    bridge: HttpBridge,
    bindings: Arc<RwLock<HashMap<String, BindingInfo>>>,
    tool_router: ToolRouter<Self>,
}

impl McpServer {
    pub fn new(config: SidecarConfig) -> Result<Self, BridgeError> {
        Ok(Self {
            bridge: HttpBridge::new(config)?,
            bindings: Arc::new(RwLock::new(HashMap::new())),
            tool_router: Self::tool_router(),
        })
    }

    async fn direct(&self, method: &str) -> CallToolResult {
        into_tool_result(self.bridge.call(method, Map::new(), None).await)
    }

    async fn bound(
        &self,
        method: &str,
        params: Map<String, Value>,
        binding_id: String,
    ) -> CallToolResult {
        into_tool_result(self.bridge.checked_call(method, params, binding_id).await)
    }

    async fn bound_with_request_id(
        &self,
        request_id: String,
        method: &str,
        params: Map<String, Value>,
        binding_id: String,
    ) -> CallToolResult {
        into_tool_result(
            self.bridge
                .checked_call_with_request_id(request_id, method, params, binding_id)
                .await,
        )
    }

    async fn bind(&self) -> CallToolResult {
        let result = self.bridge.call("app.bind_session", Map::new(), None).await;
        if let Ok(reply) = &result
            && let Some(binding) = binding_from_reply(reply)
        {
            self.bindings
                .write()
                .expect("MCP binding map lock poisoned")
                .insert(binding.0, binding.1);
        }
        into_tool_result(result)
    }

    fn discovery_metadata(&self) -> CallToolResult {
        let tools = self.tool_router.list_all();
        let mut result = CallToolResult::success(vec![ContentBlock::text(
            "Jueming MCP tool metadata; every listed tool is currently bound by this sidecar.",
        )]);
        result.structured_content = Some(serde_json::json!({
            "transport": "stdio",
            "tools": tools,
            "executable_plugin_loading": {
                "available": false,
                "reason": "No shared ToolProvider runtime contract is available."
            },
            "native_only_methods": [
                "app.get_projection",
                "ui.publish_context",
                "ui.ack",
                "proposal.approve",
                "pipeline.create_default",
                "pipeline.update",
                "pipeline.approve_update"
            ]
        }));
        result
    }
}

/// Performs a non-mutating bridge smoke check without starting the stdio protocol server.
/// This is intended for packaging and local launcher diagnostics; it calls only `app.describe`.
pub async fn smoke(config: SidecarConfig) -> Result<(), BridgeError> {
    HttpBridge::new(config)?
        .call("app.describe", Map::new(), None)
        .await
        .map(|_| ())
}

#[tool_router]
impl McpServer {
    /// Describes the external-safe application contract and native-only approval boundary.
    #[tool(
        name = "jueming_describe",
        description = "Describe Jueming's safe external MCP contract."
    )]
    async fn describe(&self) -> CallToolResult {
        self.direct("app.describe").await
    }

    /// Reads non-secret application capability metadata from the bound host.
    #[tool(
        name = "jueming_application_settings_metadata",
        description = "Read non-secret application capability metadata. Credentials and settings values are never exposed."
    )]
    async fn application_settings_metadata(&self) -> CallToolResult {
        self.direct("app.describe").await
    }

    /// Lists only tools implemented by this sidecar; plugin loading is metadata-only and unavailable.
    #[tool(
        name = "jueming_tool_discovery",
        description = "List currently available Jueming MCP tools and capability boundaries."
    )]
    async fn tool_discovery(&self) -> CallToolResult {
        self.discovery_metadata()
    }

    /// Creates a binding against the currently open project and revision.
    #[tool(
        name = "jueming_bind_session",
        description = "Bind this MCP client to the current project revision."
    )]
    async fn bind_session(&self) -> CallToolResult {
        self.bind().await
    }

    /// Reads a sanitized UI context after checking the binding.
    #[tool(
        name = "jueming_get_context",
        description = "Read the current sanitized Jueming context for one binding."
    )]
    async fn get_context(&self, Parameters(input): Parameters<BindingRequest>) -> CallToolResult {
        self.bound("ui.get_context", Map::new(), input.binding_id)
            .await
    }

    /// Requests native UI navigation. The Desktop owns draft gating and visual guidance.
    #[tool(
        name = "jueming_navigate",
        description = "Request navigation to a Jueming tab; native UI applies draft safeguards."
    )]
    async fn navigate(&self, Parameters(input): Parameters<NavigateRequest>) -> CallToolResult {
        let request_id = Uuid::now_v7().to_string();
        self.bound_with_request_id(
            request_id.clone(),
            "ui.navigate",
            params(&NavigationPayload {
                tab: input.tab,
                request_id,
            }),
            input.binding_id,
        )
        .await
    }

    /// Requests native UI reveal. The Desktop chooses the final focus and butterfly guidance.
    #[tool(
        name = "jueming_reveal",
        description = "Request reveal of a stable segment, alignment, or UI node."
    )]
    async fn reveal(&self, Parameters(input): Parameters<RevealRequest>) -> CallToolResult {
        let request_id = Uuid::now_v7().to_string();
        self.bound_with_request_id(
            request_id.clone(),
            "ui.reveal",
            params(&RevealPayload {
                segment_id: input.segment_id,
                alignment_id: input.alignment_id,
                node_id: input.node_id,
                request_id,
            }),
            input.binding_id,
        )
        .await
    }

    /// Reads the status of an operation returned by navigation, reveal, or another bound request.
    #[tool(
        name = "jueming_get_operation",
        description = "Read one bound operation status by its stable operation ID."
    )]
    async fn get_operation(
        &self,
        Parameters(input): Parameters<OperationRequest>,
    ) -> CallToolResult {
        self.bound(
            "operation.get",
            params_without_binding(&input),
            input.binding_id.clone(),
        )
        .await
    }

    /// Cancels a pending navigation or reveal operation before the native UI applies it.
    #[tool(
        name = "jueming_cancel_operation",
        description = "Cancel one pending native UI operation by its stable operation ID."
    )]
    async fn cancel_operation(
        &self,
        Parameters(input): Parameters<OperationRequest>,
    ) -> CallToolResult {
        self.bound(
            "operation.cancel",
            params_without_binding(&input),
            input.binding_id.clone(),
        )
        .await
    }

    /// Reads the current project summary after checking the binding.
    #[tool(
        name = "jueming_project_summary",
        description = "Read the current project's summary for one binding."
    )]
    async fn project_summary(
        &self,
        Parameters(input): Parameters<BindingRequest>,
    ) -> CallToolResult {
        self.bound("project.get_summary", Map::new(), input.binding_id)
            .await
    }

    /// Reads one stable segment ID.
    #[tool(
        name = "jueming_get_segment",
        description = "Read one segment by its stable UUID."
    )]
    async fn get_segment(&self, Parameters(input): Parameters<SegmentRequest>) -> CallToolResult {
        self.bound("segment.get", params(&input), input.binding_id.clone())
            .await
    }

    /// Reads one stable alignment ID.
    #[tool(
        name = "jueming_get_alignment",
        description = "Read one alignment by its stable UUID."
    )]
    async fn get_alignment(
        &self,
        Parameters(input): Parameters<AlignmentRequest>,
    ) -> CallToolResult {
        self.bound("alignment.get", params(&input), input.binding_id.clone())
            .await
    }

    /// Atomically applies and executes one bounded search specification.
    #[tool(
        name = "jueming_search",
        description = "Execute a bounded search against the currently bound revision."
    )]
    async fn search(&self, Parameters(input): Parameters<SearchRequest>) -> CallToolResult {
        let binding_id = input.binding_id.clone();
        self.bound(
            "search.execute",
            params(&AtomicSearchRequest {
                spec: SearchSpecRequest::from(&input),
                page_size: input.page_size,
                expected_revision_id: input.expected_revision_id,
            }),
            binding_id,
        )
        .await
    }

    /// Reads a page from a prior search session.
    #[tool(
        name = "jueming_search_results",
        description = "Read a page from a prior Jueming search session."
    )]
    async fn search_results(
        &self,
        Parameters(input): Parameters<SearchResultsRequest>,
    ) -> CallToolResult {
        self.bound(
            "search.get_results",
            params_without_binding(&input),
            input.binding_id.clone(),
        )
        .await
    }

    /// Stages a replace proposal for native review; it cannot approve canonical changes.
    #[tool(
        name = "jueming_preview_replace",
        description = "Stage a replace proposal for native review without applying it."
    )]
    async fn preview_replace(
        &self,
        Parameters(input): Parameters<PreviewReplaceRequest>,
    ) -> CallToolResult {
        self.bound(
            "proposal.preview_replace",
            params(&input),
            input.binding_id.clone(),
        )
        .await
    }

    /// Lists staged proposals.
    #[tool(
        name = "jueming_list_proposals",
        description = "List staged, reviewable proposals for one binding."
    )]
    async fn list_proposals(
        &self,
        Parameters(input): Parameters<BindingRequest>,
    ) -> CallToolResult {
        self.bound("proposal.list", Map::new(), input.binding_id)
            .await
    }

    /// Rejects a staged proposal without applying canonical changes.
    #[tool(
        name = "jueming_reject_proposal",
        description = "Reject a pending proposal; native UI remains the only approval path."
    )]
    async fn reject_proposal(
        &self,
        Parameters(input): Parameters<ProposalIdRequest>,
    ) -> CallToolResult {
        self.bound("proposal.reject", params(&input), input.binding_id.clone())
            .await
    }

    /// Lists Pipeline method summaries for the explicit bound project.
    #[tool(
        name = "jueming_pipeline_list",
        description = "List derived Pipeline methods for the current bound project."
    )]
    async fn pipeline_list(&self, Parameters(input): Parameters<BindingRequest>) -> CallToolResult {
        self.bound("pipeline.list", Map::new(), input.binding_id)
            .await
    }

    /// Reads a Pipeline method using its stable method ID.
    #[tool(
        name = "jueming_pipeline_get",
        description = "Read one Pipeline method by its stable method ID."
    )]
    async fn pipeline_get(
        &self,
        Parameters(input): Parameters<PipelineMethodRequest>,
    ) -> CallToolResult {
        self.bound(
            "pipeline.get",
            params_without_binding(&input),
            input.binding_id.clone(),
        )
        .await
    }

    /// Stages an immutable Pipeline method update proposal for native review.
    #[tool(
        name = "jueming_pipeline_preview_update",
        description = "Stage a Pipeline method update proposal; native Jueming UI must approve it."
    )]
    async fn pipeline_preview_update(
        &self,
        Parameters(input): Parameters<PipelinePreviewUpdateRequest>,
    ) -> CallToolResult {
        self.bound(
            "pipeline.preview_update",
            params(&PipelineUpdatePayload {
                request: input.request,
            }),
            input.binding_id,
        )
        .await
    }

    /// Lists immutable Pipeline method proposals awaiting native review.
    #[tool(
        name = "jueming_pipeline_list_proposals",
        description = "List Pipeline method proposals for the current bound project."
    )]
    async fn pipeline_list_proposals(
        &self,
        Parameters(input): Parameters<BindingRequest>,
    ) -> CallToolResult {
        self.bound("pipeline.list_proposals", Map::new(), input.binding_id)
            .await
    }

    /// Rejects one pending Pipeline method proposal without changing canonical content.
    #[tool(
        name = "jueming_pipeline_reject_update",
        description = "Reject a pending Pipeline method proposal by its stable proposal ID."
    )]
    async fn pipeline_reject_update(
        &self,
        Parameters(input): Parameters<PipelineProposalRequest>,
    ) -> CallToolResult {
        self.bound(
            "pipeline.reject_update",
            params_without_binding(&input),
            input.binding_id.clone(),
        )
        .await
    }

    /// Executes a derived Pipeline method against one stable segment and base canonical revision.
    #[tool(
        name = "jueming_pipeline_execute",
        description = "Execute a Pipeline method for one stable segment at an explicit canonical revision."
    )]
    async fn pipeline_execute(
        &self,
        Parameters(input): Parameters<PipelineExecuteRequest>,
    ) -> CallToolResult {
        self.bound(
            "pipeline.execute",
            params_without_binding(&input),
            input.binding_id.clone(),
        )
        .await
    }

    /// Reads a Pipeline artifact by stable artifact ID or lists artifacts for one canonical revision.
    #[tool(
        name = "jueming_pipeline_artifacts",
        description = "Read one Pipeline artifact or list artifact summaries for one input canonical revision."
    )]
    async fn pipeline_artifacts(
        &self,
        Parameters(input): Parameters<PipelineArtifactsRequest>,
    ) -> CallToolResult {
        self.bound(
            "pipeline.artifacts",
            params_without_binding(&input),
            input.binding_id.clone(),
        )
        .await
    }

    /// Cancels an in-flight Pipeline operation identified by its stable operation ID.
    #[tool(
        name = "jueming_pipeline_cancel",
        description = "Cancel an in-flight Pipeline operation by its stable operation ID."
    )]
    async fn pipeline_cancel(
        &self,
        Parameters(input): Parameters<PipelineCancelRequest>,
    ) -> CallToolResult {
        self.bound(
            "pipeline.cancel",
            params_without_binding(&input),
            input.binding_id.clone(),
        )
        .await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_instructions(
            "Use jueming_bind_session before every targeted read. This server cannot approve proposals or publish trusted UI state.",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult::with_all_items(vec![
            Resource::new("jueming://service/guide", "jueming-mcp-guide")
                .with_description("How to bind and read targeted Jueming resources.")
                .with_mime_type("text/markdown"),
        ]))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult::with_all_items(vec![
            ResourceTemplate::new(
                "jueming://project/{project_id}/summary{?binding_id}",
                "project-summary",
            )
            .with_description("Current project summary, guarded by an explicit binding."),
            ResourceTemplate::new(
                "jueming://project/{project_id}/revision/{revision_id}/segment/{segment_id}{?binding_id}",
                "segment",
            )
            .with_description("One stable segment at the binding's current revision."),
            ResourceTemplate::new(
                "jueming://project/{project_id}/revision/{revision_id}/alignment/{alignment_id}{?binding_id}",
                "alignment",
            )
            .with_description("One stable alignment at the binding's current revision."),
        ]))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ReadResourceResponse, McpError> {
        if request.uri == "jueming://service/guide" {
            return Ok(ReadResourceResult::new(vec![
                ResourceContents::text(GUIDE, request.uri).with_mime_type("text/markdown"),
            ])
            .into());
        }
        let resource = BoundResource::parse(&request.uri)
            .map_err(|message| McpError::invalid_params(message, None))?;
        let binding = self
            .bindings
            .read()
            .expect("MCP binding map lock poisoned")
            .get(&resource.binding_id)
            .cloned()
            .ok_or_else(|| {
                McpError::invalid_params(
                    "unknown binding for this sidecar; call jueming_bind_session again",
                    None,
                )
            })?;
        if binding.project_id != resource.project_id
            || resource
                .revision_id
                .as_deref()
                .is_some_and(|revision| revision != binding.revision_id)
        {
            return Err(McpError::invalid_params(
                "resource project or revision does not match the explicit binding",
                None,
            ));
        }
        let (method, params) = match resource.kind {
            ResourceKind::Summary => ("project.get_summary", Map::new()),
            ResourceKind::Segment(segment_id) => (
                "segment.get",
                json!({"segment_id": segment_id})
                    .as_object()
                    .cloned()
                    .expect("object"),
            ),
            ResourceKind::Alignment(alignment_id) => (
                "alignment.get",
                json!({"alignment_id": alignment_id})
                    .as_object()
                    .cloned()
                    .expect("object"),
            ),
        };
        let reply = self
            .bridge
            .checked_call(method, params, resource.binding_id)
            .await
            .map_err(bridge_to_mcp_error)?;
        let text = serde_json::to_string(&reply)
            .map_err(|_| McpError::internal_error("cannot encode resource result", None))?;
        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(text, request.uri).with_mime_type("application/json"),
        ])
        .into())
    }
}

fn into_tool_result(result: Result<AgentReply, BridgeError>) -> CallToolResult {
    match result {
        Ok(reply) => {
            let mut result =
                CallToolResult::success(vec![ContentBlock::text("Jueming request completed.")]);
            result.structured_content = serde_json::to_value(reply).ok();
            result
        }
        Err(error) => CallToolResult::error(vec![ContentBlock::text(error.to_string())]),
    }
}

fn bridge_to_mcp_error(error: BridgeError) -> McpError {
    McpError::invalid_params(error.to_string(), None)
}

fn binding_from_reply(reply: &AgentReply) -> Option<(String, BindingInfo)> {
    Some((
        reply.data.get("binding_id")?.as_str()?.to_owned(),
        BindingInfo {
            project_id: reply.data.get("project_id")?.as_str()?.to_owned(),
            revision_id: reply.data.get("revision_id")?.as_str()?.to_owned(),
        },
    ))
}

fn params<T: Serialize>(value: &T) -> Map<String, Value> {
    serde_json::to_value(value)
        .expect("MCP tool parameters must serialize")
        .as_object()
        .cloned()
        .expect("MCP tool parameters must be an object")
}

fn params_without_binding<T: Serialize>(value: &T) -> Map<String, Value> {
    let mut result = params(value);
    result.remove("binding_id");
    result
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BindingRequest {
    /// Stable binding returned from jueming_bind_session.
    binding_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NavigateRequest {
    /// Stable binding returned from jueming_bind_session.
    binding_id: String,
    /// Desktop tab identifier to request.
    tab: String,
}

#[derive(Debug, Serialize)]
struct NavigationPayload {
    tab: String,
    request_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RevealRequest {
    binding_id: String,
    #[serde(default)]
    segment_id: Option<String>,
    #[serde(default)]
    alignment_id: Option<String>,
    #[serde(default)]
    node_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct RevealPayload {
    segment_id: Option<String>,
    alignment_id: Option<String>,
    node_id: Option<String>,
    request_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct OperationRequest {
    /// Stable binding returned from jueming_bind_session.
    binding_id: String,
    /// Stable operation ID returned by navigation, reveal, or a bound host request.
    operation_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SegmentRequest {
    binding_id: String,
    segment_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AlignmentRequest {
    binding_id: String,
    alignment_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SearchRequest {
    binding_id: String,
    query: String,
    #[serde(default)]
    regex: bool,
    #[serde(default)]
    case_sensitive: bool,
    #[serde(default)]
    language_id: Option<String>,
    #[serde(default)]
    page_size: Option<usize>,
    /// Optional decimal canonical revision expected by this atomic search request.
    #[serde(default)]
    expected_revision_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct SearchSpecRequest {
    query: String,
    regex: bool,
    case_sensitive: bool,
    language_id: Option<String>,
}

impl From<&SearchRequest> for SearchSpecRequest {
    fn from(value: &SearchRequest) -> Self {
        Self {
            query: value.query.clone(),
            regex: value.regex,
            case_sensitive: value.case_sensitive,
            language_id: value.language_id.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
struct AtomicSearchRequest {
    spec: SearchSpecRequest,
    page_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_revision_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SearchResultsRequest {
    binding_id: String,
    session_id: String,
    #[serde(default)]
    cursor: Option<usize>,
    #[serde(default)]
    page_size: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PreviewReplaceRequest {
    binding_id: String,
    query: String,
    replacement: String,
    #[serde(default)]
    regex: bool,
    #[serde(default)]
    case_sensitive: bool,
    #[serde(default)]
    language_id: Option<String>,
    #[serde(default)]
    selected_segment_ids: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ProposalIdRequest {
    binding_id: String,
    proposal_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PipelineMethodRequest {
    /// Stable binding returned from jueming_bind_session.
    binding_id: String,
    /// Stable Pipeline method UUID.
    method_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PipelinePreviewUpdateRequest {
    /// Stable binding returned from jueming_bind_session.
    binding_id: String,
    /// Complete update checked against base_method_revision_id by the host.
    request: PipelineUpdateRequest,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PipelineUpdatePayload {
    request: PipelineUpdateRequest,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PipelineUpdateRequest {
    /// Stable Pipeline method UUID.
    method_id: String,
    /// Stable current Pipeline method revision UUID used for conflict detection.
    base_method_revision_id: String,
    /// Replacement display name; null retains the method name.
    name: Option<String>,
    /// Complete validated Pipeline graph revision.
    plan: PipelinePlanRequest,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PipelinePlanRequest {
    /// Stable Pipeline method UUID.
    method_id: String,
    /// Stable Pipeline method revision UUID represented by this plan.
    method_revision_id: String,
    /// Nodes in the typed, derived-data-only graph.
    nodes: Vec<PipelineNodeRequest>,
    /// Stable output node UUID.
    output_node_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PipelineNodeRequest {
    /// Stable Pipeline node UUID.
    node_id: String,
    /// One of the currently implemented deterministic operators.
    operator: PipelineOperator,
    /// Stable input node UUIDs.
    #[serde(default)]
    inputs: Vec<String>,
    /// Operator-specific non-secret configuration.
    #[serde(default)]
    config: Map<String, Value>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum PipelineOperator {
    Source,
    Normalize,
    ChineseTokenize,
    Artifact,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PipelineProposalRequest {
    /// Stable binding returned from jueming_bind_session.
    binding_id: String,
    /// Stable immutable Pipeline proposal ID.
    proposal_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PipelineExecuteRequest {
    /// Stable binding returned from jueming_bind_session.
    binding_id: String,
    /// Stable Pipeline method UUID.
    method_id: String,
    /// Optional stable historical Pipeline method revision UUID.
    #[serde(default)]
    method_revision_id: Option<String>,
    /// Stable Segment UUID. The host obtains its source text itself.
    segment_id: String,
    /// Decimal canonical revision ID used for stale-revision protection.
    base_revision_id: String,
    /// Optional stable operation ID for concurrent cancellation.
    #[serde(default)]
    operation_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PipelineArtifactsRequest {
    /// Stable binding returned from jueming_bind_session.
    binding_id: String,
    /// Stable derived artifact UUID. Supply exactly one selector.
    #[serde(default)]
    artifact_id: Option<String>,
    /// Decimal canonical revision ID. Supply exactly one selector.
    #[serde(default)]
    input_revision_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PipelineCancelRequest {
    /// Stable binding returned from jueming_bind_session.
    binding_id: String,
    /// Stable in-flight operation ID.
    operation_id: String,
}

#[derive(Debug)]
struct BoundResource {
    binding_id: String,
    project_id: String,
    revision_id: Option<String>,
    kind: ResourceKind,
}

#[derive(Debug)]
enum ResourceKind {
    Summary,
    Segment(String),
    Alignment(String),
}

impl BoundResource {
    fn parse(raw: &str) -> Result<Self, String> {
        let url = Url::parse(raw).map_err(|_| "resource URI is invalid".to_string())?;
        if url.scheme() != "jueming" || url.host_str() != Some("project") {
            return Err("unsupported Jueming resource URI".into());
        }
        let binding_id = url
            .query_pairs()
            .find_map(|(name, value)| (name == "binding_id").then(|| value.into_owned()))
            .ok_or_else(|| "resource URI requires binding_id".to_string())?;
        let segments: Vec<_> = url
            .path_segments()
            .ok_or_else(|| "resource URI has no path".to_string())?
            .filter(|part| !part.is_empty())
            .collect();
        match segments.as_slice() {
            [project_id, "summary"] => Ok(Self {
                binding_id,
                project_id: (*project_id).to_owned(),
                revision_id: None,
                kind: ResourceKind::Summary,
            }),
            [project_id, "revision", revision_id, "segment", segment_id] => Ok(Self {
                binding_id,
                project_id: (*project_id).to_owned(),
                revision_id: Some((*revision_id).to_owned()),
                kind: ResourceKind::Segment((*segment_id).to_owned()),
            }),
            [
                project_id,
                "revision",
                revision_id,
                "alignment",
                alignment_id,
            ] => Ok(Self {
                binding_id,
                project_id: (*project_id).to_owned(),
                revision_id: Some((*revision_id).to_owned()),
                kind: ResourceKind::Alignment((*alignment_id).to_owned()),
            }),
            _ => Err("unsupported Jueming resource URI shape".into()),
        }
    }
}

const GUIDE: &str = "# Jueming MCP\n\nCall `jueming_bind_session` first. Use its binding_id on every tool and as `binding_id` in a resource URI query. Bindings are invalidated when the project or revision changes. This server exposes targeted reads and reviewable proposal staging only; native Jueming UI alone approves canonical changes.";

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, extract::State, routing::post};
    use rmcp::ServiceExt;
    use rmcp::model::CallToolRequestParams;
    use tokio::sync::Mutex as AsyncMutex;

    #[test]
    fn refuses_non_loopback_bridge() {
        assert!(SidecarConfig::new("http://example.invalid:3000/", "token").is_err());
    }

    #[test]
    fn parses_revision_bound_segment_resource() {
        let resource = BoundResource::parse(
            "jueming://project/project-1/revision/revision-2/segment/segment-3?binding_id=binding-4",
        )
        .expect("resource URI");
        assert_eq!(resource.binding_id, "binding-4");
        assert_eq!(resource.project_id, "project-1");
        assert_eq!(resource.revision_id.as_deref(), Some("revision-2"));
        assert!(matches!(resource.kind, ResourceKind::Segment(ref id) if id == "segment-3"));
    }

    #[test]
    fn refuses_resource_without_explicit_binding() {
        assert!(BoundResource::parse("jueming://project/project-1/summary").is_err());
    }

    #[tokio::test]
    async fn rmcp_protocol_advertises_exact_external_pipeline_surface() {
        let server = McpServer::new(
            SidecarConfig::new("http://127.0.0.1:32123/", "protocol-test-token")
                .expect("loopback sidecar config"),
        )
        .expect("MCP server");
        let (server_transport, client_transport) = tokio::io::duplex(16 * 1024);
        let server_task = tokio::spawn(async move {
            let running = server
                .serve(server_transport)
                .await
                .expect("server initialization");
            let _ = running.waiting().await;
        });

        let client = ().serve(client_transport).await.expect("MCP initialize");
        let tools = client.list_all_tools().await.expect("tools/list response");
        let names: std::collections::BTreeSet<_> =
            tools.iter().map(|tool| tool.name.as_ref()).collect();
        for name in [
            "jueming_pipeline_list",
            "jueming_pipeline_get",
            "jueming_pipeline_preview_update",
            "jueming_pipeline_list_proposals",
            "jueming_pipeline_reject_update",
            "jueming_pipeline_execute",
            "jueming_pipeline_artifacts",
            "jueming_pipeline_cancel",
            "jueming_get_operation",
            "jueming_cancel_operation",
            "jueming_application_settings_metadata",
            "jueming_tool_discovery",
        ] {
            assert!(names.contains(name), "missing advertised tool {name}");
        }
        for forbidden in [
            "jueming_pipeline_create_default",
            "jueming_pipeline_update",
            "jueming_pipeline_approve_update",
            "jueming_get_projection",
            "jueming_approve_proposal",
        ] {
            assert!(
                !names.contains(forbidden),
                "native-only method was advertised as {forbidden}"
            );
        }

        let native_only = client
            .call_tool(CallToolRequestParams::new("jueming_get_projection"))
            .await;
        assert!(
            native_only.is_err(),
            "a native-only projection must not be callable through MCP"
        );

        let execute = tools
            .iter()
            .find(|tool| tool.name == "jueming_pipeline_execute")
            .expect("pipeline execute tool schema");
        let properties = execute
            .input_schema
            .get("properties")
            .and_then(Value::as_object)
            .expect("structured execute parameter schema");
        for field in ["binding_id", "method_id", "segment_id", "base_revision_id"] {
            assert!(
                properties.contains_key(field),
                "missing schema field {field}"
            );
        }

        drop(client);
        server_task.abort();
    }

    #[tokio::test]
    async fn rmcp_protocol_forwards_typed_pipeline_and_operation_calls() {
        async fn mock_agent_call(
            State(calls): State<Arc<AsyncMutex<Vec<AgentCall>>>>,
            Json(call): Json<AgentCall>,
        ) -> Json<AgentReply> {
            let data = if call.method == "ui.get_context" {
                serde_json::json!({
                    "project_id": "project-7",
                    "revision_id": "42",
                    "segment_ids": []
                })
            } else {
                serde_json::json!({"operation_id": "operation-9", "artifact": null})
            };
            calls.lock().await.push(call.clone());
            Json(AgentReply {
                request_id: call.request_id,
                data,
                sequence: "1".into(),
            })
        }

        let calls = Arc::new(AsyncMutex::new(Vec::new()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock loopback bind");
        let endpoint = format!("http://{}/", listener.local_addr().expect("mock address"));
        let router = Router::new()
            .route("/v1/agent/call", post(mock_agent_call))
            .with_state(Arc::clone(&calls));
        let bridge_task = tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });

        let server = McpServer::new(
            SidecarConfig::new(endpoint, "protocol-test-token").expect("loopback sidecar config"),
        )
        .expect("MCP server");
        let (server_transport, client_transport) = tokio::io::duplex(16 * 1024);
        let server_task = tokio::spawn(async move {
            let running = server
                .serve(server_transport)
                .await
                .expect("server initialization");
            let _ = running.waiting().await;
        });
        let client = ().serve(client_transport).await.expect("MCP initialize");
        let result = client
            .call_tool(
                CallToolRequestParams::new("jueming_pipeline_execute").with_arguments(
                    serde_json::json!({
                        "binding_id": "binding-3",
                        "method_id": "018f5b4c-0000-7000-8000-000000000001",
                        "method_revision_id": "018f5b4c-0000-7000-8000-000000000002",
                        "segment_id": "018f5b4c-0000-7000-8000-000000000003",
                        "base_revision_id": "42",
                        "operation_id": "operation-9"
                    })
                    .as_object()
                    .expect("object arguments")
                    .clone(),
                ),
            )
            .await
            .expect("tools/call response");
        assert!(
            !result.is_error.unwrap_or(true),
            "typed pipeline call failed"
        );

        let received_calls = calls.lock().await;
        assert_eq!(
            received_calls.len(),
            2,
            "binding preflight and Pipeline request expected"
        );
        assert_eq!(received_calls[0].method, "ui.get_context");
        assert_eq!(received_calls[0].binding_id.as_deref(), Some("binding-3"));
        assert_eq!(received_calls[1].method, "pipeline.execute");
        assert_eq!(received_calls[1].binding_id.as_deref(), Some("binding-3"));
        assert_eq!(
            received_calls[1].params,
            serde_json::json!({
                "method_id": "018f5b4c-0000-7000-8000-000000000001",
                "method_revision_id": "018f5b4c-0000-7000-8000-000000000002",
                "segment_id": "018f5b4c-0000-7000-8000-000000000003",
                "base_revision_id": "42",
                "operation_id": "operation-9"
            })
            .as_object()
            .expect("expected object")
            .clone()
        );
        drop(received_calls);

        let result = client
            .call_tool(
                CallToolRequestParams::new("jueming_get_operation").with_arguments(
                    serde_json::json!({
                        "binding_id": "binding-3",
                        "operation_id": "operation-9"
                    })
                    .as_object()
                    .expect("object arguments")
                    .clone(),
                ),
            )
            .await
            .expect("operation tools/call response");
        assert!(
            !result.is_error.unwrap_or(true),
            "typed operation call failed"
        );
        let received_calls = calls.lock().await;
        assert_eq!(
            received_calls.len(),
            4,
            "operation also requires a preflight"
        );
        assert_eq!(received_calls[2].method, "ui.get_context");
        assert_eq!(received_calls[3].method, "operation.get");
        assert_eq!(
            received_calls[3].params,
            serde_json::json!({"operation_id": "operation-9"})
                .as_object()
                .expect("expected object")
                .clone()
        );

        drop(received_calls);
        drop(client);
        server_task.abort();
        bridge_task.abort();
    }

    #[tokio::test]
    async fn rmcp_protocol_updates_search_spec_and_returns_a_paginated_session() {
        async fn mock_agent_call(
            State(calls): State<Arc<AsyncMutex<Vec<AgentCall>>>>,
            Json(call): Json<AgentCall>,
        ) -> Json<AgentReply> {
            let data = match call.method.as_str() {
                "ui.get_context" => serde_json::json!({
                    "project_id": "project-7",
                    "revision_id": "42",
                    "segment_ids": []
                }),
                "search.execute" | "search.get_results" => serde_json::json!({
                    "session_id": "search-session-8",
                    "revision_id": "42",
                    "hits": [],
                    "next_cursor": null,
                    "total": 0
                }),
                _ => serde_json::json!({"valid": true}),
            };
            calls.lock().await.push(call.clone());
            Json(AgentReply {
                request_id: call.request_id,
                data,
                sequence: "1".into(),
            })
        }

        let calls = Arc::new(AsyncMutex::new(Vec::new()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock loopback bind");
        let endpoint = format!("http://{}/", listener.local_addr().expect("mock address"));
        let router = Router::new()
            .route("/v1/agent/call", post(mock_agent_call))
            .with_state(Arc::clone(&calls));
        let bridge_task = tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });
        let server = McpServer::new(
            SidecarConfig::new(endpoint, "protocol-test-token").expect("loopback sidecar config"),
        )
        .expect("MCP server");
        let (server_transport, client_transport) = tokio::io::duplex(16 * 1024);
        let server_task = tokio::spawn(async move {
            let running = server
                .serve(server_transport)
                .await
                .expect("server initialization");
            let _ = running.waiting().await;
        });
        let client = ().serve(client_transport).await.expect("MCP initialize");

        let search = client
            .call_tool(
                CallToolRequestParams::new("jueming_search").with_arguments(
                    serde_json::json!({
                        "binding_id": "binding-3",
                        "query": "needle",
                        "regex": false,
                        "case_sensitive": true,
                        "language_id": "zh-CN",
                        "page_size": 9999,
                        "expected_revision_id": "42"
                    })
                    .as_object()
                    .expect("object arguments")
                    .clone(),
                ),
            )
            .await
            .expect("search tools/call response");
        assert!(!search.is_error.unwrap_or(true), "typed search call failed");
        assert_eq!(
            search
                .structured_content
                .as_ref()
                .and_then(|value| value.get("data"))
                .and_then(|value| value.get("session_id"))
                .and_then(Value::as_str),
            Some("search-session-8"),
            "search execute must expose the host pagination session"
        );

        let page = client
            .call_tool(
                CallToolRequestParams::new("jueming_search_results").with_arguments(
                    serde_json::json!({
                        "binding_id": "binding-3",
                        "session_id": "search-session-8",
                        "cursor": 25,
                        "page_size": 9999
                    })
                    .as_object()
                    .expect("object arguments")
                    .clone(),
                ),
            )
            .await
            .expect("search page tools/call response");
        assert!(
            !page.is_error.unwrap_or(true),
            "typed search page call failed"
        );

        let received_calls = calls.lock().await;
        let methods: Vec<_> = received_calls
            .iter()
            .map(|call| call.method.as_str())
            .collect();
        assert_eq!(
            methods,
            [
                "ui.get_context",
                "search.execute",
                "ui.get_context",
                "search.get_results",
            ]
        );
        assert_eq!(
            received_calls[1].params,
            serde_json::json!({
                "spec": {
                    "query": "needle",
                    "regex": false,
                    "case_sensitive": true,
                    "language_id": "zh-CN"
                },
                "page_size": 9999,
                "expected_revision_id": "42"
            })
            .as_object()
            .expect("expected object")
            .clone()
        );
        assert_eq!(
            received_calls[3].params,
            serde_json::json!({
                "session_id": "search-session-8",
                "cursor": 25,
                "page_size": 9999
            })
            .as_object()
            .expect("expected object")
            .clone()
        );

        drop(received_calls);
        drop(client);
        server_task.abort();
        bridge_task.abort();
    }
}
