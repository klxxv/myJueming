import type {
  CompanionActor,
  CompanionController,
  CompanionControllerOptions,
  CompanionMode,
  CompanionRenderer,
  CompanionRuntime,
  CompanionUpdate,
} from "./types";

const lazyWebAnimationsRenderer = async (): Promise<CompanionRenderer> => {
  const module = await import("./web-animations-renderer");
  return module.createWebAnimationsCompanionRenderer();
};

const isPromise = <T>(value: T | Promise<T>): value is Promise<T> => typeof (value as Promise<T>).then === "function";

const defaultRuntime = (): CompanionRuntime => {
  const query = typeof window !== "undefined" && typeof window.matchMedia === "function"
    ? window.matchMedia("(prefers-reduced-motion: reduce)")
    : null;
  return {
    systemReducedMotion: () => query?.matches ?? false,
    windowFocused: () => typeof document !== "undefined" && document.hasFocus(),
    documentVisible: () => typeof document === "undefined" || document.visibilityState !== "hidden",
    listenSystemReducedMotion(listener) {
      if (!query) return () => undefined;
      const handler = (event: MediaQueryListEvent) => listener(event.matches);
      query.addEventListener("change", handler);
      return () => query.removeEventListener("change", handler);
    },
    listenVisibility(listener) {
      if (typeof document === "undefined") return () => undefined;
      const handler = () => listener(document.visibilityState !== "hidden");
      document.addEventListener("visibilitychange", handler);
      return () => document.removeEventListener("visibilitychange", handler);
    },
    listenWindowFocus(listener) {
      if (typeof window === "undefined") return () => undefined;
      const focus = () => listener(true);
      const blur = () => listener(false);
      window.addEventListener("focus", focus);
      window.addEventListener("blur", blur);
      return () => { window.removeEventListener("focus", focus); window.removeEventListener("blur", blur); };
    },
  };
};

const isActor = (value: string | undefined): value is CompanionActor => value === "cat" || value === "dog" || value === "plant" || value === "butterfly";

export const createCompanionController = (options: CompanionControllerOptions = {}): CompanionController => {
  const runtime = options.runtime ?? defaultRuntime();
  const rendererFactory = options.rendererFactory ?? lazyWebAnimationsRenderer;
  let root: HTMLElement | null = null;
  let renderer: CompanionRenderer | null = null;
  let unlistenRuntime: Array<() => void> = [];
  let removePetListener: (() => void) | null = null;
  let mode: CompanionMode = "static";
  let activity: CompanionUpdate["activity"] = "idle";
  let dirtyEditor = false;
  let reducedMotion = runtime.systemReducedMotion();
  let focused = runtime.windowFocused();
  let visible = runtime.documentVisible();
  let lastActivity: CompanionUpdate["activity"] | null = null;
  let rendererGeneration = 0;

  const canAnimate = () => mode === "animated" && !dirtyEditor && !reducedMotion && focused && visible;
  const stopRenderer = () => {
    renderer?.dispose();
    renderer = null;
  };
  const mountRenderer = (candidate: CompanionRenderer, expectedRoot: HTMLElement, generation: number) => {
    if (generation !== rendererGeneration || root !== expectedRoot || !canAnimate() || renderer) {
      candidate.dispose();
      return;
    }
    renderer = candidate;
    renderer.mount(expectedRoot);
    renderer.update({ activity });
  };
  const emitActivity = () => {
    if (lastActivity === activity) return;
    lastActivity = activity;
    options.onEvent?.({ kind: activity });
  };
  const sync = () => {
    const generation = ++rendererGeneration;
    if (!root || !canAnimate()) {
      stopRenderer();
      return;
    }
    if (renderer) {
      renderer.cancel();
      renderer.update({ activity });
      return;
    }
    const expectedRoot = root;
    const candidate = rendererFactory();
    if (isPromise(candidate)) {
      void candidate.then((next) => mountRenderer(next, expectedRoot, generation)).catch(() => undefined);
    } else {
      mountRenderer(candidate, expectedRoot, generation);
    }
  };
  const setRuntimeState = (next: { reducedMotion?: boolean; focused?: boolean; visible?: boolean }) => {
    if (next.reducedMotion !== undefined) reducedMotion = next.reducedMotion;
    if (next.focused !== undefined) focused = next.focused;
    if (next.visible !== undefined) visible = next.visible;
    sync();
  };
  const pet = (actor: CompanionActor) => {
    renderer?.pet?.(actor);
    options.onEvent?.({ kind: "manual_pet", actor });
  };
  const dispose = () => {
    rendererGeneration += 1;
    removePetListener?.();
    removePetListener = null;
    for (const unlisten of unlistenRuntime) unlisten();
    unlistenRuntime = [];
    stopRenderer();
    root = null;
    lastActivity = null;
  };
  const mount = (nextRoot: HTMLElement) => {
    if (root === nextRoot) return;
    dispose();
    root = nextRoot;
    const onClick = (event: MouseEvent) => {
      const target = event.target as Element | null;
      const actorTarget = target?.closest<HTMLElement>("[data-companion-actor]") ?? null;
      const actor = actorTarget?.dataset.companionActor;
      if (isActor(actor)) pet(actor);
    };
    root.addEventListener("click", onClick);
    removePetListener = () => root?.removeEventListener("click", onClick);
    unlistenRuntime = [
      runtime.listenSystemReducedMotion((next) => setRuntimeState({ reducedMotion: next })),
      runtime.listenVisibility((next) => setRuntimeState({ visible: next })),
      runtime.listenWindowFocus((next) => setRuntimeState({ focused: next })),
    ];
    emitActivity();
    sync();
  };

  return {
    mount,
    update(next) {
      const changed = mode !== next.mode || activity !== next.activity || dirtyEditor !== next.dirtyEditor;
      const activityChanged = activity !== next.activity;
      mode = next.mode;
      activity = next.activity;
      dirtyEditor = next.dirtyEditor;
      if (activityChanged) emitActivity();
      if (changed) sync();
    },
    setMode(next) { if (mode !== next) { mode = next; sync(); } },
    setSystemReducedMotion(next) { if (reducedMotion !== next) setRuntimeState({ reducedMotion: next }); },
    setWindowFocused(next) { if (focused !== next) setRuntimeState({ focused: next }); },
    setDirtyEditor(next) { if (dirtyEditor !== next) { dirtyEditor = next; sync(); } },
    pet,
    dispose,
  };
};
