# Agent / MCP / Pipeline implementation handoff

Status: implementation started 2026-09-06, branch `codex/agent-mcp-pipeline`.

## Global contract

Jueming remains a local-first Tauri 2 / Vue 3 / Rust application. One authoritative LocalAppHost owns the open project and all canonical writes. External MCP clients, native IPC and a future embedded agent use the same application services. AG-UI is an event adapter, never the authority for project mutations. Preserve all Phase 0 identity, revision, annotation and edit-draft invariants. No fake providers, no hidden remote inference, no direct storage from Vue. Current user explicitly authorizes implementing the planned agent/Pipeline extension, commits and pushes.

## Shared integration boundary (v1)

New Rust crate `jueming-application` exports `LocalAppHost`, `AgentCall`, `AgentReply`, `AppEvent`, `AppError`, `ContextSnapshot`, `SearchSpec`. Native state holds `Arc<LocalAppHost>`. Public transport operations: `host.dispatch(call: AgentCall) -> Result<AgentReply, AppError>`; `host.subscribe() -> tokio::sync::broadcast::Receiver<AppEvent>`; `host.projection() -> Result<AgentReply, AppError>`. Keep dispatch synchronous for bounded initial queries; transports call it through spawn_blocking. Each dispatcher method deserializes and validates a typed request; JSON is limited to the protocol adapter, not domain truth.

Wire envelopes:

```typescript
type AgentCall = { request_id: string; method: string; params: Record<string, unknown>; binding_id?: string | null };
type AgentReply = { request_id: string; data: unknown; sequence: string };
type AppError = { code: string; message: string };
type AppEvent = { contract_version: '1.0'; sequence: string; kind: string; binding_id: string | null; origin: string; payload: unknown };
type SearchSpec = { query: string; regex: boolean; case_sensitive: boolean; language_id: string | null };
type ContextSnapshot = { tab: string; mode: string | null; project_id: string | null; revision_id: string | null; binding_id: string | null; window_focused: boolean; focused_control: string | null; selected_text: string; segment_ids: string[]; alignment_ids: string[]; text_range: { start_utf16: number; end_utf16: number } | null; captured_at: string };
```

Methods: `app.describe`, `app.bind_session`, `app.get_projection`, `ui.get_context`, `ui.get_selection`, `ui.publish_context`, `ui.navigate`, `ui.reveal`, `ui.ack`, `search.validate_regex`, `search.set_spec`, `search.execute`, `search.get_results`, `project.get_summary`, `segment.get`, `alignment.get`, `proposal.preview_replace`, `proposal.list`, `proposal.approve`, `proposal.reject`, `operation.get`, `operation.cancel`. `proposal.approve` is native trusted UI only; external transport MUST reject it. Destructive/canonical tool requests stage reviewable proposals. Binding is invalidated on project switch; stale revision is an error. Any additions must be documented and coordinated.

`app.bind_session` returns `{ binding_id, project_id, revision_id }`. Trusted `app.get_projection`/`projection` returns data `{ binding_id, project, context, search_spec, search_results, proposals, pending_ui_actions }`, where project is a nullable ProjectIdentity containing only `{ project: Project }`; text bodies and workspace structure are fetched through separate Kernel queries. Event kinds also include `pipeline_changed`. Navigation payload is `{ tab, request_id, operation_id, project_id, revision_id }`; reveal adds stable `segment_id?`, `alignment_id?`, or `node_id?`. `agent_subscribe` is a native event name, not an invoke command. Call command is `agent_call` with `{ call }`; projection command is `agent_projection`. Subscribe before loading projection, discard <= snapshot sequence, resnapshot on gaps. Numbers crossing JS for sequence/revision are decimal strings.

External and native clients intentionally have different bindings. An action retains its originating binding for ownership/audit; a trusted native window ACK uses its own binding and the exact action/request IDs, after validating project, revision and lifecycle. `accepted` is not `ui_applied`. Project switches and terminal cancellation prevent late ACK execution. App-only context and project/settings navigation are available with null project/revision. No demo IDs cross native commands.

