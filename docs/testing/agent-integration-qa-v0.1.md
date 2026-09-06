# Agent / MCP / Pipeline integration QA

Status: final QA handoff, 2026-09-06 (Windows).

## Final Windows gate

| Check | Result | Evidence |
| --- | --- | --- |
| `cargo fmt --all -- --check` | pass | completed with no formatting discrepancy |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass | all registered application, transport, runtime, MCP, pipeline, and desktop targets completed cleanly |
| `cargo test --workspace` | pass | all unit, integration, and doc tests passed |
| `corepack pnpm --dir apps/desktop tauri build --debug --no-bundle` | pass | `vue-tsc --noEmit`, Vite production build, Rust debug link, and `target/debug/jueming-desktop.exe` all completed; recorder exit code `0` |
| `node tests/agent-context/search-production-component-contract.mjs` | pass | production SFC with TanStack virtualizer mounted using a minimal viewport surface |

The workspace test output emitted one existing Windows linker message while
building `jueming-desktop`; it remained a warning and did not affect the
passing result.

## Focused production acceptance evidence

The final workspace test run includes runtime 12/12, transport 9/9,
application host integration 12/12, MCP 6/6, and Pipeline service 5/5. It
exercises the OpenAI-compatible HTTP function aliases and response mapping,
native approval continuation for a pipeline method proposal, real paginated
search, exact binding acknowledgement, native-only approval, and Pipeline's
host-reserved exact method-revision ID. The application gateway tests also
cover matching canonical/method revisions, host snapshot authority, and
cancellation releasing the initiating binding.

The real host/MCP subprocess and two-binding evidence remains passed on
Windows: the QA fixture created a new temporary `.jm` project, negotiated the
actual stdio MCP sidecar, consumed AG-UI SSE, and verified external-A/native-B
navigation acknowledgement, wrong-project rejection, and terminal late-ack
rejection. MCP tools expose frozen `jueming_*` wire names and exclude
native-only operations.

The QA-owned `crates/jueming-agent-transport/examples/qa_fixture_server.rs`
creates only a new caller-supplied temporary `.jm` project, binds an ephemeral
loopback bridge, prints only a readiness endpoint, and receives its token only
from `JUEMING_AGENT_TOKEN`. It neither opens an existing project nor prints
project text or secrets. For QA-only two-binding coverage it accepts
`ack <operation_id> <request_id>`, `wrong-project-ack <operation_id>
<request_id>`, and `shutdown` on stdin. These commands are unavailable in the
production bridge.

The main thread additionally verified the current production preview at its
temporary Vite address: switching Pipeline and assistant panels emitted no
warning or error. A one-time development-server dynamic-module fetch failure
during dependency optimization did not reproduce in the fresh production
load, so it is recorded as development-only history and is not a production
blocker.

## Scope and remaining platform evidence

Production-path QA covers the typed desktop client, real Vue proposal and
search components, loopback HTTP transport, AG-UI SSE, stdio MCP, and host
lifecycle acknowledgements. The subprocess fixtures create only small
temporary projects and never use a real user project.

`event-stream-contract.mjs` and `renderer-lifecycle-contract.mjs` remain
supplemental models; they are not counted as host, transport, or browser
evidence. This Windows task did not perform manual native UI interaction and
did not run an actual macOS build or UI session. Those remain separate
platform evidence; no macOS pass is claimed here.

## Reusable checks

Run event sequence reconciliation independently:

```powershell
node tests/agent/event-stream-contract.mjs
```

This is a small supplemental contract model only. It is not M1 transport
evidence; the real-source adapter coverage is the next check and the
subprocess smoke is the production transport evidence.

Run the real frontend adapter through a mock Tauri bridge (including
subscribe-before-projection, duplicate discard, gap resync, binding envelope,
and listener disposal):

```powershell
node tests/agent/agent-client-integration.mjs
```

Run the renderer disposal contract independently:

```powershell
node tests/agent/renderer-lifecycle-contract.mjs
```

This is likewise supplemental until the production companion renderer exposes
its lifecycle API to a browser-level test.

Run the production right-panel proposal regression:

```powershell
node tests/agent/agent-panel-proposal-availability.mjs
```

This uses the Vue SFC compiler and a real Vue renderer to mount
`GlobalSidePanel → AgentPanel → AgentActionBar`. With no `CompanionGarden`
mounted, it verifies that `running=true, reviewing=false` leaves proposal
approve/reject enabled and emits their real events, while `reviewing=true`
disables both buttons. The current Windows execution passed.

Run the production native-search and native-preview component regression:

```powershell
node tests/agent-context/search-production-component-contract.mjs
```

