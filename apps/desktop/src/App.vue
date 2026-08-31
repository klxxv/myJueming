<script setup lang="ts">
import { t, formatDate, formatNumber, uiLocale, type LocalizedMessage } from './i18n';
import { formatError, rawErrorMessage, revisionAction, revisionSummary } from './i18n/kernel-messages';
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { confirm, open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Check, ChevronDown, Download, Eye, FilePlus2, Folder, FolderOpen, History, Link2, Maximize2, MessageSquareText, Minimize2, Monitor, PanelLeft, Pencil, Redo2, Save, Search, Settings2, Star, Undo2, X } from "@lucide/vue";
import ParallelWorkspace from "./components/ParallelWorkspace.vue";
import AnnotationPanel, { type AnnotationDraft, type AnnotationFilter, type AnnotationItem } from "./components/AnnotationPanel.vue";
import HistoryWorkspace, { type HistoryDiff, type RevisionItem } from "./components/HistoryWorkspace.vue";
import SearchReplaceWorkspace, { type ReplacePreview as UiReplacePreview, type SearchQueryOptions, type SearchResult } from "./components/SearchReplaceWorkspace.vue";
import BookmarksWorkspace from "./components/BookmarksWorkspace.vue";
import NewProjectDialog from "./components/NewProjectDialog.vue";
import SettingsWorkspace from "./components/SettingsWorkspace.vue";
import { useAppSettings } from "./composables/useAppSettings";
import type { ReorderIntent } from "./composables/useOrderDragAndDrop";
import { useViewModeController } from "./composables/useViewModeController";
import { useStructureMutations } from "./composables/useStructureMutations";
import { alignments as fixtureAlignments, sourceSegments, targetSegments } from "./data/governmentFixture";
import { createKernelClient, snapshotToWorkspace, type AlignmentDto, type BookmarkPreviewDto, type ExportFormat, type LanguageSide, type ProjectSnapshot, type ProjectSummaryDto, type ReplacePreviewRequest, type RevisionComparison, type SegmentDto, type WorkspaceMode } from "./domain/kernel-client";
import brandIcon from "../../../assets/brand/jueming-aligner-icon-master-v2.png";
import "./styles.css";

type NavId = "project" | "parallel" | "search" | "bookmarks" | "history" | "settings";
type ModeMenuId = WorkspaceMode | "annotation";
type ParallelWorkspaceExposed = {
  clearSelection: () => void;
  focusSegment: (segmentId: string) => Promise<boolean>;
  revealSegments: (sourceIds: string[], targetIds: string[]) => Promise<void>;
  openFind: () => void;
  navigateFind: (direction: 1 | -1) => void;
};
type NewProjectDialogExposed = { open: () => Promise<void> };
const detectedMacOS = /Mac|iPhone|iPad|iPod/i.test(navigator.platform || navigator.userAgent);
const isTauriRuntime = "__TAURI_INTERNALS__" in window;
const kernelClient = createKernelClient();
const activeNav = ref<NavId>("parallel");
const selectedAlignmentId = ref("alignment-000004");
const busy = ref(false);
const statusMessage = ref<LocalizedMessage>(() => isTauriRuntime ? t('projectNone') : t('demoPreview'));
const displayedStatus = computed(() => typeof statusMessage.value === 'function' ? statusMessage.value() : statusMessage.value);
const exportOpen = ref(false);
const annotationOpen = ref(false);
const annotationWidth = ref(355);
const annotationResizing = ref(false);
const annotationContextSegmentId = ref<string | null>(null);
const sideNavCollapsed = ref(false);
const searchQuery = ref("");
const searchSide = ref<"both" | "source" | "target">("both");
const searchRegex = ref(false);
const searchCaseSensitive = ref(false);
const replacement = ref("");
const searchResults = ref<SearchResult[]>([]);
const annotationFilter = ref<AnnotationFilter>("all");
const selectedAnnotationId = ref<string | null>(null);
const selectedRevisionId = ref<string | null>(null);
const baseRevisionId = ref<string | null>(null);
const historyComparison = ref<RevisionComparison | null>(null);
const historyDiff = computed(() => historyComparison.value ? comparisonToDiff(historyComparison.value) : null);
const orderBaseline = ref<Record<LanguageSide, string[]>>({ source: [], target: [] });
const pendingNav = ref<NavId | null>(null);
const parallelWorkspaceRef = ref<ParallelWorkspaceExposed | null>(null);
const newProjectDialogRef = ref<NewProjectDialogExposed | null>(null);
const bookmarkPreviews = ref<BookmarkPreviewDto[]>([]);
const sourceRows = ref<SegmentDto[]>(sourceSegments.map((segment) => ({ ...segment })));
const targetRows = ref<SegmentDto[]>(targetSegments.map((segment) => ({ ...segment })));
const alignmentRows = ref<AlignmentDto[]>(fixtureAlignments.map((alignment) => ({ ...alignment })));
const projectSnapshot = ref<ProjectSnapshot | null>(null);
const workspaceWritable = computed(() => projectSnapshot.value !== null || !isTauriRuntime);
const projectSummary = ref<ProjectSummaryDto>({ project_id: "fixture-project", name: t('demoName'), source_label: "report_zh.txt", target_label: "report_en.txt", source_count: 8, target_count: 8, alignment_count: 8, source_unlinked_count: 0, target_unlinked_count: 0, revision_id: "0" });