`search.execute` accepts `{ spec?: SearchSpec, expected_revision_id?: string, page_size?: number }`. Passing `spec` applies and executes it atomically. It returns `{ session_id, revision_id, hits, next_cursor, total }`; page size is bounded to 200. Native UI uses this same entry point and ordered projection events. Manual replacement previews use Kernel's Rust regex results and freeze the request/revision; browser regex highlighting is not mutation authority.

Frontend exported `agentClient.call<T>(method, params?, bindingId?)`, `agentClient.projection()`, `agentClient.subscribe(listener)` and `useAgentWorkspace` integration will be owned by context worker. Integration clarification: `agent_subscribe` is the typed event name (listen before projection), not a Channel command; main owns broadcasting host events in desktop setup. Components only use this facade. UI publishes context on selection/focus/navigation changes, never interval polling. Per-message context captures immutable copies. Never transmit password fields or settings credentials. Segment selection uses stable `data-segment-id`; UTF-16 offsets explicitly match DOM.

Pipeline is a separate `jueming-pipeline` crate: method definitions/revisions, validation and deterministic derived artifacts; it must not mutate canonical Segment text to tokenize. `PipelineService` API and typed contracts are published by owner before integration. Store method history under an explicit non-canonical extension directory only after ADR; no changes to existing history files. Initial operators source -> normalize -> Chinese tokenize -> artifact, no fake POS/NER/OCR. Method annotation anchor extension needs its own contract.

## Ownership and handoff

- Host worker: jueming-application, desktop state/commands (except settings), DTO schema; no frontend, transports or Pipeline crate.
- Transport worker: jueming-agent-transport + jueming-mcp binary crate and transport docs; no host/desktop files. Export `start_server(host: Arc<LocalAppHost>, config: ServerConfig)` contract and send wiring instructions.
- Context worker: frontend domain/agent-client.ts, domain/agent-types.ts, composables/useAgentWorkspace.ts, stores/agent-workspace.ts and tests; no App.vue/settings/component visual files.
- Pipeline worker: jueming-pipeline + its tests and API handoff; no host/frontend.
- Independent QA worker: baseline and regression tests under tests/agent/, review reports under docs/testing; no production implementation changes.
- Platform worker: platform scripts/workflow/docs only; coordinate sidecar packaging with transport owner.
- Main agent: root workspace/dependency integration, ADRs/progress/memory, App.vue/global panels/Pipeline/settings/companion visual UI, review and commits/push.

All workers edit directly, do not commit/push or edit other owners' files. Report files, public signatures, executed checks/results, residual issues. Do not run cargo fmt --all while another agent writes files; format owned files only. Shared root manifests/lockfiles are main-owned; send exact additions and request registration early. Tests run independently; failures return to implementation owner. If agents have isolated forks, provide commit/diff for integration; never discard another worker's changes.

## Milestones / acceptance

1. M1: host + context + search + MCP + AG-UI + native approval; external connection works without embedded chat. Event replay/resnapshot, stale bindings, revision conflict, approval and cancellation tests pass.
2. M2: global assistant/annotation panel across project/search/history/Pipeline/settings, drafts retained on tabs and isolated per project; settings reflect real capabilities.
3. M3: versioned Pipeline method and executable derived tokenizer, external/manual edits synchronize, artifacts include input revision.
4. M4: embedded agent through a replaceable provider/runtime port, persistent conversation and explicit provider configuration; plugin/MCP capability boundaries.
5. M5: static companion and optional isolated animation renderer, garden approvals and butterfly guidance; static/hidden/reduced modes preserve all operations and release animation resources.

Every implementation milestone requires appropriate typecheck/build/Rust tests and native debug build for IPC. Windows and macOS checks remain explicit; a Windows run is not a macOS verification. Commit validated milestones and push the implementation branch, never main. Preexisting reference artifacts and third-party trees stay outside commits.
