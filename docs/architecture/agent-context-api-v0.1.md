# Agent context frontend API (v0.1)

Status: integration handoff for the desktop main worker. This document defines only the Vue-facing adapter boundary; it does not add an embedded model or a browser fallback.

## Dependency and native wiring

No package, root manifest, or lockfile change is needed. The existing `@tauri-apps/api` and Pinia dependencies are sufficient.

The native layer must register these exact Tauri entry points:

- command `agent_call` with argument `{ call: AgentCall }`, returning `AgentReply<T>`;
- command `agent_projection`, returning `AgentReply<AgentProjection>`;
- event channel `agent_subscribe`, whose payload is `AppEvent`.

The renderer registers the event listener before calling `agent_projection`. It deduplicates events at or below the snapshot sequence and obtains a fresh projection if a later event leaves a sequence gap. It never polls.

`ui.publish_context` is dispatched as `agentClient.call("ui.publish_context", { ...contextSnapshot }, bindingId)`. `ContextSnapshot` fields are top-level call parameters; do not wrap them in `{ context: ... }`, because the host deserializes the snapshot directly.

## Main integration

Create the composable from `apps/desktop/src/composables/useAgentWorkspace.ts` after the app's existing project, navigation, selection, and draft state have been created:

```ts
const agentWorkspace = useAgentWorkspace({
  tab: () => activeNav.value,
  mode: () => activeMode.value,
  projectSnapshot: () => projectSnapshot.value,
  selection: () => ({
    segmentIds: selectedSegmentIds.value,
    alignmentIds: selectedAlignmentIds.value,
  }),
  searchState: () => ({
    query: searchQuery.value,
    regex: searchRegex.value,
    caseSensitive: searchCaseSensitive.value,
    languageId: searchLanguageId.value,
  }),
  selectionSharingEnabled: () => settings.value.device.agent.shareSelection,
  canLeaveDraft: () => !hasDirtyDraft.value && !isSavingDraft.value,
  navigate: async (tab) => { await setNav(tab); },
  reveal: async ({ segmentId, alignmentId }) => { /* use ParallelWorkspace stable-ID APIs */ },
  onProjectSnapshot: async (identity) => { await refreshWorkspaceFor(identity.project.project_id, identity.project.current_revision_id); },
  onSearchState: ({ spec, results }) => { /* update the existing search refs */ },
});

onMounted(() => { void agentWorkspace.start(); });
onBeforeUnmount(() => agentWorkspace.dispose());
```

`tab`, `mode`, `projectSnapshot`, `selection`, `searchState`, and `selectionSharingEnabled` may be `Ref`, `ComputedRef`, or a getter. `canLeaveDraft` is synchronous and must return a boolean. The `selection` hook keeps alignment and segment selections distinct. Do not pass text or credentials through these hooks.

`AgentProjection.project` is a text-free `ProjectIdentity` (`{ project: Project }`), not a canonical snapshot. The native renderer queries `WorkspaceProject` only when its identity/revision changes and obtains bodies through bounded `load_parallel_slice` queries.

For a projection containing both `project` and search state, the composable awaits a successful `onProjectSnapshot(project)` first. It sends `onSearchState({ spec, results })` only while the same lifecycle epoch remains current. Search owners that reject results whose revision differs from their current snapshot can therefore validate the matching projected revision.

`start()` binds a native session for an actual project and starts the event stream. `setProjectSnapshot()` must be called immediately when the project is opened, closed, or replaced; it invalidates the old binding and clears the prior corpus selection before publishing the new context. Browser preview exposes `available === false` and does not send fixture IDs to Rust. Call `clearSelectionContext()` when the user explicitly deselects corpus content; do not call it merely because the assistant takes focus.

In Tauri with no open project, `start()` still publishes app-only tab, mode, focus, and time context with `project_id`, `revision_id`, `binding_id`, `segment_ids`, `alignment_ids`, selected text, and text range all null or empty. It does not bind or transmit demonstration fixture IDs. Opening a real project then creates the binding and publishes project context.

