import { t, type LocalizedMessage } from '../i18n';
import { formatError } from '../i18n/kernel-messages';
import { computed, onBeforeUnmount, ref, type Ref } from "vue";
import type { WorkspaceMode } from "../domain/kernel-client";

export type EditSessionStatus = "clean" | "dirty" | "saving" | "error";

export interface EditSession {
  segmentId: string;
  alignmentId: string;
  persistedText: string;
  draft: string;
  status: EditSessionStatus;
  error: unknown;
}

export interface PendingModeTransition {
  mode: WorkspaceMode;
}

interface ViewModeControllerOptions {
  autosaveDelayMs: Ref<number>;
  persist: (segmentId: string, text: string) => Promise<void>;
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
    options.onStatus?.(() => reason === "auto" ? t('autosaving') : t('savingEdit'));
    saveInFlight = (async () => {
      try {
        await options.persist(segmentId, text);
        const current = editSession.value;
        if (current?.segmentId === segmentId) {
          current.persistedText = text;
          current.status = current.draft === text ? "clean" : "dirty";
          current.error = null;
        }
        options.onStatus?.(() => t('savedLocally'));
        return true;
      } catch (error) {
        const current = editSession.value;
        if (current?.segmentId === segmentId) {
          current.status = "error";
          current.error = error;
        }
        options.onStatus?.(() => t('autosaveFailed', { p0: formatError(error) }));
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
    options.onStatus?.(() => t('draftDiscarded'));
  };

  const requestMode = (mode: WorkspaceMode): "applied" | "guarded" => {
    if (mode === activeMode.value) return "applied";
    if (activeMode.value === "edit" && editSession.value) {
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
    if (!saved) return false;
    editSession.value = null;
    pendingTransition.value = null;
    activeMode.value = pending.mode;
    return true;
  };

  const confirmPendingWithDiscard = () => {
    const pending = pendingTransition.value;
    if (!pending) return false;
    clearAutosave();
    editSession.value = null;
    pendingTransition.value = null;
    activeMode.value = pending.mode;
    options.onStatus?.(() => t('draftDiscarded'));
    return true;
  };

  const cancelPendingTransition = () => {
    pendingTransition.value = null;
  };

  const forceMode = (mode: WorkspaceMode) => {
    clearAutosave();
    editSession.value = null;
    pendingTransition.value = null;
    activeMode.value = mode;
  };

  onBeforeUnmount(clearAutosave);

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
  };
}
