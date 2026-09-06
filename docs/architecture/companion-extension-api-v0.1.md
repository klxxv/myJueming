# Companion and extension frontend API (v0.1)

Status: main integration handoff. These APIs are renderer-local presentation state and declarative capability metadata. They do not write canonical project data, load a plugin runtime, or wait for UI operations.

## Dependencies

No package, root manifest, or lockfile addition is needed. The companion Web Animations implementation uses the browser `Element.animate()` API only when it is enabled. Its module is dynamically imported only after `animated` mode becomes eligible, so static startup does not load it. It does not import Spine, Pixi, or a model provider.

The `domain/companion` barrel intentionally exports the controller and types only. An explicit backend user may import `domain/companion/web-animations-renderer`, but visual code should use the controller factory so the backend remains lazy.

## Companion controller

```ts
import { createCompanionController } from "./domain/companion";

const companion = createCompanionController({
  onEvent: (event) => { /* optional analytics or visible guidance only */ },
});

onMounted(() => {
  companion.mount(companionRoot.value!);
  companion.update({
    mode: effectiveMotionMode.value === "standard" ? "animated" : "static",
    activity: "idle",
    dirtyEditor: hasDirtyDraft.value || isSavingDraft.value,
  });
});

watch([effectiveMotionMode, hasDirtyDraft, isSavingDraft], () => {
  companion.update({
    mode: effectiveMotionMode.value === "standard" ? "animated" : "static",
    activity: agentActivity.value,
    dirtyEditor: hasDirtyDraft.value || isSavingDraft.value,
  });
});

onBeforeUnmount(() => companion.dispose());
```

`mount(root: HTMLElement)` searches the existing static actor elements by `data-companion-actor="cat" | "dog" | "plant" | "butterfly"`. `update(input: CompanionUpdate)` and `pet(actor)` are synchronous; project, proposal, navigation, and approval operations must never await animation. `setMode`, `setSystemReducedMotion`, `setWindowFocused`, and `setDirtyEditor` are convenience methods with the same immediate effect.

Modes are `animated`, `quiet`, `static`, and `hidden`. Quiet/static/hidden modes never instantiate the animation backend. When mode, activity, dirty-editor state, visibility, focus, or system reduced-motion changes, existing animations are synchronously cancelled. A future Spine/Pixi renderer can implement the same `CompanionRenderer` interface and be passed as `rendererFactory`.

The only activity values are `idle`, `running`, `awaiting_approval`, `succeeded`, `failed`, and `cancelled`. Manual actor interaction emits the separate `manual_pet` event.

## Declarative extension capabilities

```ts
import { createExtensionCapabilityRegistry } from "./domain/extensions";

const extensions = createExtensionCapabilityRegistry();
const generation = extensions.beginGeneration("pipeline-ui");
generation.register({
  navigation: [{ id: "pipeline", label: "Pipeline", location: "primary", order: 40 }],
  context: [{ id: "pipeline-selection", label: "Pipeline selection", scope: "project", fields: ["project_id", "revision_id"] }],
  operators: [{ id: "tokenize-zh", label: "Chinese tokenize", inputSchema: "pipeline.method.v1" }],
  tools: [{ id: "pipeline.inspect", label: "Inspect derived artifact", access: "read" }],
});

// A reload starts a new generation and disposes all previous contributions.
const nextGeneration = extensions.beginGeneration("pipeline-ui");
generation.dispose();
nextGeneration.dispose();
```

`NavigationContribution`, `ContextProvider`, `OperatorProvider`, and `ToolProvider` are data-only descriptors. The registry intentionally has no `load`, `import`, callback execution, or executable tool field. `subscribe(listener)` observes snapshots and returns a disposer; callers must dispose it on unmount. Generation disposal removes entries and subscriptions do not survive a registry disposal.

## Agent event hook

`AgentEventKind` includes `pipeline_changed`. `useAgentWorkspace` accepts optional `onAppEvent(event: AppEvent)`, invoked for every received native/external application event without forcing a full project projection for `pipeline_changed` or `context_changed`. Main can reload Pipeline UI or companion guidance from that callback.