`AgentProjection.pending_ui_actions` is replayed after every accepted projection, including a gap resnapshot. Each item has `{ operation_id, request_id, binding_id, project_id, revision_id, kind, payload, status }`. The source `binding_id` may belong to an external MCP client and is never compared with the native UI binding. The renderer instead validates the action's project/revision against its current project and lifecycle. `ui.ack` sends `{ operation_id, request_id, status: "ui_applied" }` through the native UI binding only after a successful navigation or reveal; callbacks that fail, lose their project, or lose their lifecycle do not acknowledge success. With no open project, only explicit `project` and `settings` navigation actions carrying null project/revision may run and ACK with a null binding.

## Context requirements for visual owners

The document root may already expose `data-segment-id` on segment cards. Any additional segment renderer must retain that stable ID and put `data-segment-content` on the canonical text child. Call `captureDomSelection()` on `selectionchange`, focus, and navigation-related user events. It captures text only when the browser selection is wholly inside one canonical content child; offsets are JavaScript UTF-16 code-unit offsets relative to that child, never the row number or controls. A selection spanning multiple cards sends stable segment IDs but no text range. Editing textareas are deliberately omitted until their draft can be proved identical to canonical content. Focusing the assistant must not clear the document selection.

`ParallelWorkspace` emits `selection-context` with `{ segmentIds: string[], alignmentIds: string[] }` whenever an operation selection changes. It includes source/target Segment multi-selection, selected Alignment multi-selection, and the explicit Order Segment; it intentionally excludes the navigation-only `selectedAlignmentId`. Main should retain this payload in a ref and pass it to `useAgentWorkspace({ selection: () => ref.value })`.

The existing search component is `SearchReplaceWorkspace.vue`. Its source and target result-body spans carry `data-segment-id` from `SearchResult.sourceId`/`targetId` and `data-segment-content`; result labels, IDs, match labels, and alignment labels carry neither attribute. This makes selection offsets relative to text content only.

For Kernel-authoritative search and replace, pass `:authoritative-results="true"`, `:native-preview="nativePreview"`, `:preview-loading="previewLoading"`, and `:preview-error="previewError"`. `authoritativeResults` defaults to `false`; when true the component displays `results` unchanged and only uses its JavaScript matcher for best-effort highlighting. `nativePreview` has the exported immutable `ReplacePreview` shape `{ options, replacement, resultIds, source, target }`; both source and target entries are shown. `replace-preview` has no payload and asks the owner to freeze the current native request/revision and asynchronously set `nativePreview`. Clear `nativePreview` when query or replacement changes. `apply-replace` always receives the preview currently displayed in the dialog; in authoritative mode it stays disabled until that native snapshot is present and no preview error/loading state exists. The dialog tells the reviewer that approval applies to the original request snapshot.

Never mark password, credential, or settings controls as context. The composable filters `input[type=password]`, autocomplete credential fields, and controls inside `[data-agent-private]`, `[data-agent-context="private"]`, or `[data-agent-context="exclude"]`. Mark the global assistant panel and settings root with `data-agent-context="exclude"`; their focus must retain, rather than clear, the last meaningful corpus selection.

## Exports

- `agentClient.call<T>(method, params?, bindingId?)`
- `agentClient.projection()`
- `agentClient.subscribe(listener)`; listener receives `AgentClientUpdate` (`projection`, `event`, or stream `error`), and subscription exposes `dispose()`.
- `useAgentWorkspace(options)` returns reactive `available`, `starting`, `error`, `binding`, `context`, `start`, `dispose`, `publishContext`, `captureDomSelection`, `setProjectSnapshot`, `recordOutboundMessage`, and the Pinia `store`.
- `useAgentWorkspaceStore()` exposes activity and proposals plus project-keyed `chat`, `annotationScope`, `drafts`, and immutable context attached to each outgoing chat message.

Navigation and reveal events are acknowledged through `ui.ack` only after the supplied callback completes. The navigation callback is draft-gated first.