The test compiles and mounts the production `SearchReplaceWorkspace.vue` with
the Vue SFC compiler and renderer. It proves authoritative Rust results remain
visible when the supplied JavaScript regex is invalid, blocks null/loading/
error/empty native previews, renders both source and target before/after
entries, and emits the immutable displayed native preview after live inputs
change. The test is discovered automatically by `corepack pnpm test:agent-ui`.
The current Windows execution passed.

Run the external MCP stdio acceptance test once the transport binary can start without a browser:

```powershell
$env:JUEMING_MCP_COMMAND = "<transport-executable>"
$env:JUEMING_MCP_ARGS = '["--stdio"]'
node tests/agent/mcp-stdio-acceptance.mjs
```

The MCP test validates `initialize`, `notifications/initialized`, `tools/list`, `app.describe`, invalid regex handling, and that external `proposal.approve` is rejected. It has no interval polling and terminates its child process on completion.

The current transport implementation instead exposes an authenticated loopback
HTTP call endpoint and AG-UI SSE. Exercise that implementation without
altering the workspace by running:

```powershell
powershell -ExecutionPolicy Bypass -File tests/agent/run-transport-http-acceptance.ps1
```

The script generates a temporary Rust probe outside the repository, starts the
registered transport with an ephemeral loopback port, verifies `app.describe`,
rejects an absent bearer token, and verifies external `proposal.approve` is
blocked. It removes only its own resolved temporary directory.

Run the full real-process M1 acceptance using:

```powershell
node tests/agent/m1-real-subprocess-acceptance.mjs
```

It starts the QA fixture server against a newly created temporary project,
starts the actual `jueming-mcp` stdio sidecar, verifies MCP negotiation/tool
listing/native-only exclusion/binding/search/navigation/reveal, consumes the
actual AG-UI SSE stream, requests fixture shutdown, and removes only the temp
fixture root. It never prints its token or fixture content.

Current Windows evidence: `node tests/agent/m1-real-subprocess-acceptance.mjs`
passed. It built and started the real QA fixture server against a fresh
temporary `.jm` project, negotiated the actual `jueming-mcp` stdio sidecar,
called binding, invalid/valid regex search, navigation and reveal, observed
the production AG-UI SSE `search_changed`, `navigation_requested`, and
`reveal_requested` events, then requested clean fixture shutdown. The fixture
also verifies the host accepts a top-level native `ContextSnapshot` carrying a
current stable segment UUID before exposing the bridge.

### QA finding Q-08 — resolved: external and native bindings are intentionally distinct

`tests/agent/run-two-binding-workspace-ack.mjs` bundles and runs the real
`useAgentWorkspace` composable. It supplies an event emitted under external
binding A and asserts that the same-project native workspace executes it and
acknowledges the exact `{ operation_id, request_id }` with its distinct native
binding B. It does not inject DOM state or replace the composable.

`tests/agent/two-binding-host-acceptance.mjs` starts the production transport
fixture and drives client A through the authenticated HTTP bridge. It observes
the actual AG-UI navigation event, then has fixture-owned trusted native B
call real `ui.ack`. It verifies `ui_applied`; cancels a second action and
verifies a late native acknowledgement returns `operation_terminal` while the
operation remains `cancelled`; then switches the fixture to a separate
temporary project and verifies that its new native binding cannot acknowledge
an earlier-project action. The current Windows execution passed.

## Required M1 acceptance matrix

| Risk | Observable assertion | Test layer |
| --- | --- | --- |
| Binding and project switch | `app.bind_session` returns string IDs; switching projects invalidates prior binding; an old binding is rejected | host + MCP integration |
| Immutable selection context | `ui.publish_context` keeps stable IDs and UTF-16 offsets; cross-card selection sends no text range; private controls never appear | frontend/unit + native adapter |
| Regex and search | invalid regex rejects; valid language-scoped search returns stable segment IDs and revision-bound results | host/integration |
| Navigation and reveal | event has request ID and stable target ID; native UI completes navigation/reveal before `ui.ack` | Tauri/UI (main owner) |
| Native-only approval | external MCP rejects `proposal.approve`; native facade checks proposal, binding and current revision before one canonical commit | host + MCP integration |
| Revision idempotence | replayed command/proposal operation returns its original result and creates no second revision | host + kernel integration |
| Event recovery | subscribe before projection; discard events at/below snapshot; a gap triggers projection resync | frontend unit using `event-stream-contract.mjs` semantics |
| Static companion lifecycle | static/hidden/reduced modes retain controls; disable removes listeners, frames and GPU resources | renderer unit + browser smoke |
| Global panels | assistant and annotations retain independent project-keyed drafts across tabs | browser/UI (main owner) |

No Windows native UI interaction, actual macOS build, or companion browser
lifecycle verification has been run by this QA work. They remain prerequisites
for a commit that claims complete native cross-platform acceptance.