const displayedProjectName = computed(() => projectSnapshot.value ? projectSummary.value.name : t('demoName'));
const modeItems = computed<Array<{ id: WorkspaceMode; label: string; hint: string; icon: typeof Eye }>>(() => [
  { id: "review", label: t('modeReview'), hint: t('modeReviewHint'), icon: Eye },
  { id: "edit", label: t('modeEdit'), hint: t('modeEditHint'), icon: Pencil },
  { id: "history", label: t('modeHistory'), hint: t('modeHistoryHint'), icon: History },
]);
const annotationModeItem = computed<{ id: "annotation"; label: string; hint: string; icon: typeof Eye }>(() => ({
  id: "annotation",
  label: t('modeAnnotation'),
  hint: t('modeAnnotationHint'),
  icon: MessageSquareText,
}));
const modeMenuItems = computed<Array<{ id: ModeMenuId; label: string; hint: string; icon: typeof Eye }>>(() => [
  ...modeItems.value,
  annotationModeItem.value,
]);
const navItems = computed<Array<{ id: NavId; label: string; hint?: string; icon: typeof Folder }>>(() => [
  { id: "project", label: t('navProject'), icon: Folder }, { id: "parallel", label: t('navParallel'), icon: PanelLeft },
  { id: "search", label: t('navSearch'), icon: Search }, { id: "bookmarks", label: t('navBookmarks'), icon: Star },
  { id: "history", label: t('navHistory'), icon: History }, { id: "settings", label: t('navSettings'), icon: Settings2 },
]);
const currentMode = computed(() => {
  if (annotationOpen.value) return annotationModeItem.value;
  if (activeNav.value === "search") return { id: activeMode.value, label: t('navSearch'), hint: t('modeSearchHint'), icon: Search };
  return modeItems.value.find((item) => item.id === activeMode.value) ?? modeItems.value[0];
});
const segmentLabels = computed(() => new Map([...sourceRows.value, ...targetRows.value].map((segment) => [segment.id, String(segment.order + 1).padStart(6, "0")])));
const alignmentLabels = computed(() => new Map(alignmentRows.value.map((alignment) => [alignment.id, segmentLabels.value.get(alignment.sourceIds[0] ?? alignment.targetIds[0] ?? "") ?? alignment.id.slice(0, 8)])));
const segmentLabel = (segmentId: string) => segmentLabels.value.get(segmentId) ?? segmentId.slice(0, 8);
const alignmentLabel = (alignmentId: string | null | undefined) => {
  const alignment = alignmentRows.value.find((candidate) => candidate.id === alignmentId);
  return alignment ? segmentLabel(alignment.sourceIds[0] ?? alignment.targetIds[0] ?? alignment.id) : "—";
};
const humanizeSummary = (summary: string) => summary.replace(/[0-9a-f]{8}-[0-9a-f-]{27,}/gi, (id) => segmentLabels.value.get(id) ?? alignmentLabels.value.get(id) ?? id.slice(0, 8));
const revisionItems = computed<RevisionItem[]>(() => [...(projectSnapshot.value?.revisions ?? [])].reverse().map((revision) => ({ id: revision.revision_id, label: `R${revision.revision_id}`, timestamp: formatDate(revision.created_at), action: revisionAction(revision.change_set.operation), summary: humanizeSummary(revisionSummary(revision.summary)), current: revision.revision_id === projectSummary.value.revision_id })));
const canUndo = computed(() => (projectSnapshot.value?.revisions.length ?? 0) > 1);
const canRedo = computed(() => { const revisions = projectSnapshot.value?.revisions ?? []; return revisions[revisions.length - 1]?.change_set.operation.startsWith("undo:") ?? false; });
const annotations = computed<AnnotationItem[]>(() => (projectSnapshot.value?.annotations ?? []).map((annotation, index) => ({ id: annotation.annotation_id, number: index + 1, status: annotation.status, title: annotation.title, body: annotation.body, createdAt: formatDate(annotation.updated_at, true), links: annotation.linked_segment_ids.map((segmentId) => { const segment = [...sourceRows.value, ...targetRows.value].find((candidate) => candidate.id === segmentId); return { side: segment?.side ?? "source", segmentId, label: segmentLabel(segmentId), text: segment?.text }; }) })));
const bookmarkedSegmentIds = computed(() => (projectSnapshot.value?.bookmarks ?? []).map((bookmark) => bookmark.segment_id));
const annotatedSegmentIds = computed(() => [...new Set((projectSnapshot.value?.annotations ?? []).flatMap((annotation) => annotation.linked_segment_ids))]);
const notify = (message: LocalizedMessage) => { statusMessage.value = message; };
const {
  theme,
  fontScale,
  uiScale,
  autoSaveDelayMs,
  cacheCleanupPolicy,
  cacheCleaning,
  lastCacheCleanupAt,
  shortcutProfile,
  trackpadOptimized,
  usesMacShortcuts,
  shortcutLabels,
  shortcutRows,
  initializeSettings,
  applyUiSettings,
  applyInteractionSettings,
  applyPersistenceSettings,
  clearProjectCache,
  maybeCleanupCache,
} = useAppSettings({
  detectedMacOS,
  clearCache: () => kernelClient.clearCache(),
  onStatus: notify,
});
const requireOpenProject = () => {
  if (projectSnapshot.value) return true;
  notify(() => t('demoReadOnly'));
  if (isTauriRuntime) activeNav.value = "project";
  return false;
};
const clearWorkspaceSelection = () => parallelWorkspaceRef.value?.clearSelection();
const {
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
} = useViewModeController({
  autosaveDelayMs: autoSaveDelayMs,
  persist: async (segmentId, text) => persistSegment(segmentId, text),
  onStatus: notify,
});
const dirty = computed(() => hasDirtyDraft.value || isSavingDraft.value);
let unlistenClose: (() => void) | null = null;
watch(uiLocale, () => {
  if (isTauriRuntime) void getCurrentWindow().setTitle(t("appName")).catch((error: unknown) => notify(() => formatError(error)));
}, { immediate: true });

const closeWindowSafely = async () => {
  const saved = await persistDraft(true, "close");
  if (!saved) return;
  if (projectSnapshot.value) {
    try { await kernelClient.flushProject(); } catch (error) { notify(() => t('closeSaveFailed', { p0: errorMessage(error) })); return; }
  }
  const currentWindow = getCurrentWindow();
  unlistenClose?.();
  unlistenClose = null;
  await currentWindow.close();
};

