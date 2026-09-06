export const companionActors = ["cat", "dog", "plant", "butterfly"] as const;
export type CompanionActor = typeof companionActors[number];

export type CompanionMode = "animated" | "quiet" | "static" | "hidden";
export type CompanionActivity = "idle" | "running" | "awaiting_approval" | "succeeded" | "failed" | "cancelled";

export interface CompanionUpdate {
  mode: CompanionMode;
  activity: CompanionActivity;
  dirtyEditor: boolean;
}

export interface CompanionRendererState {
  activity: CompanionActivity;
}

export interface CompanionRenderer {
  mount(root: HTMLElement): void;
  update(state: CompanionRendererState): void;
  cancel(): void;
  dispose(): void;
  pet?(actor: CompanionActor): void;
}

export type CompanionEvent =
  | { kind: CompanionActivity }
  | { kind: "manual_pet"; actor: CompanionActor };

export interface CompanionRuntime {
  systemReducedMotion(): boolean;
  windowFocused(): boolean;
  documentVisible(): boolean;
  listenSystemReducedMotion(listener: (reduced: boolean) => void): () => void;
  listenVisibility(listener: (visible: boolean) => void): () => void;
  listenWindowFocus(listener: (focused: boolean) => void): () => void;
}

export interface CompanionControllerOptions {
  rendererFactory?: () => CompanionRenderer | Promise<CompanionRenderer>;
  runtime?: CompanionRuntime;
  onEvent?: (event: CompanionEvent) => void;
}

export interface CompanionController {
  mount(root: HTMLElement): void;
  update(update: CompanionUpdate): void;
  setMode(mode: CompanionMode): void;
  setSystemReducedMotion(reduced: boolean): void;
  setWindowFocused(focused: boolean): void;
  setDirtyEditor(dirty: boolean): void;
  pet(actor: CompanionActor): void;
  dispose(): void;
}
