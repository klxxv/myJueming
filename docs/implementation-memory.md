# Implementation memory

## 2026-09-06 — agent / MCP / Pipeline

- User changed planning-only scope to execution and explicitly requested branch, milestone commits/pushes, many parallel `gpt-5.6-terra` high workers and independent QA. Main agent owns acceptance and redispatch.
- Branch: `codex/agent-mcp-pipeline`. Preexisting MVP source/settings/refactor changes preserved as a separate baseline commit. Existing artifacts and third-party references must not be swept into commits.
- Authoritative current handoff: [agent implementation](architecture/agent-implementation-handoff-v0.1.md). Existing Phase 0 invariants remain binding.
- Priority: external AI can control Jueming through MCP while host events synchronize UI, before built-in Agent panel. AG-UI does not imply access to external clients' full conversations.
- User UI: global optional agent/annotations; Pipeline sibling of History, Settings below Pipeline; lower-left cat tree/bed over doghouse; bottom cassia garden expands for action/approval, butterfly is guidance; agent sees tab/focus/selected content. Static minimal/hidden modes retain normal controls.
- Preserve Windows/macOS keyboard, trackpad, accessibility, reduced motion and eye-care theme. No fake unavailable providers. No real project corpora, credentials or generated build artifacts committed.