const windowAction = async (action: "minimize" | "maximize" | "close") => {
  if (!("__TAURI_INTERNALS__" in window)) return;
  const currentWindow = getCurrentWindow();
  if (action === "minimize") await currentWindow.minimize();
  else if (action === "maximize") await currentWindow.toggleMaximize();
  else await closeWindowSafely();
};
const errorMessage = formatError;
const closeAnnotationPanel = () => {
  annotationOpen.value = false;
  annotationContextSegmentId.value = null;
};
const applySnapshot = async (
  snapshot: ProjectSnapshot,
  resetView = false,
  selectFirstAlignmentOnMissing = true,
) => {
  const previousAlignmentId = selectedAlignmentId.value;
  const workspace = snapshotToWorkspace(snapshot);
  projectSnapshot.value = snapshot;
  sourceRows.value = workspace.sourceSegments;
  targetRows.value = workspace.targetSegments;
  alignmentRows.value = workspace.alignments;
  selectedAlignmentId.value = !resetView && workspace.alignments.some((alignment) => alignment.id === previousAlignmentId)
    ? previousAlignmentId
    : selectFirstAlignmentOnMissing ? (workspace.alignments[0]?.id ?? "") : "";
  projectSummary.value = await kernelClient.getProjectSummary();
  try { bookmarkPreviews.value = await kernelClient.listBookmarks(); } catch { bookmarkPreviews.value = []; }
  if (resetView) {
    orderBaseline.value = {
      source: workspace.sourceSegments.map((segment) => segment.id),
      target: workspace.targetSegments.map((segment) => segment.id),
    };
    activeNav.value = "parallel";
    forceMode("review");
    closeAnnotationPanel();
    selectedAnnotationId.value = null;
    resetSearch();
  }
};
onMounted(async () => {
  sideNavCollapsed.value = localStorage.getItem("jueming-nav-collapsed") === "true";
  const savedAnnotationWidthValue = localStorage.getItem("jueming-annotation-width");
  const savedAnnotationWidth = savedAnnotationWidthValue === null ? Number.NaN : Number(savedAnnotationWidthValue);
  if (Number.isFinite(savedAnnotationWidth)) annotationWidth.value = Math.min(560, Math.max(300, savedAnnotationWidth));
  initializeSettings();
  try {
    await applySnapshot(await kernelClient.getCurrentProject(), true);
  } catch {
    const recentProjectPath = isTauriRuntime ? localStorage.getItem("jueming-last-project-path") : null;
    if (recentProjectPath) {
      try {
        await applySnapshot(await kernelClient.openProject(recentProjectPath), true);
        notify(() => t('projectReopened', { p0: projectSummary.value.name }));
      } catch {
        localStorage.removeItem("jueming-last-project-path");
      }
    }
  }
  if (projectSnapshot.value) await maybeCleanupCache();
  else if (isTauriRuntime) {
    activeNav.value = "project";
    selectedAlignmentId.value = "";
    notify(() => t('projectOpenPrompt'));
  }
  if (isTauriRuntime) {
    unlistenClose = await getCurrentWindow().onCloseRequested((event) => { event.preventDefault(); void closeWindowSafely(); });
  }
});
const previousRevisionId = () => { const revisions = projectSnapshot.value?.revisions ?? []; return revisions[revisions.length - 2]?.revision_id ?? projectSummary.value.revision_id; };
const applyModeContext = (mode: WorkspaceMode) => {
  activeNav.value = mode === "history" ? "history" : "parallel";
  closeAnnotationPanel();
  exportOpen.value = false;
  if (mode === "history") void selectHistoryRevision(previousRevisionId());
};
const setMode = (mode: WorkspaceMode) => {
  pendingNav.value = null;
  if (requestMode(mode) === "applied") applyModeContext(mode);
};
const applyNavContext = (nav: NavId) => {
  activeNav.value = nav;
  closeAnnotationPanel();
  if (nav === "history") { forceMode("history"); void selectHistoryRevision(previousRevisionId()); }
  else if (activeMode.value === "history") forceMode("review");
};
const setNav = (nav: NavId) => {
  if (activeMode.value === "edit" && nav !== "parallel") {
    pendingNav.value = nav;
    if (requestMode(nav === "history" ? "history" : "review") === "guarded") return;
  }
  pendingNav.value = null;
  applyNavContext(nav);
};
const openAnnotationPanel = (segmentId?: string, alignmentId?: string | null) => {
  activeNav.value = "parallel";
  if (activeMode.value === "history") forceMode("review");
  if (alignmentId !== undefined) selectedAlignmentId.value = alignmentId ?? "";
  annotationContextSegmentId.value = segmentId ?? null;
  if (segmentId) {
    selectedAnnotationId.value = annotations.value.find((annotation) => annotation.links.some((link) => link.segmentId === segmentId))?.id ?? null;
  }
  annotationOpen.value = true;
  exportOpen.value = false;
};
const openNewProject = () => { void newProjectDialogRef.value?.open(); };
const openProject = async () => {
  const selected = await open({ directory: true, multiple: false, title: t('projectOpenDialog') });
  if (!selected) return;
  busy.value = true;
  try { await applySnapshot(await kernelClient.openProject(selected), true); localStorage.setItem("jueming-last-project-path", selected); notify(() => t('projectOpened', { p0: projectSummary.value.name })); }
  catch (error) { notify(() => t('projectOpenFailed', { p0: errorMessage(error) })); }
  finally { busy.value = false; }
};
const handleProjectCreated = async (snapshot: ProjectSnapshot, createdProjectPath: string) => {
  localStorage.setItem("jueming-last-project-path", createdProjectPath);
  try {
    await applySnapshot(snapshot, true);
    notify(() => t('projectCreated', { p0: projectSummary.value.name }));
  } catch (error) {
    notify(() => t('projectLoadFailed', { p0: errorMessage(error) }));
  }
};
const saveProject = async () => {
  if (!(await persistDraft(false, "manual"))) return;
  try { await kernelClient.flushProject(); notify(() => t('savedLocally')); }
  catch (error) { notify(() => t('saveFailedDetail', { p0: errorMessage(error) })); }
};
async function persistSegment(id: string, text: string) {
  const segment = [...sourceRows.value, ...targetRows.value].find((item) => item.id === id);
  if (!segment || segment.text === text) return;
  await kernelClient.updateSegment(id, text);
  await applySnapshot(await kernelClient.getCurrentProject());
  notify(() => t('segmentAutosaved'));
}
const requestSegmentEdit = (segmentId: string, alignmentId: string) => {
  const segment = [...sourceRows.value, ...targetRows.value].find((item) => item.id === segmentId);
  if (!segment) return;
  selectedAlignmentId.value = alignmentId;
  activeNav.value = "parallel";
  annotationOpen.value = false;
  enterEdit(segmentId, alignmentId, segment.text);
};
const commitSegmentEdit = async (exitAfterSave: boolean) => {
  const saved = await persistDraft(exitAfterSave, "manual");
  if (saved && exitAfterSave) { activeNav.value = "parallel"; annotationOpen.value = false; }
};
const escapeSegmentEdit = async () => {
  const saved = await persistDraft(true, "escape");
  if (saved) { activeNav.value = "parallel"; annotationOpen.value = false; }
};
const cancelSegmentEdit = () => { discardAndExit(); activeNav.value = "parallel"; annotationOpen.value = false; };
const moveSegment = async (side: LanguageSide, segmentId: string, direction: "up" | "down") => {
  if (!requireOpenProject()) return;
  const rows = side === "source" ? sourceRows : targetRows;
  const index = rows.value.findIndex((item) => item.id === segmentId);
  const targetIndex = direction === "up" ? index - 1 : index + 1;
  if (index < 0 || targetIndex < 0 || targetIndex >= rows.value.length) {
    notify(() => direction === "up" ? t('firstSegment') : t('lastSegment'));
    return;
  }
  const targetSegment = rows.value[targetIndex];
  if (!(await confirmCrossAlignmentReorder(side, {
    segmentId,
    targetId: targetSegment.id,
    edge: direction === "up" ? "before" : "after",
  }))) {
    notify(() => t('reorderCancelled'));
    return;
  }
  const previous = rows.value.map((segment) => ({ ...segment }));
  const next = [...rows.value];
  [next[index], next[targetIndex]] = [next[targetIndex], next[index]];
  rows.value = next.map((segment, order) => ({ ...segment, order }));
  try {
    const before = next[targetIndex - 1]?.id;
    const after = next[targetIndex + 1]?.id;
    await kernelClient.moveSegment(segmentId, before, after);
    await applySnapshot(await kernelClient.getCurrentProject());
    notify(() => t('segmentMoved', { p0: side === "source" ? t('sourceSide') : t('targetSide'), p1: direction === "up" ? t('moveUp') : t('moveDown') }));
  } catch (error) {
    rows.value = previous;
    notify(() => t('reorderFailed', { p0: errorMessage(error) }));
  }
};
const alignmentIdForSegment = (segmentId: string) => alignmentRows.value.find(
  (alignment) => [...alignment.sourceIds, ...alignment.targetIds].includes(segmentId),
)?.id ?? null;
const describeAlignmentForSegment = (segmentId: string) => {
  const alignmentId = alignmentIdForSegment(segmentId);
  return alignmentId ? `Alignment ${alignmentLabel(alignmentId)}` : t('unalignedArea');
};
const confirmCrossAlignmentReorder = async (side: LanguageSide, intent: ReorderIntent) => {
  const sourceAlignmentId = alignmentIdForSegment(intent.segmentId);
  const targetAlignmentId = alignmentIdForSegment(intent.targetId);
  if (sourceAlignmentId === targetAlignmentId) return true;
  if (import.meta.env.DEV) console.debug(`[jueming:dnd] cross-alignment-warning ${JSON.stringify({
    side,
    segmentId: intent.segmentId,
    targetId: intent.targetId,
    sourceAlignmentId,
    targetAlignmentId,
    edge: intent.edge,
  })}`);
  const message = t('crossAlignmentWarning', { p0: side === "source" ? t('sourceSide') : t('targetSide'), p1: segmentLabel(intent.segmentId), p2: describeAlignmentForSegment(intent.segmentId), p3: describeAlignmentForSegment(intent.targetId) });
  return isTauriRuntime
    ? confirm(message, { title: t('crossAlignmentTitle'), kind: "warning", okLabel: t("confirm"), cancelLabel: t("cancel") })
    : window.confirm(t('crossAlignmentConfirm', { p0: message }));
};
const reorderSegment = async (side: LanguageSide, orderedSegmentIds: string[], intent: ReorderIntent) => {
  if (!requireOpenProject()) return;
  if (!(await confirmCrossAlignmentReorder(side, intent))) {
    notify(() => t('reorderCancelled'));
    return;
  }
  busy.value = true;
  try { await applySnapshot(await kernelClient.reorderSegments(orderedSegmentIds)); clearWorkspaceSelection(); notify(() => t('dragSaved', { p0: side === "source" ? t('sourceSide') : t('targetSide') })); }
  catch (error) { notify(() => t('dragFailed', { p0: errorMessage(error) })); }
  finally { busy.value = false; }
};
const resetOrder = async () => {
  if (!requireOpenProject()) return;
  if (!orderBaseline.value.source.length && !orderBaseline.value.target.length) { notify(() => t('noOrderBaseline')); return; }
  busy.value = true;
  try {
    let snapshot: ProjectSnapshot | null = null;
    const currentOrders: Record<LanguageSide, string[]> = {
      source: sourceRows.value.map((segment) => segment.id),
      target: targetRows.value.map((segment) => segment.id),
    };
    for (const side of ["source", "target"] as const) {
      const baseline = orderBaseline.value[side];
      if (baseline.length && baseline.join("\u0000") !== currentOrders[side].join("\u0000")) {
        snapshot = await kernelClient.reorderSegments(baseline);
      }
    }
    if (snapshot) await applySnapshot(snapshot);
    clearWorkspaceSelection();
    notify(() => t('orderRestored'));
  } catch (error) {
    await applySnapshot(await kernelClient.getCurrentProject());
    notify(() => t('orderRestoreFailed', { p0: errorMessage(error) }));
  }
  finally { busy.value = false; }
};
const linkSegments = async (sourceSegmentIds: string[], targetSegmentIds: string[]) => {
  if (!requireOpenProject()) return;
  busy.value = true;
  try {
    let snapshot: ProjectSnapshot;
    try {
      snapshot = await kernelClient.linkSegments(sourceSegmentIds, targetSegmentIds, false);
    } catch (error) {
      const message = rawErrorMessage(error);
      if (!message.includes("confirm replacement before linking") && !message.includes("AlignmentSelectionConflict")) throw error;
      const approved = "__TAURI_INTERNALS__" in window
        ? await confirm(t('replaceAlignmentWarning'), { title: t('replaceAlignmentTitle'), kind: "warning", okLabel: t("confirm"), cancelLabel: t("cancel") })
        : window.confirm(t('replaceAlignmentConfirm'));
      if (!approved) return;
      snapshot = await kernelClient.linkSegments(sourceSegmentIds, targetSegmentIds, true);
    }
    await applySnapshot(snapshot);
    clearWorkspaceSelection();
    selectedAlignmentId.value = alignmentRows.value.find((alignment) => sourceSegmentIds.every((id) => alignment.sourceIds.includes(id)) && targetSegmentIds.every((id) => alignment.targetIds.includes(id)))?.id ?? selectedAlignmentId.value;
    notify(() => t('alignmentCreated', { p0: sourceSegmentIds.length, p1: targetSegmentIds.length }));
  } catch (error) { notify(() => t('linkFailed', { p0: errorMessage(error) })); }
  finally { busy.value = false; }
};
const unlinkAlignment = async (alignmentId: string) => {
  if (!requireOpenProject()) return;
  const unlinkedAlignment = alignmentRows.value.find((alignment) => alignment.id === alignmentId);
  busy.value = true;
  try {
    await applySnapshot(await kernelClient.unlinkAlignment(alignmentId), false, false);
    clearWorkspaceSelection();
    if (unlinkedAlignment) {
      await nextTick();
      await parallelWorkspaceRef.value?.revealSegments(
        unlinkedAlignment.sourceIds,
        unlinkedAlignment.targetIds,
      );
    }
    notify(() => t('alignmentRemoved'));
  }
  catch (error) { notify(() => t('unlinkFailed', { p0: errorMessage(error) })); }
  finally { busy.value = false; }
};
const {
  insertAlignmentGap,
  mergeSegmentContent,
  splitSegmentContent,
  groupAlignments,
  ungroupAlignment,
} = useStructureMutations({
  kernelClient,
  sourceRows,
  alignmentRows,
  selectedAlignmentId,
  busy,
  requireOpenProject,
  applySnapshot,
  clearWorkspaceSelection,
  notify,
  errorMessage,
});
const performUndo = async () => { busy.value = true; try { await applySnapshot(await kernelClient.undo()); notify(() => t('undoSaved')); } catch (error) { notify(() => t('undoFailed', { p0: errorMessage(error) })); } finally { busy.value = false; } };
const performRedo = async () => { busy.value = true; try { await applySnapshot(await kernelClient.redo()); notify(() => t('redoSaved')); } catch (error) { notify(() => t('redoFailed', { p0: errorMessage(error) })); } finally { busy.value = false; } };
const comparisonToDiff = (comparison: RevisionComparison): HistoryDiff => {
  const sourceDocumentId = projectSnapshot.value?.documents[0]?.document_id;
  const targetDocumentId = projectSnapshot.value?.documents[1]?.document_id;
  const sourceChange = comparison.segment_changes.find((change) => (change.before ?? change.after)?.document_id === sourceDocumentId);
  const targetChange = comparison.segment_changes.find((change) => (change.before ?? change.after)?.document_id === targetDocumentId);
  return {
    segmentId: (sourceChange ?? targetChange) ? segmentLabel((sourceChange ?? targetChange)!.segment_id) : t('structureChange'),
    sourceOld: sourceChange?.before?.content ?? "", sourceNew: sourceChange?.after?.content ?? "",
    targetOld: targetChange?.before?.content ?? "", targetNew: targetChange?.after?.content ?? "",
    deletedLines: comparison.segment_changes.filter((change) => change.before).length,
    addedLines: comparison.segment_changes.filter((change) => change.after).length,
    summary: t('diffSummary', { p0: comparison.segment_changes.length, p1: comparison.order_changes.length, p2: comparison.alignment_changes.length }),
  };
};
const compareHistory = async (fromRevisionId: string, toRevisionId: string) => {
  if (fromRevisionId === toRevisionId) return;
  busy.value = true;
  try { baseRevisionId.value = fromRevisionId; selectedRevisionId.value = toRevisionId; historyComparison.value = await kernelClient.compareRevision(fromRevisionId, toRevisionId); }
  catch (error) { notify(() => t('compareFailed', { p0: errorMessage(error) })); }
  finally { busy.value = false; }
};
const selectHistoryRevision = async (revisionId: string) => {
  const current = projectSummary.value.revision_id;
  const fallback = projectSnapshot.value?.revisions.find((revision) => revision.revision_id !== revisionId)?.revision_id;
  selectedRevisionId.value = revisionId;
  await compareHistory(revisionId === current ? (fallback ?? current) : revisionId, current);
};
const restoreHistory = async (revisionId: string) => {
  const approved = "__TAURI_INTERNALS__" in window ? await confirm(t('restoreWarning'), { title: t('restoreConfirm', { p0: revisionId }), kind: "warning", okLabel: t("confirm"), cancelLabel: t("cancel") }) : window.confirm(t('restoreConfirm', { p0: revisionId }));
  if (!approved) return;
  busy.value = true;
  try { await applySnapshot(await kernelClient.restoreRevision(revisionId)); notify(() => t('revisionRestored', { p0: revisionId })); await selectHistoryRevision(revisionId); }
  catch (error) { notify(() => t('restoreFailed', { p0: errorMessage(error) })); }
  finally { busy.value = false; }
};
const makeSearchRequest = (options: SearchQueryOptions) => {
  const snapshot = projectSnapshot.value;
  if (!snapshot) throw new Error(t('openProjectFirst'));
  return { project_id: snapshot.project.project_id, query: options.query, regex: options.regex, case_sensitive: options.caseSensitive, language_id: options.side === "source" ? snapshot.project.source_language : options.side === "target" ? snapshot.project.target_language : null, base_revision_id: snapshot.project.current_revision_id };
};
const runSearch = async (options: SearchQueryOptions) => {
  busy.value = true;
  try {
    const response = await kernelClient.searchSegments(makeSearchRequest(options));
    const sourceById = new Map(sourceRows.value.map((segment) => [segment.id, segment.text]));
    const targetById = new Map(targetRows.value.map((segment) => [segment.id, segment.text]));
    searchResults.value = response.hits.map((hit) => {
      const alignment = hit.alignment_id ? alignmentRows.value.find((candidate) => candidate.id === hit.alignment_id) : undefined;
      const sourceId = hit.language_id === projectSnapshot.value?.project.source_language ? hit.segment_id : alignment?.sourceIds[0] ?? null;
      const targetId = hit.language_id === projectSnapshot.value?.project.target_language ? hit.segment_id : alignment?.targetIds[0] ?? null;
      return { id: hit.segment_id, label: segmentLabel(hit.segment_id), sourceId, targetId, sourceText: sourceId ? sourceById.get(sourceId) ?? "" : "", targetText: targetId ? targetById.get(targetId) ?? "" : "", alignmentId: hit.alignment_id, alignmentLabel: alignmentLabel(hit.alignment_id) };
    });
    notify(() => t('searchFound', { p0: response.hits.length }));
  } catch (error) { searchResults.value = []; notify(() => t('searchFailed', { p0: errorMessage(error) })); }
  finally { busy.value = false; }
};
const currentReplaceRequest = (): ReplacePreviewRequest => ({ ...makeSearchRequest({ query: searchQuery.value, side: searchSide.value, regex: searchRegex.value, caseSensitive: searchCaseSensitive.value }), replacement: replacement.value });
const previewReplacement = async () => { try { const response = await kernelClient.previewReplace(currentReplaceRequest()); notify(() => t('replacePreviewCount', { p0: response.items.length })); } catch (error) { notify(() => t('replacePreviewFailed', { p0: errorMessage(error) })); } };
const applyReplacement = async (preview: UiReplacePreview) => { busy.value = true; try { await applySnapshot(await kernelClient.applyReplace(currentReplaceRequest(), preview.resultIds)); notify(() => t('replaceApplied', { p0: preview.resultIds.length })); await runSearch({ query: searchQuery.value, side: searchSide.value, regex: searchRegex.value, caseSensitive: searchCaseSensitive.value }); } catch (error) { notify(() => t('replaceFailed', { p0: errorMessage(error) })); } finally { busy.value = false; } };
const jumpToSegment = async (segmentId: string, alignmentId: string | null = null) => {
  const resolvedAlignmentId = alignmentId ?? alignmentRows.value.find((alignment) => [...alignment.sourceIds, ...alignment.targetIds].includes(segmentId))?.id ?? "";
  selectedAlignmentId.value = resolvedAlignmentId;
  activeNav.value = "parallel";
  activeMode.value = "review";
  annotationOpen.value = false;
  await nextTick();
  return parallelWorkspaceRef.value?.focusSegment(segmentId) ?? false;
};
const selectSearchResult = (result: SearchResult) => { void jumpToSegment(result.id, result.alignmentId ?? null); };
const toggleBookmark = async (segmentId: string, alignmentId: string | null) => {
  const existing = projectSnapshot.value?.bookmarks.find((bookmark) => bookmark.segment_id === segmentId);
  const segment = [...sourceRows.value, ...targetRows.value].find((candidate) => candidate.id === segmentId);
  const content = segment?.text.replace(/\s+/g, " ").trim() ?? t('currentSegment');
  try { await applySnapshot(existing ? await kernelClient.deleteBookmark(existing.bookmark_id) : await kernelClient.createBookmark(segmentId, alignmentId, t('bookmarkDefaultLabel', { p0: segmentLabel(segmentId), p1: content.slice(0, 48) }))); notify(() => existing ? t('bookmarkRemoved') : t('bookmarkAdded')); }
  catch (error) { notify(() => t('bookmarkFailed', { p0: errorMessage(error) })); }
};
const openBookmark = (segmentId: string, alignmentId: string | null) => { void jumpToSegment(segmentId, alignmentId); };
const openAnnotationLink = (segmentId: string) => { void jumpToSegment(segmentId); };
const removeBookmark = async (bookmarkId: string) => { try { await applySnapshot(await kernelClient.deleteBookmark(bookmarkId)); notify(() => t('bookmarkRemoved')); } catch (error) { notify(() => t('bookmarkRemoveFailed', { p0: errorMessage(error) })); } };
const resetSearch = () => { searchQuery.value = ""; replacement.value = ""; searchResults.value = []; };
const copyHistoryValue = async (value: string) => { try { await navigator.clipboard.writeText(value); notify(() => t('copied')); } catch { notify(() => t('clipboardUnavailable')); } };
const toggleSideNav = () => { sideNavCollapsed.value = !sideNavCollapsed.value; localStorage.setItem("jueming-nav-collapsed", String(sideNavCollapsed.value)); };
const annotationRequest = (draft: AnnotationDraft) => ({ title: draft.title, body: draft.body, status: draft.status, linked_segment_ids: draft.links.map((link) => link.segmentId), alignment_id: selectedAlignmentId.value || null });
const createAnnotation = async (draft: AnnotationDraft) => {
  const contextSegment = annotationContextSegmentId.value
    ? [...sourceRows.value, ...targetRows.value].find((segment) => segment.id === annotationContextSegmentId.value)
    : undefined;
  const alignment = alignmentRows.value.find((item) => item.id === selectedAlignmentId.value);
  const fallbackLinks = contextSegment
    ? [{ side: contextSegment.side, segmentId: contextSegment.id }]
    : [...(alignment?.sourceIds ?? []).map((segmentId) => ({ side: "source" as const, segmentId })), ...(alignment?.targetIds ?? []).map((segmentId) => ({ side: "target" as const, segmentId }))];
  const linked = draft.links.length ? draft : { ...draft, links: fallbackLinks };
  try { await applySnapshot(await kernelClient.createAnnotation(annotationRequest(linked))); notify(() => t('annotationCreated')); }
  catch (error) { notify(() => t('annotationCreateFailed', { p0: errorMessage(error) })); }
};
const editAnnotation = async (annotationId: string, draft: AnnotationDraft) => { try { await applySnapshot(await kernelClient.updateAnnotation(annotationId, annotationRequest(draft))); notify(() => t('annotationUpdated')); } catch (error) { notify(() => t('annotationUpdateFailed', { p0: errorMessage(error) })); } };
const deleteAnnotation = async (annotationId: string) => { const approved = "__TAURI_INTERNALS__" in window ? await confirm(t('annotationDeleteWarning'), { title: t('annotationDeleteConfirm'), kind: "warning", okLabel: t("confirm"), cancelLabel: t("cancel") }) : window.confirm(t('annotationDeleteConfirm')); if (!approved) return; try { await applySnapshot(await kernelClient.deleteAnnotation(annotationId)); notify(() => t('annotationDeleted')); } catch (error) { notify(() => t('annotationDeleteFailed', { p0: errorMessage(error) })); } };
const resolveAnnotation = async (annotationId: string) => { try { await applySnapshot(await kernelClient.resolveAnnotation(annotationId)); notify(() => t('annotationResolved')); } catch (error) { notify(() => t('annotationResolveFailed', { p0: errorMessage(error) })); } };
const exportProject = async (format: ExportFormat) => { exportOpen.value = false; const outputPath = await saveDialog({ title: t('exportDialog', { p0: format.toUpperCase() }), defaultPath: `${projectSummary.value.name}.${format}`, filters: [{ name: format.toUpperCase(), extensions: [format] }] }); if (!outputPath) return; busy.value = true; try { await kernelClient.exportProject(format, outputPath); notify(() => t('exported', { p0: outputPath })); } catch (error) { notify(() => t('exportFailed', { p0: errorMessage(error) })); } finally { busy.value = false; } };
const finishPendingTransition = (mode: WorkspaceMode | undefined, nav: NavId | null) => {
  pendingNav.value = null;
  if (nav) applyNavContext(nav);
  else if (mode) applyModeContext(mode);
};
const savePendingTransition = async () => {
  const mode = pendingTransition.value?.mode;
  const nav = pendingNav.value;
  if (await confirmPendingWithSave()) finishPendingTransition(mode, nav);
};
const discardPendingTransition = () => {
  const mode = pendingTransition.value?.mode;
  const nav = pendingNav.value;
  if (confirmPendingWithDiscard()) finishPendingTransition(mode, nav);
};
const stayInEdit = () => { pendingNav.value = null; cancelPendingTransition(); };
const annotationMinimumWidth = 300;
const annotationMaximumWidth = () => {
  const navWidth = sideNavCollapsed.value ? 68 : window.innerWidth <= 1280 ? 150 : 168;
  return Math.max(annotationMinimumWidth, Math.min(560, window.innerWidth - navWidth - 480));
};
const setAnnotationWidth = (width: number) => {
  annotationWidth.value = Math.min(annotationMaximumWidth(), Math.max(annotationMinimumWidth, width));
};
const resizeAnnotationPanel = (event: PointerEvent) => setAnnotationWidth(window.innerWidth - event.clientX);
const stopAnnotationResize = () => {
  if (!annotationResizing.value) return;
  annotationResizing.value = false;
  window.removeEventListener("pointermove", resizeAnnotationPanel);
  window.removeEventListener("pointerup", stopAnnotationResize);
  window.removeEventListener("pointercancel", stopAnnotationResize);
  localStorage.setItem("jueming-annotation-width", String(Math.round(annotationWidth.value)));
};
const startAnnotationResize = (event: PointerEvent) => {
  if (event.button !== 0) return;
  event.preventDefault();
  annotationResizing.value = true;
  window.addEventListener("pointermove", resizeAnnotationPanel);
  window.addEventListener("pointerup", stopAnnotationResize);
  window.addEventListener("pointercancel", stopAnnotationResize);
};
const resetAnnotationWidth = () => {
  setAnnotationWidth(355);
  localStorage.setItem("jueming-annotation-width", String(annotationWidth.value));
};
const resizeAnnotationWithKeyboard = (event: KeyboardEvent) => {
  if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
  event.preventDefault();
  if (event.key === "ArrowLeft") setAnnotationWidth(annotationWidth.value + 16);
  else if (event.key === "ArrowRight") setAnnotationWidth(annotationWidth.value - 16);
  else if (event.key === "Home") setAnnotationWidth(annotationMinimumWidth);
  else setAnnotationWidth(annotationMaximumWidth());
  localStorage.setItem("jueming-annotation-width", String(Math.round(annotationWidth.value)));
};
const modeSelect = (event: Event) => {
  const mode = (event.target as HTMLSelectElement).value as ModeMenuId;
  if (mode === "annotation") openAnnotationPanel();
  else setMode(mode);
};
const handleShortcut = (event: KeyboardEvent) => {
  if (event.isComposing || event.key === "Process") return;
  const key = event.key.toLowerCase();
  if (key === "escape" && activeMode.value === "edit" && editSession.value) { event.preventDefault(); void escapeSegmentEdit(); return; }
  const primaryModifier = usesMacShortcuts.value ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
  if (!primaryModifier || event.altKey) return;
  const target = event.target as HTMLElement | null;
  const editingText = target?.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target?.tagName ?? "");
  if (key === "n" && !event.shiftKey) { event.preventDefault(); openNewProject(); return; }
  if (key === "o" && !event.shiftKey) { event.preventDefault(); void openProject(); return; }
  if (key === "s" && !event.shiftKey) { event.preventDefault(); void saveProject(); return; }
  if (key === "f" && event.shiftKey) { event.preventDefault(); setNav("search"); return; }
  if (key === "f" && !event.shiftKey && activeMode.value === "review" && activeNav.value === "parallel") { event.preventDefault(); void parallelWorkspaceRef.value?.openFind(); return; }
  if (key === "g" && activeMode.value === "review" && activeNav.value === "parallel") { event.preventDefault(); void parallelWorkspaceRef.value?.navigateFind(event.shiftKey ? -1 : 1); return; }
  if (editingText) return;
  if (key === "z" && !event.shiftKey) { event.preventDefault(); void performUndo(); }
  else if ((usesMacShortcuts.value && key === "z" && event.shiftKey) || (!usesMacShortcuts.value && (key === "y" || (key === "z" && event.shiftKey)))) { event.preventDefault(); void performRedo(); }
};
onMounted(() => window.addEventListener("keydown", handleShortcut));
onBeforeUnmount(() => { window.removeEventListener("keydown", handleShortcut); stopAnnotationResize(); unlistenClose?.(); });
</script>

