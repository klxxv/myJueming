import { computed, onBeforeUnmount, ref, type Ref } from "vue";
import { newCommandId, type CommandContext, type CommandScope, type WorkspaceMode } from "../domain/kernel-client";
import { t, type LocalizedMessage } from "../i18n";

export type EditSessionStatus = "clean" | "dirty" | "saving" | "error";

export interface EditSession {
  scope: CommandScope | null;
  pendingSave: { text: string; commandId: string } | null;
  segmentId: string;
  alignmentId: string;
  persistedText: string;
  draft: string;
  status: EditSessionStatus;
  error: string | null;
}

export interface PendingModeTransition {
  mode: WorkspaceMode;
}

interface ViewModeControllerOptions {
  autosaveDelayMs: Ref<number>;
  scope?: () => CommandScope | null;
  persist: (segmentId: string, text: string, context?: CommandContext) => Promise<CommandScope | void>;
  onStatus?: (message: LocalizedMessage) => void;
}

/**
 * Owns the Review/Edit/Order/History transition contract. Draft state lives
 * here rather than in the virtualized row component so mode changes, window
 * close, and keyboard commands can all observe the same state.
 */
export function useViewModeController(options: ViewModeControllerOptions) {
  const activeMode = ref<WorkspaceMode>("review");
  const editSession = ref<EditSession | null>(null);
  const pendingTransition = ref<PendingModeTransition | null>(null);
  let pendingLeave: ((allowed: boolean) => void) | null = null;
  const resolveLeave = (allowed: boolean) => { const resolve = pendingLeave; pendingLeave = null; resolve?.(allowed); };
  let autosaveTimer: ReturnType<typeof setTimeout> | null = null;
  let saveInFlight: Promise<boolean> | null = null;

  const hasDirtyDraft = computed(() => editSession.value?.status === "dirty" || editSession.value?.status === "error");
  const isSavingDraft = computed(() => editSession.value?.status === "saving");

  const clearAutosave = () => {
    if (autosaveTimer !== null) clearTimeout(autosaveTimer);
    autosaveTimer = null;
  };

  const scheduleAutosave = () => {
    clearAutosave();
    if (!editSession.value || editSession.value.status !== "dirty") return;
    autosaveTimer = setTimeout(() => {
      autosaveTimer = null;
      void persistDraft(false, "auto");
    }, Math.max(250, options.autosaveDelayMs.value));
  };

  const enterEdit = (segmentId: string, alignmentId: string, text: string) => {
    clearAutosave();
    activeMode.value = "edit";
    pendingTransition.value = null;
    editSession.value = {
      scope: options.scope?.() ?? null,
      pendingSave: null,
      segmentId,
      alignmentId,
      persistedText: text,
      draft: text,
      status: "clean",
      error: null,
    };
  };

  const updateDraft = (draft: string) => {
    const session = editSession.value;
    if (!session) return;
    session.draft = draft;
    session.error = null;
    if (session.status !== "saving") {
      session.status = draft === session.persistedText ? "clean" : "dirty";
    }
    if (session.status === "dirty") scheduleAutosave();
    else clearAutosave();
  };

  async function persistDraft(exitAfterSave: boolean, reason: "auto" | "manual" | "escape" | "close" = "manual"): Promise<boolean> {
    clearAutosave();
    if (saveInFlight) await saveInFlight;
    const session = editSession.value;
    if (!session) {
      if (exitAfterSave) activeMode.value = "review";
      return true;
    }
    if (session.draft === session.persistedText) {
      session.status = "clean";
      if (exitAfterSave) {
        editSession.value = null;
        activeMode.value = "review";
      }
      return true;
    }

    const segmentId = session.segmentId;
    const text = session.draft.trim() ? session.draft : session.persistedText;
    session.status = "saving";
    session.error = null;
    const automaticSave = reason === "auto";
    options.onStatus?.(() => t(automaticSave ? "opAutosaveSaving" : "opEditSaving"));
    saveInFlight = (async () => {
      try {
        if (session.pendingSave?.text !== text) session.pendingSave = { text, commandId: newCommandId() };
        const scope = await options.persist(segmentId, text, session.scope ? { ...session.scope, command_id: session.pendingSave.commandId } : undefined);
        const current = editSession.value;
        if (current?.segmentId === segmentId) {
          current.persistedText = text;
          current.pendingSave = null;
          if (scope) current.scope = scope;
          current.status = current.draft === text ? "clean" : "dirty";
          current.error = null;
        }
        options.onStatus?.(() => t("opSavedLocally"));
        return true;
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        const current = editSession.value;
        if (current?.segmentId === segmentId) {
          current.status = "error";
          current.error = message;
        }
        options.onStatus?.(() => t("opAutosaveFailed", { error: message }));
        return false;
      } finally {
        saveInFlight = null;
      }
    })();

    const saved = await saveInFlight;
    if (!saved) return false;
    const current = editSession.value;
    if (current?.segmentId === segmentId && current.status === "dirty") scheduleAutosave();
    if (exitAfterSave && current?.segmentId === segmentId && current.status === "clean") {
      editSession.value = null;
      activeMode.value = "review";
    }
    return true;
  }

  const discardAndExit = () => {
    clearAutosave();
    editSession.value = null;
    pendingTransition.value = null;
    activeMode.value = "review";
    options.onStatus?.(() => t("opDiscardedEdit"));
  };

  const requestMode = (mode: WorkspaceMode): "applied" | "guarded" => {
    if (mode === activeMode.value && (mode === "edit" || !editSession.value)) return "applied";
    if (editSession.value) {
      if (hasDirtyDraft.value || isSavingDraft.value) {
        pendingTransition.value = { mode };
        return "guarded";
      }
      editSession.value = null;
    }
    clearAutosave();
    activeMode.value = mode;
    return "applied";
  };

  const confirmPendingWithSave = async () => {
    const pending = pendingTransition.value;
    if (!pending) return false;
    const saved = await persistDraft(false, "manual");
    if (!saved || hasDirtyDraft.value || isSavingDraft.value) return false;
    editSession.value = null;
    pendingTransition.value = null;
    activeMode.value = pending.mode;
    resolveLeave(true);
    return true;
  };

  const confirmPendingWithDiscard = async () => {
    const pending = pendingTransition.value;
    if (!pending) return false;
    clearAutosave();
    if (saveInFlight) await saveInFlight;
    clearAutosave();
    editSession.value = null;
    pendingTransition.value = null;
    activeMode.value = pending.mode;
    options.onStatus?.(() => t("opDiscardedEdit"));
    resolveLeave(true);
    return true;
  };

  const cancelPendingTransition = () => {
    resolveLeave(false);
    pendingTransition.value = null;
  };

  const guardProjectChange = (): Promise<boolean> => {
    if (pendingLeave) return Promise.resolve(false);
    if (requestMode("review") === "applied") return Promise.resolve(true);
    clearAutosave();
    return new Promise(resolve => { pendingLeave = resolve; });
  };

  const forceMode = (mode: WorkspaceMode) => {
    clearAutosave();
    editSession.value = null;
    pendingTransition.value = null;
    activeMode.value = mode;
  };

  onBeforeUnmount(() => { clearAutosave(); resolveLeave(false); });

  return {
    activeMode,
    editSession,
    pendingTransition,
    hasDirtyDraft,
    isSavingDraft,
    enterEdit,
    updateDraft,
    persistDraft,
    discardAndExit,
    requestMode,
    confirmPendingWithSave,
    confirmPendingWithDiscard,
    cancelPendingTransition,
    forceMode,
    guardProjectChange,
  };
}
