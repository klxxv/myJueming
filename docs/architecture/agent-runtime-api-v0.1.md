# Agent Runtime API v0.1

## Main integration handoff

Main must register `crates/jueming-agent-runtime` as a workspace member and add this desktop dependency:

```toml
jueming-agent-runtime = { path = "../../../crates/jueming-agent-runtime" }
```

The runtime crate depends on `jueming-application`, `reqwest 0.12` with `json,rustls-tls`, `rusqlite 0.32` with `bundled`, and `keyring 3.6` with `apple-native,windows-native`. Update the root lockfile only after registering the member. The crate owns no Tauri state and no project storage.

Desktop state constructs the runtime once:

```rust
pub fn initialize(host: Arc<LocalAppHost>, app_data: impl AsRef<Path>) -> Result<Arc<AgentRuntime>, RuntimeError>;
```

It then exposes these native-only command names. `api_key` is accepted solely by `agent_runtime_configure`; it is never included in a result, event, persisted row, or status response.

```rust
agent_runtime_status() -> Result<RuntimeStatus, String>
agent_runtime_configure(config: RuntimeConfigurationInput) -> Result<RuntimeConfiguration, String>
agent_runtime_start(request: RunRequest) -> Result<RunHandle, String>
agent_runtime_cancel(run_id: String) -> Result<(), String>
agent_runtime_history(request: HistoryRequest) -> Result<SessionHistory, String>
agent_runtime_sessions(project_id: String) -> Result<Vec<SessionSummary>, String>
```

The native process manages `AgentRuntimeState` as an optional capability. If opening `<app-data>/agent-runtime.sqlite3` fails, desktop startup continues with the core project and MCP transport available; every runtime command, including `agent_runtime_status`, returns the explicit error `embedded agent runtime is unavailable because its local chat storage could not be initialized`. Runtime commands run their SQLite and host-validation work on Tauri blocking workers.

The desktop process relays `RuntimeEvent` on the Tauri event channel `agent_runtime_event`. Events have `kind` values `run_started`, `tool_started`, `tool_finished`, `awaiting_approval`, `run_completed`, `run_cancelled`, and `run_failed`.

The desktop process emits `agent_runtime_resync` when main must reload durable runtime state after `project_changed`, `runtime_recovered`, or an `event_gap`. Its payload is `{ reason, project_id?, session_id? }`. The frontend client exposes `subscribeResync(listener)` for this channel; the main UI uses it to reload `status`, `sessions(project_id)`, and `history` rather than inferring missed events.

## Runtime contract

`AgentRuntimePort` has `status`, `configure`, `start`, `cancel`, `history`, `sessions`, and `subscribe` methods. `ModelProvider` has one `complete(ModelRequest) -> Future<Result<ModelCompletion, RuntimeError>>` operation. The default provider sends OpenAI-compatible `/chat/completions` requests to `http://127.0.0.1:11434/v1`; its endpoint validation only accepts a loopback host. A remote provider is possible only with `provider_kind: "https"` and an explicit `https://` endpoint. There is no remote fallback.

`RunRequest.context` is a required immutable `ContextSnapshot` frozen by the frontend when Send is pressed. Native code calls `ui.get_context` using its submitted `binding_id`; the host validates project and revision before the runtime validates the frozen project/revision/binding, each stable Segment/Alignment reference, and UTF-16 selected-text range. Focus and capture timestamps are intentionally not compared with later live UI state. The submitted immutable snapshot is persisted with the user turn and never updated in-place. Tools are JSON-schema-defined, have no shell capability, and dispatch only an allowlist through `LocalAppHost::dispatch`: `project.get_summary`, `segment.get`, `alignment.get`, `search.set_spec`, `search.execute`, `search.get_results`, `proposal.preview_replace`, `proposal.list`, `proposal.reject`, `ui.navigate`, `ui.reveal`, `pipeline.list`, `pipeline.get`, `pipeline.preview_update`, `pipeline.list_proposals`, and `pipeline.execute`. `proposal.approve`, `pipeline.create_default`, `pipeline.update`, and `pipeline.approve_update` are not runtime tools.

Tool/model turns and response bytes are bounded. A preview proposal pauses the run in `awaiting_approval`; a native `proposal_changed` event is only a wake-up signal. Before resuming, the runtime binds the current project and queries `proposal.list`, then resumes only if the exact saved `proposal_id` is approved. A rejected proposal, explicit `cancel(run_id)`, or `project_changed` for another project terminates the paused run, persists `cancelled`, and emits one `run_cancelled` event. An approved proposal that cannot be verified against the current local project terminates as `failed`, so it cannot wait indefinitely. It never repeats the completed preview tool. The runtime never invokes native approval.

`pipeline.execute` accepts no model-supplied operation ID in its schema. The runtime injects a newly generated `agent-runtime:<run-id>:<uuid-v7>` ID for each dispatch, isolating concurrent calls from different sessions even if a model repeats a tool-call ID.

Conversations are stored in `<app-data>/agent-runtime.sqlite3`, outside `.jm`, keyed by project id. `history` without `session_id` restores the latest session; `sessions(project_id)` supplies durable session IDs for a chooser. On startup, persisted `running` and `awaiting_approval` rows are changed to `failed`, so status never reports invisible active work from a prior process. `configured` is false until the user saves a non-empty endpoint/model; it indicates saved configuration only, never provider reachability. The database holds prompts, responses, tool result JSON, immutable context JSON, and run state, but never an API key. API keys are scoped to normalized provider kind and endpoint in Windows Credential Manager/macOS Keychain through `keyring`; changing endpoint without supplying a key never reuses the prior endpoint's key. If that store cannot be initialized, the key is held only in process memory and `secret_storage: "session_memory"` is returned explicitly.

## Why the thin provider

Rig 0.42 documents dynamic tools and runtime model swapping, but its current migration guide changes provider capabilities, streaming result handling, and agent/model type erasure. M4 needs a stable OpenAI-compatible local loopback protocol, durable resumable approval state, and an exact host-dispatch boundary today. The runtime therefore keeps a small provider trait and JSON wire implementation; Rig may later be adapted behind `ModelProvider` without changing native IPC or persisted conversation data. [Rig AgentBuilder](https://docs.rs/rig/latest/rig/agent/struct.AgentBuilder.html) [Rig 0.42 migration notes](https://docs.rs/crate/rig/latest/source/MIGRATING.md)