<template>
  <div class="app-shell" :class="{ 'app-shell--macos': detectedMacOS }">
    <header class="window-chrome" data-tauri-drag-region><div class="brand-lockup" data-tauri-drag-region><img :src="brandIcon" alt="" /><span class="brand-title" data-tauri-drag-region>{{ t('appName') }} <em v-if="uiLocale === 'zh'" data-tauri-drag-region>{{ t('appBrand') }}</em></span></div><div class="window-actions"><button type="button" :title="t('windowMinimize')" @click="windowAction('minimize')"><Minimize2 :size="16" /></button><button type="button" :title="t('windowMaximize')" @click="windowAction('maximize')"><Maximize2 :size="15" /></button><button type="button" :title="t('close')" @click="windowAction('close')"><X :size="18" /></button></div></header>
    <header class="app-toolbar"><div class="toolbar-left"><button class="toolbar-button" type="button" :title="t('newProjectShortcut', { p0: shortcutLabels.newProject })" @click="openNewProject"><FilePlus2 :size="19" />{{ t('new') }}</button><button class="toolbar-button" type="button" :title="t('openProjectShortcut', { p0: shortcutLabels.openProject })" @click="openProject"><FolderOpen :size="19" />{{ t('open') }}</button><button class="toolbar-button" type="button" :title="t('saveProjectShortcut', { p0: shortcutLabels.save })" @click="saveProject"><Save :size="19" />{{ t('save') }}</button><div class="toolbar-divider"></div><div class="toolbar-actions"><div class="toolbar-button--export-wrap"><button class="toolbar-button toolbar-button--export" type="button" :title="t('exportProject')" @click="exportOpen = !exportOpen"><Download :size="19" />{{ t('export') }}<ChevronDown :size="15" /></button><div v-if="exportOpen" class="export-menu"><button type="button" @click="exportProject('txt')">{{ t('exportTxt') }}</button><button type="button" @click="exportProject('json')">{{ t('exportJson') }}</button><button type="button" @click="exportProject('xml')">{{ t('exportXml') }}</button></div></div><div class="toolbar-divider"></div><button class="toolbar-button" type="button" :disabled="!canUndo || busy" :title="t('undoShortcut', { p0: shortcutLabels.undo })" @click="performUndo"><Undo2 :size="19" />{{ t('undo') }}</button><button class="toolbar-button" type="button" :disabled="!canRedo || busy" :title="t('redoShortcut', { p0: shortcutLabels.redo })" @click="performRedo"><Redo2 :size="19" />{{ t('redo') }}</button><div class="toolbar-divider"></div><button class="toolbar-button" type="button" @click="setNav('settings')"><Settings2 :size="19" />{{ t('navSettings') }}</button></div></div><div class="mode-control" :class="`mode-control--${annotationOpen ? 'annotation' : activeMode}`"><component :is="currentMode.icon" :size="17" /><span>{{ currentMode.label }} <small v-if="uiLocale === 'zh'">{{ currentMode.hint }}</small></span><ChevronDown :size="15" /><select :value="annotationOpen ? 'annotation' : activeMode" :aria-label="t('switchMode')" @change="modeSelect"><option v-for="item in modeMenuItems" :key="item.id" :value="item.id">{{ item.label }}{{ uiLocale === 'zh' ? ` ${item.hint}` : '' }}</option></select></div></header>
    <div class="content-grid" :class="{ 'content-grid--collapsed': sideNavCollapsed, 'content-grid--annotation': annotationOpen, 'content-grid--resizing': annotationResizing }" :style="{ '--annotation-width': `${annotationWidth}px` }">
      <nav class="side-nav" :class="{ 'side-nav--collapsed': sideNavCollapsed }"><button v-for="item in navItems" :key="item.id" class="nav-item" :class="{ 'nav-item--active': activeNav === item.id }" type="button" :title="item.label" @click="setNav(item.id)"><component :is="item.icon" :size="22" :stroke-width="activeNav === item.id ? 2.2 : 1.8" /><span class="nav-label">{{ item.label }}<small v-if="item.hint">{{ item.hint }}</small></span></button><div class="nav-collapse"><button class="nav-item" type="button" :title="sideNavCollapsed ? t('expandSidebar') : t('collapseSidebar')" @click="toggleSideNav"><ChevronDown :size="21" :style="{ transform: sideNavCollapsed ? 'rotate(-90deg)' : 'rotate(90deg)' }" /><span class="nav-label">{{ sideNavCollapsed ? t('expand') : t('collapse') }}</span></button></div></nav>
      <main class="main-stage">
        <ParallelWorkspace ref="parallelWorkspaceRef" v-if="activeNav === 'parallel'" :mode="activeMode" :source-language="projectSnapshot?.project.source_language" :target-language="projectSnapshot?.project.target_language" :source-segments="sourceRows" :target-segments="targetRows" :alignments="alignmentRows" :selected-alignment-id="selectedAlignmentId" :bookmarked-segment-ids="bookmarkedSegmentIds" :annotated-segment-ids="annotatedSegmentIds" :edit-session="editSession" :trackpad-optimized="trackpadOptimized" :writable="workspaceWritable" @select="selectedAlignmentId = $event" @request-edit="requestSegmentEdit" @edit-draft="updateDraft" @commit-edit="commitSegmentEdit" @cancel-edit="cancelSegmentEdit" @escape-edit="escapeSegmentEdit" @move="moveSegment" @reorder="reorderSegment" @insert-gap="insertAlignmentGap" @reset-order="resetOrder" @link="linkSegments" @unlink="unlinkAlignment" @merge-segments="mergeSegmentContent" @split-segment="splitSegmentContent" @group="groupAlignments" @ungroup="ungroupAlignment" @bookmark="toggleBookmark" @annotation="openAnnotationPanel" @status="notify" />
        <SearchReplaceWorkspace v-else-if="activeNav === 'search'" v-model:query="searchQuery" v-model:side="searchSide" v-model:regex="searchRegex" v-model:case-sensitive="searchCaseSensitive" v-model:replacement="replacement" :results="searchResults" :project-label="displayedProjectName" :loading="busy" @search="runSearch" @select-result="selectSearchResult" @replace-preview="previewReplacement" @apply-replace="applyReplacement" @reset="resetSearch" />
        <HistoryWorkspace v-else-if="activeNav === 'history'" :revisions="revisionItems" :diff="historyDiff" :base-revision-id="baseRevisionId" :selected-revision-id="selectedRevisionId" :current-revision-id="projectSummary.revision_id" :loading="busy" @select-revision="selectHistoryRevision" @compare="compareHistory" @restore="restoreHistory" @copy-value="copyHistoryValue" />
        <SettingsWorkspace v-else-if="activeNav === 'settings'" v-model:theme="theme" v-model:font-scale="fontScale" v-model:ui-scale="uiScale" v-model:shortcut-profile="shortcutProfile" v-model:trackpad-optimized="trackpadOptimized" v-model:auto-save-delay-ms="autoSaveDelayMs" v-model:cache-cleanup-policy="cacheCleanupPolicy" :uses-mac-shortcuts="usesMacShortcuts" :shortcut-rows="shortcutRows" :cache-cleaning="cacheCleaning" :last-cache-cleanup-at="lastCacheCleanupAt" @apply-ui="applyUiSettings" @apply-interaction="applyInteractionSettings" @apply-persistence="applyPersistenceSettings" @clear-cache="clearProjectCache(true)" />
        <BookmarksWorkspace v-else-if="activeNav === 'bookmarks'" :bookmarks="projectSnapshot?.bookmarks ?? []" :previews="bookmarkPreviews" :source-segments="sourceRows" :target-segments="targetRows" :alignments="alignmentRows" :segment-labels="segmentLabels" :alignment-labels="alignmentLabels" @open="openBookmark" @remove="removeBookmark" />
        <section v-else class="aux-view project-view"><Folder :size="28" /><h2>{{ t('navProject') }}</h2><p>{{ displayedProjectName }}</p><div class="project-summary-card"><span>{{ formatNumber(projectSummary.source_count) }}</span><small>{{ t('sourceSegments') }}</small><span>{{ formatNumber(projectSummary.target_count) }}</span><small>{{ t('targetSegments') }}</small><span>{{ formatNumber(projectSummary.alignment_count) }}</span><small>{{ t('alignment') }}</small></div><div class="project-actions"><button type="button" class="primary-button" @click="openProject">{{ t('openProject') }}</button><button type="button" class="secondary-button" @click="openNewProject">{{ t('newProject') }}</button></div></section>
      </main>
      <aside class="annotation-drawer" :class="{ 'annotation-drawer--open': annotationOpen }" :aria-hidden="!annotationOpen" :inert="!annotationOpen">
        <div class="annotation-resize-handle" role="separator" tabindex="0" :aria-label="t('annotationResize')" aria-orientation="vertical" :aria-valuemin="annotationMinimumWidth" :aria-valuemax="annotationMaximumWidth()" :aria-valuenow="Math.round(annotationWidth)" :title="t('resizeHint')" @pointerdown="startAnnotationResize" @dblclick="resetAnnotationWidth" @keydown="resizeAnnotationWithKeyboard"></div>
        <AnnotationPanel v-model:active-filter="annotationFilter" :annotations="annotations" :selected-id="selectedAnnotationId" :readonly="!workspaceWritable" @select="selectedAnnotationId = $event" @open-link="openAnnotationLink" @create="createAnnotation" @edit="editAnnotation" @delete="deleteAnnotation" @resolve="resolveAnnotation" @close="closeAnnotationPanel" />
      </aside>
    </div>
    <footer class="bottom-status"><div class="footer-left"><div class="footer-project"><strong>{{ t('projectLabel') }}</strong>{{ displayedProjectName }}</div><div class="footer-separator"></div><div class="footer-project">{{ t('filesLabel') }}<span>{{ projectSummary.source_label }}</span><Link2 :size="14" /><span>{{ projectSummary.target_label }}</span></div><div class="footer-separator"></div><div>{{ t('alignmentStatusLabel') }}<span class="footer-status-pill">{{ projectSummary.source_unlinked_count + projectSummary.target_unlinked_count === 0 ? '1:1' : t('needsReview') }}</span></div></div><div class="footer-right"><span class="save-state" :title="displayedStatus" role="status" :class="{ 'save-state--dirty': dirty }"><Monitor :size="14" />{{ busy ? t('processing') : isSavingDraft ? t('autosaving') : dirty ? t('autosavePending', { p0: autoSaveDelayMs / 1000 }) : displayedStatus }}</span><span>{{ t('processedLabel') }}{{ formatNumber(projectSummary.alignment_count) }} / {{ Math.max(projectSummary.source_count, projectSummary.target_count) }}</span><span>{{ t('progressLabel') }}</span><div class="progress-track"><span :style="{ width: `${Math.round(100 * projectSummary.alignment_count / Math.max(1, projectSummary.source_count, projectSummary.target_count))}%` }"></span></div><span>{{ formatNumber(projectSummary.alignment_count / Math.max(1, projectSummary.source_count, projectSummary.target_count), { style: 'percent' }) }}</span></div></footer>
    <div v-if="pendingTransition" class="modal-backdrop mode-guard-backdrop"><section class="mode-guard" role="dialog" aria-modal="true" aria-labelledby="mode-guard-title"><span class="eyebrow">{{ t('unsavedEyebrow') }}</span><h2 id="mode-guard-title">{{ t('unsavedTitle') }}</h2><p>{{ t('unsavedDescription') }}</p><div><button class="secondary-button" type="button" @click="stayInEdit">{{ t('keepEditing') }}</button><button class="secondary-button danger-button" type="button" @click="discardPendingTransition">{{ t('discardDraft') }}</button><button class="primary-button" type="button" @click="savePendingTransition"><Check :size="15" />{{ t('saveAndSwitch') }}</button></div></section></div>
    <NewProjectDialog ref="newProjectDialogRef" v-model:busy="busy" :kernel-client="kernelClient" @created="handleProjectCreated" @status="notify" />
  </div>
