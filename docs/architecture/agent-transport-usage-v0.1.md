# Local agent transport usage v0.1

`jueming-agent-transport` is the Desktop-owned, local-only boundary between the single `LocalAppHost` and local consumers. It does not open a project, acquire a `.jm` lock, retain a project snapshot, or write canonical data. The Desktop creates one controller against its already-running host:

```rust
use std::sync::Arc;
use jueming_agent_transport::{AuthToken, ServerConfig, ServerHandle, start_server};
use jueming_application::LocalAppHost;

let token = AuthToken::new(local_secret_from_secure_storage)?;
let controller: ServerHandle = start_server(
    Arc::clone(&host),
    ServerConfig::disabled(token),
).await?;

// Called only from the explicit local Settings action.
let running = controller.set_enabled(true).await?;
let status = controller.status();
controller.stop().await?;
```

The public contract is:

```rust
pub async fn start_server(
    host: Arc<LocalAppHost>,
    config: ServerConfig,
) -> Result<ServerHandle, TransportError>;

impl ServerHandle {
    pub fn status(&self) -> ServerStatus;
    pub async fn set_enabled(&self, enabled: bool) -> Result<ServerStatus, TransportError>;
    pub async fn stop(&self) -> Result<(), TransportError>;
}
```

`ServerConfig::disabled(AuthToken)` is the construction default and binds no listener. When enabled, the implementation uses a bounded, authenticated HTTP loopback connection (`127.0.0.1` or `::1` only) because the Windows named-pipe and Unix-domain-socket adapter has not yet received the required cross-platform lifecycle and packaging validation. It is a bridge between two processes on the same device, never a remotely reachable server. The config rejects wildcard and non-loopback bind addresses.

Every request requires `Authorization: Bearer <token>`. For this increment, the Desktop generates a CSPRNG token at process start and keeps it only in native memory. The explicit native Copy configuration command is the only way to reveal it to the local user; status never contains it. The token is supplied to the sidecar through its inherited environment, never a command-line argument or log line. It expires on Desktop restart, and the user must copy the new configuration again. Runtime provider API keys remain in secure storage or are explicitly session-only; they never share this transport token. Requests are limited to 1 MiB by default and the host call has a 10-second deadline. The bridge emits no CORS headers. Native sidecars may omit `Origin`; browser requests with an Origin must exactly match `ServerConfig.allowed_origins`, otherwise they are rejected before host access. Only `app.describe` and `app.bind_session` are binding-free. The bridge requires a nonempty `binding_id` for every other external call and preflights it through `ui.get_context`, so even methods whose host implementation does not directly read a binding cannot serve a changed project through a stale session.

The endpoint surface is intentionally small:

| Endpoint | Purpose |
| --- | --- |
| `POST /v1/agent/call` | Authenticated typed `AgentCall` forwarded via `spawn_blocking` to `LocalAppHost::dispatch`. |
| `GET /v1/ag-ui/events` | Authenticated SSE of application events only. |

There is no HTTP endpoint for `app.get_projection`. Full `ProjectSnapshot` is a trusted native frontend resync payload and must not be exposed through MCP or AG-UI. The SSE adapter emits only AG-UI `CUSTOM` events: `jueming.app_event` wraps one `AppEvent`, and `jueming.resync_required` reports a broadcast gap. It never fabricates chat-token events. Consumers that receive a gap must use their native trusted projection path; external consumers bind again and issue targeted reads. Disabling revokes acceptance before graceful shutdown, actively closes SSE streams, waits at most two seconds for the server, then aborts it; concurrent enable/disable calls are serialized.

## MCP sidecar

`jueming-mcp` is launched by an MCP host and communicates exclusively over standard input/output using the official `rmcp` SDK. Its stdout is reserved for MCP protocol frames; diagnostics go to stderr. It connects to the already enabled loopback bridge using `JUEMING_AGENT_ENDPOINT` and `JUEMING_AGENT_TOKEN`.

For a launcher or package diagnostic, run `jueming-mcp --smoke` with those two environment variables. The harness performs only `app.describe`, writes its result only to stderr, and exits nonzero if the authenticated local bridge cannot be reached. It does not load a project or start an MCP protocol session.

