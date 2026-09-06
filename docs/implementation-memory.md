# Implementation memory

## 2026-09-06 — agent / MCP / Pipeline

- User changed planning-only scope to execution and explicitly requested branch, milestone commits/pushes, many parallel `gpt-5.6-terra` high workers and independent QA. Main agent owns acceptance and redispatch.
- Branch: `codex/agent-mcp-pipeline`. Preexisting MVP source/settings/refactor changes preserved as a separate baseline commit. Existing artifacts and third-party references must not be swept into commits.
- Authoritative current handoff: [agent implementation](architecture/agent-implementation-handoff-v0.1.md). Existing Phase 0 invariants remain binding.
- Priority: external AI can control Jueming through MCP while host events synchronize UI, before built-in Agent panel. AG-UI does not imply access to external clients' full conversations.
- User UI: global optional agent/annotations; Pipeline sibling of History, Settings below Pipeline; lower-left cat tree/bed over doghouse; bottom cassia garden expands for action/approval, butterfly is guidance; agent sees tab/focus/selected content. Static minimal/hidden modes retain normal controls.
- Preserve Windows/macOS keyboard, trackpad, accessibility, reduced motion and eye-care theme. No fake unavailable providers. No real project corpora, credentials or generated build artifacts committed.

### Integration decisions and handoffs

- Baseline `9300cfa` and contract `0db0336` were committed and pushed before feature integration. Six Terra/high workers implemented disjoint scopes; independent QA returned real failures to implementation owners. Main rerouted follow-up work rather than silently waiving gates.
- Native `agent_subscribe` is an event, not an invoke command. `agent_call` / `agent_projection` run host work through blocking workers. An external binding and the native window binding are different; ACK checks exact request/operation/project/revision, not binding equality.
- Native and MCP search now pass a complete SearchSpec to atomic `search.execute`; set-spec then execute was a real multi-client race. UI replacement reviews use Rust results with frozen request/revision, not JavaScript replacement output.
- Canonical replacement proposals store a preallocated operation ID for crash reconciliation. Pipeline methods/artifacts have separate method revisions and persistent proposals; running cancellation is bound to its initiator. Host and transport own no second canonical writer.
- Built-in runtime is a separate optional crate and provider port. SQLite/credential initialization failure disables chat while leaving application/MCP available. Credentials are endpoint scoped; changing endpoint must not reuse a previous service's key. No configured provider means no model call.
- Review actions have independent busy state: a run waiting for approval must not disable its own approval buttons. Project switches must not clear another project's chat draft or apply late method/search responses.
- Vue Flow Pipeline and companion animation backends are separate dynamic chunks. Default companion is static; future Spine/Pixi assets go through CompanionRenderer. Hidden garden retains normal DOM review controls. Original SVG cats/dog/plant/butterfly were added only under `src/assets/companion`.
- Browser UI QA uses 4173 because Windows reserved TCP range includes1420. Repository dev URL remains127.0.0.1:1420. 1440×900 and minimum1180×760 UI layouts, global sidebar persistence, garden toggle and eye theme were inspected. Native interaction/macOS hardware tests require their own evidence.
- Production-import frontend tests initially resolved `esbuild` from the user's ancestor directory. Added exact workspace dev dependency plus cross-platform runner and CI step; do not rely on machine-global modules.
- Delivery map and unimplemented extension boundaries: [delivery status](agent-delivery-status-v0.1.md). Final executable/test evidence belongs in [QA](testing/agent-integration-qa-v0.1.md). Arbitrary third-party plugin execution, outbound MCP client, Spine assets and method-node annotation anchors are not advertised as implemented.
- Final Pipeline recovery uses a host-reserved method revision ID persisted before approval commit, checked against exact revision history. Same-content manual saves are not approval evidence; subsequent method revisions do not hide a real approved commit.
- Runtime maps host method names to explicit `jueming_*` provider function aliases and back. A local HTTP server test verifies actual request JSON and response mapping; approved Pipeline proposals now resume the tool loop through real host events.
- Final local gates passed: locked install, frontend typecheck/build, 10 production-import UI contracts, workspace fmt/Clippy/tests and Windows Tauri debug build. Fresh production preview on 4174 loaded Pipeline and global assistant with no browser warnings/errors. macOS Universal compilation is delegated to the branch CI; native hardware interaction and real provider configuration remain separate evidence.