</template>

<style scoped>
.toolbar-button--export-wrap { position: relative; }.export-menu { position: absolute; z-index: 8; top: 41px; left: 0; width: 142px; padding: 6px; border: 1px solid var(--line); border-radius: 7px; background: #fff; box-shadow: 0 10px 25px rgb(35 55 38 / 14%); }.export-menu button { display: block; width: 100%; padding: 8px 10px; border: 0; border-radius: 4px; background: transparent; text-align: left; cursor: pointer; }.export-menu button:hover { color: var(--green-900); background: var(--green-050); }.mode-control small { margin-left: 2px; color: var(--green-700); font-size: 12px; font-weight: 500; }.mode-control--edit { border-color: #e6d19d; color: #916714; background: #fffaf0; }.mode-control--edit small { color: #af8321; }.mode-control--history { border-color: #c7d4df; color: #536b7a; background: #f8fbfd; }
.annotation-drawer { position: relative; z-index: 6; width: var(--annotation-width); height: 100%; min-width: 0; opacity: 0; visibility: hidden; transform: translateX(100%); transition: opacity 180ms ease, transform 240ms cubic-bezier(.2, .8, .2, 1), visibility 0s linear 240ms; }
.annotation-drawer--open { opacity: 1; visibility: visible; transform: translateX(0); transition-delay: 0s; }
.content-grid--resizing .annotation-drawer { transition: none; }
.annotation-resize-handle { position: absolute; z-index: 8; top: 16px; bottom: 16px; left: 0; width: 1px; background: #ccd9cd; cursor: col-resize; touch-action: none; }
.annotation-resize-handle::before { position: absolute; inset: 0 -5px; content: ""; }
.annotation-resize-handle:hover, .annotation-resize-handle:focus-visible, .content-grid--resizing .annotation-resize-handle { background: var(--green-700); box-shadow: 0 0 0 1px rgb(47 129 67 / 12%); }
.search-view { display: flex; flex-direction: column; height: 100%; min-height: 0; padding: 25px 28px; }.search-bar { display: flex; align-items: center; gap: 10px; }.search-bar > label { width: 42px; font-size: 14px; }.search-input-wrap { display: flex; align-items: center; width: min(570px, 48vw); height: 42px; padding: 0 12px; border: 1px solid #c9d1ca; border-radius: 7px 0 0 7px; color: var(--ink-500); }.search-input-wrap input { flex: 1; min-width: 0; padding: 0 9px; border: 0; outline: none; }.search-input-wrap button { padding: 3px; border: 0; background: transparent; color: var(--ink-500); cursor: pointer; }.primary-search, .primary-button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; height: 42px; padding: 0 22px; border: 1px solid var(--green-900); border-radius: 0 6px 6px 0; color: #fff; background: var(--green-900); cursor: pointer; }.filter-button { height: 42px; margin-left: -10px; padding: 0 12px; border: 1px solid #c9d1ca; border-radius: 0 7px 7px 0; background: #fff; cursor: pointer; }.check-label { display: inline-flex; align-items: center; gap: 6px; margin-left: 22px; color: var(--ink-700); font-size: 13px; }.current-project { display: inline-flex; align-items: center; gap: 4px; margin-left: auto; color: var(--green-900); font-size: 13px; }.search-summary { padding: 15px 5px 12px; color: var(--ink-700); font-size: 13px; }.search-summary strong { color: var(--green-900); }.search-summary span { color: var(--ink-500); }.search-table { min-height: 0; overflow: auto; border: 1px solid var(--line); border-radius: 7px; }.search-table-head, .search-row { display: grid; grid-template-columns: 85px 1.2fr 95px 1.45fr 90px; align-items: center; gap: 14px; padding: 0 17px; }.search-table-head { height: 43px; color: var(--ink-700); background: #f8faf8; font-size: 12px; }.search-row { width: 100%; min-height: 57px; border: 0; border-top: 1px solid var(--line); background: #fff; color: var(--ink-900); font-size: 13px; text-align: left; cursor: pointer; }.search-row:hover { background: #f5fbf3; }.search-row > span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }.search-row mark { width: max-content; padding: 2px 7px; color: #72591b; background: #fff1c9; font-weight: 660; }
.aux-view { display: flex; flex-direction: column; align-items: flex-start; justify-content: center; gap: 13px; max-width: 540px; height: 100%; margin: auto; padding: 32px; color: var(--green-900); }.aux-view h2 { margin: 0; color: var(--ink-900); font-size: 22px; }.aux-view p { margin: 0; color: var(--ink-700); line-height: 1.6; }.secondary-button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; height: 38px; padding: 0 17px; border: 1px solid #bdcabf; border-radius: 6px; color: var(--ink-700); background: #fff; cursor: pointer; }.aux-view .primary-button { border-radius: 6px; }.project-summary-card { display: grid; grid-template-columns: auto auto; gap: 3px 14px; margin: 4px 0; padding: 14px 18px; border: 1px solid var(--line); border-radius: 8px; color: var(--ink-900); }.project-summary-card span { font-size: 23px; color: var(--green-900); }.project-summary-card small { color: var(--ink-500); font-size: 11px; }
.bookmark-list { display: grid; gap: 8px; width: min(560px, 72vw); max-height: 420px; overflow: auto; }.bookmark-list article { display: grid; grid-template-columns: 1fr 36px; align-items: center; border: 1px solid var(--line); border-radius: 7px; background: var(--paper); }.bookmark-list button { display: flex; align-items: center; gap: 11px; min-width: 0; padding: 11px 13px; border: 0; color: var(--green-900); background: transparent; text-align: left; cursor: pointer; }.bookmark-list button:last-child { justify-content: center; padding-inline: 8px; color: var(--ink-500); }.bookmark-list span { display: grid; gap: 3px; }.bookmark-list small { color: var(--ink-500); font-family: ui-monospace, Consolas, monospace; }.bookmark-list em { overflow: hidden; color: var(--ink-700); font-size: 12px; font-style: normal; line-height: 1.4; text-overflow: ellipsis; white-space: nowrap; }.empty-copy { padding: 20px; border: 1px dashed #c8d3c9; border-radius: 7px; }.project-actions { display: flex; align-items: center; gap: 9px; }.project-actions > button { height: 42px; }
.modal-backdrop { position: fixed; z-index: 20; inset: 0; display: grid; place-items: center; background: rgb(31 42 34 / 22%); }.eyebrow { color: var(--green-700); font-size: 10px; font-weight: 700; letter-spacing: .1em; }
.annotation-panel { position: fixed; z-index: 10; top: 116px; right: 0; bottom: 58px; display: flex; flex-direction: column; width: 355px; overflow: auto; border-left: 1px solid #ccd9cd; background: #fbfdfb; box-shadow: -12px 0 30px rgb(32 51 35 / 10%); }.annotation-panel header { display: flex; align-items: flex-start; justify-content: space-between; padding: 19px 18px 12px; }.annotation-panel header button { padding: 5px; border: 0; background: transparent; color: var(--ink-500); cursor: pointer; }.annotation-panel h2 { margin: 4px 0 0; color: var(--ink-900); font-size: 16px; }.annotation-panel h2 small { display: inline-block; margin-left: 3px; padding: 2px 6px; border-radius: 10px; color: var(--green-900); background: var(--green-100); font-size: 11px; }.annotation-filter { display: flex; gap: 4px; padding: 0 13px 13px; border-bottom: 1px solid var(--line); }.annotation-filter button { padding: 7px 8px; border: 1px solid transparent; border-radius: 5px; color: var(--ink-500); background: transparent; font-size: 12px; cursor: pointer; }.annotation-filter button.active { border-color: #b3d5b7; color: var(--green-900); background: #f3faf1; }.annotation-card { display: grid; grid-template-columns: 25px 1fr; gap: 7px; margin: 13px 13px 0; padding: 13px 11px; border: 1px solid #d3dde5; border-radius: 8px; background: #fff; }.annotation-card--draft { border-color: #d3c4ec; }.annotation-number { display: grid; place-items: center; width: 22px; height: 22px; border-radius: 6px; color: #fff; background: #9864d5; font-size: 12px; }.annotation-number--green { background: #3c9a5a; }.annotation-state { display: flex; align-items: center; gap: 7px; color: var(--ink-900); font-size: 12px; font-weight: 650; }.annotation-state span { color: var(--ink-500); font-weight: 500; }.annotation-state time { margin-left: auto; color: var(--ink-500); font-size: 10px; font-weight: 400; }.annotation-card h3 { margin: 12px 0 7px; color: var(--ink-900); font-size: 14px; }.annotation-card p { margin: 0; color: var(--ink-700); font-size: 12px; line-height: 1.55; }.annotation-links { display: flex; flex-direction: column; gap: 5px; margin-top: 10px; padding: 8px; border-radius: 5px; background: #f8faf8; color: var(--ink-700); font-size: 11px; }.annotation-links b { float: right; color: var(--ink-500); font-weight: 500; }.annotation-card footer { display: flex; gap: 15px; margin-top: 10px; padding-top: 9px; border-top: 1px solid var(--line); }.annotation-card footer button, .resolve-button, .new-annotation { display: inline-flex; align-items: center; gap: 5px; border: 0; color: var(--ink-700); background: transparent; font-size: 12px; cursor: pointer; }.resolve-button { margin-top: 11px; padding: 6px 9px; border: 1px solid #b5dab9; border-radius: 5px; color: var(--green-900); background: #f2faf1; }.new-annotation { justify-content: center; margin: 13px; padding: 10px; border: 1px solid #abd1af; border-radius: 6px; color: var(--green-900); background: #f4fbf2; }
.mode-guard-backdrop { z-index: 30; }.mode-guard { width: min(460px, calc(100vw - 60px)); padding: 24px; border: 1px solid #d6c58f; border-radius: 10px; background: #fff; box-shadow: 0 22px 70px rgb(29 48 32 / 23%); }.mode-guard h2 { margin: 5px 0 9px; color: var(--ink-900); font-size: 20px; }.mode-guard p { margin: 0; color: var(--ink-700); font-size: 13px; line-height: 1.6; }.mode-guard > div { display: flex; justify-content: flex-end; gap: 9px; margin-top: 20px; }.mode-guard .primary-button { height: 38px; border-radius: 6px; }.danger-button { border-color: #dab7b7; color: #9b4e4e; }
</style>