The sidecar does not load a project. A client first calls `jueming_bind_session`, then provides the returned stable `binding_id` to every target read. The sidecar verifies a binding through the host before forwarding target reads, so project changes and revision advances return the host's `stale_binding` or `stale_revision` error instead of silently serving a new project.

The externally allowed operations are `app.describe`, `app.bind_session`, sanitized context reads, project summary, targeted segment/alignment reads, regex validation and search reads, proposal preview/list/reject, Pipeline list/get/preview-update/list-proposals/reject-update/execute/artifacts/cancel, `ui.navigate` / `ui.reveal`, and operation status/cancellation. Navigation and reveal only emit a request to the native UI; that UI keeps draft gating, focus decisions, and butterfly guidance. Clients can poll `jueming_get_operation` until the operation status becomes `ui_applied`, or use `jueming_cancel_operation` while it remains pending. `app.get_projection`, `ui.publish_context`, `ui.ack`, `proposal.approve`, `pipeline.create_default`, `pipeline.update`, and `pipeline.approve_update` are not MCP tools. Approval remains a trusted native UI operation.

The MCP names deliberately use a stable `jueming_` prefix while their bridge methods remain dotted application names: `jueming_describe` and `jueming_application_settings_metadata` → `app.describe`, `jueming_bind_session` → `app.bind_session`, `jueming_navigate` → `ui.navigate`, `jueming_reveal` → `ui.reveal`, `jueming_get_operation` → `operation.get`, `jueming_cancel_operation` → `operation.cancel`, `jueming_project_summary` → `project.get_summary`, `jueming_get_segment` → `segment.get`, `jueming_get_alignment` → `alignment.get`, `jueming_search` → atomic `search.execute`, `jueming_search_results` → `search.get_results`, and proposal tools → their corresponding `proposal.*` methods. `jueming_search` keeps its existing query/regex/case/language/page-size input fields and additionally accepts optional `expected_revision_id`; the sidecar maps them to `{ spec, page_size, expected_revision_id? }` in one host request. Both operation MCP tools take `{ binding_id, operation_id }`; the bridge carries the binding separately and forwards only `{ operation_id }`. Pipeline mappings are `jueming_pipeline_list` → `pipeline.list`, `jueming_pipeline_get` → `pipeline.get`, `jueming_pipeline_preview_update` → `pipeline.preview_update`, `jueming_pipeline_list_proposals` → `pipeline.list_proposals`, `jueming_pipeline_reject_update` → `pipeline.reject_update`, `jueming_pipeline_execute` → `pipeline.execute`, `jueming_pipeline_artifacts` → `pipeline.artifacts`, and `jueming_pipeline_cancel` → `pipeline.cancel`. `jueming_tool_discovery` reports metadata for the currently bound sidecar tools only; it cannot load executable plugins because no shared `ToolProvider` contract exists. QA may call these loopback bridge dotted methods directly with the same typed parameter shapes; the generic bridge still rejects native-only methods.

MCP resources do not enumerate project text. A static guide resource and URI templates identify the required `binding_id` query parameter; concrete reads resolve to the same targeted host operations. Resource and tool results are structured JSON with an MCP text fallback. No credentials, settings secrets, local filesystem paths, or full snapshots are published.

## Current binding semantics

`search.execute` validates, stores, and executes the supplied `SearchSpec` while holding the host operation lock, so a client cannot interleave another `set_spec` between its update and execution. It returns the first page of a host-created `search_session`; `jueming_search_results` reads later pages by cursor. The host clamps every requested page size to `1..=200` and rejects a supplied `expected_revision_id` that is no longer current.

For external UI operations, `operation.get` and `operation.cancel` require the exact initiating binding and current project. A mismatched binding receives `operation_forbidden`. Trusted native `ui.ack` may use its distinct native binding for the same current project and revision; request-ID and epoch checks still apply. Pipeline cancellation also requires the exact initiating binding: the host records `initiating_binding_id` for each active run and rejects cancellation from another bound client, even when both bindings target the same project.
