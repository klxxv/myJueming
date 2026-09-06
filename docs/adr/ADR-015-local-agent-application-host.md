# ADR-015 — Local application host and agent adapters

Status: Accepted for the user-authorized agent extension, 2026-09-06.

## Context

External AI tools must operate the currently open Jueming project and update its visible UI without requiring an embedded chat window. Existing native commands own a mutex-protected snapshot; a second MCP-owned snapshot would diverge. Navigation, selection, search drafts and conversations are application state, not canonical Segment data.

## Decision

Extract one Rust `LocalAppHost` used by native commands and external protocol adapters. Keep domain operations in KernelService and all existing IDs, append-only revisions and sidecar invariants. Native typed facades carry application operations; MCP exposes an allowlisted schema-validated subset. External canonical changes create immutable, revision-bound proposals. Approval is a trusted native action, unavailable through MCP, AG-UI state patches or model tools. Revalidate project binding, revision and proposal identity when committing.

Application events have ordered decimal-string sequence numbers and a versioned envelope. Subscribe before acquiring a consistent projection watermark; deduplicate, detect gaps and resnapshot. Transport delivery and UI acknowledgement do not constitute a canonical commit. All native changes use the same publisher. UI focus and selection come from event-driven view context providers; password/configuration credentials are excluded. Capture an immutable context per agent request, distinguish application focus from last content selection, and invalidate project references on project switch.

MCP and AG-UI are adapters. MCP external tools do not imply access to the external agent's full transcript. AG-UI projects activity and state; it cannot directly patch canonical data. External access is explicitly enabled in local settings, authenticated and revocable. Model execution uses a separate replaceable runtime/provider port with no implicit remote fallback. Credentials stay outside `.jm`, frontend storage and logs.

Pipeline methods and artifacts require a separate decision. Global assistant/annotation UI lives outside page routes. Companion renderers only consume activity and navigation outcomes. Normal accessible DOM controls own approval; static or hidden mode retains the same functionality. Dynamic backends must dispose listeners, render loops and GPU resources on disable.

## Consequences

This expands the previously deferred agent and Pipeline scope under the current user's explicit request. POS, Lemma, NER, OCR, cloud collaboration and automatic semantic alignment remain deferred. Capability registries report real availability. Windows and macOS transports, keyboard navigation and reduced-motion modes must be tested separately. A local bridge never silently creates another writer for an already open project.
