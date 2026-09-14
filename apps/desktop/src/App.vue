<script setup lang="ts">
import { computed, defineAsyncComponent, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { confirm, open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Check, ChevronDown, Download, Eye, FilePlus2, Folder, FolderOpen, HelpCircle, History, Link2, ListOrdered, MessageSquareText, Monitor, PanelLeft, Pencil, Redo2, Save, Search, Settings2, Sparkles, Star, Undo2, Workflow, X } from "@lucide/vue";
import ComparisonWorkspace from "./components/ComparisonWorkspace.vue";
import ParallelWorkspace, { type OperationSelectionContext } from "./components/ParallelWorkspace.vue";
import AnnotationPanel, { type AnnotationDraft, type AnnotationFilter, type AnnotationItem } from "./components/AnnotationPanel.vue";
import HistoryWorkspace, { type HistoryDiff, type RevisionItem } from "./components/HistoryWorkspace.vue";
import SearchReplaceWorkspace, { type ReplacePreview as UiReplacePreview, type SearchQueryOptions, type SearchResult } from "./components/SearchReplaceWorkspace.vue";
import BookmarksWorkspace from "./components/BookmarksWorkspace.vue";
import NewProjectDialog from "./components/NewProjectDialog.vue";
import SettingsWorkspace from "./components/SettingsWorkspace.vue";
import GlobalSidePanel from "./components/GlobalSidePanel.vue";
import AgentPanel from "./components/AgentPanel.vue";
import AgentConnectionSettings from "./components/AgentConnectionSettings.vue";
import AgentModelSettings from "./components/AgentModelSettings.vue";
import { agentRuntimeClient, type AgentRuntimeMessage, type AgentRuntimeStatus, type AgentRuntimeEvent } from "./domain/agent-runtime-client";
const loadPipelineWorkspace = () => import("./components/PipelineWorkspace.vue");
const PipelineWorkspace = defineAsyncComponent(loadPipelineWorkspace);
const ResearchWorkspace = defineAsyncComponent(() => import("./components/ResearchWorkspace.vue"));
import { researchClient } from "./domain/research-client";
import type { FeatureSnapshot } from "./domain/research-types";
import { pipelineClient } from "./domain/pipeline-client";
import type { PipelineMethodProposal } from "./domain/pipeline-types";
import CompanionHabitat from "./components/CompanionHabitat.vue";
import CompanionGarden from "./components/CompanionGarden.vue";
import gardenPawIcon from "./assets/companion/garden-paw-button-v1.png";
import GuidanceOverlay from "./components/GuidanceOverlay.vue";
import ParallelTutorial from "./components/ParallelTutorial.vue";
import type { CompanionActivity } from "./domain/companion";
import type { ReviewableProposal } from "./components/AgentActionBar.vue";
import { useAgentWorkspace } from "./composables/useAgentWorkspace";
import { agentClient } from "./domain/agent-client";
import type { SearchSpec } from "./domain/agent-types";
import type { SearchSegmentsResponse } from "./domain/kernel-client";
import { useAppSettings } from "./composables/useAppSettings";
import type { ReorderIntent } from "./composables/useOrderDragAndDrop";
import { useViewModeController } from "./composables/useViewModeController";
import { useWorkspaceText } from "./composables/useWorkspaceText";
import { useWorkspaceIndex } from "./composables/useWorkspaceIndex";
import { useTutorialWorkspace } from "./composables/useTutorialWorkspace";
import { formatDate, t, type LocalizedMessage } from "./i18n";
import { formatError, LocalizedError, rawErrorMessage, revisionAction, revisionSummary } from "./i18n/kernel-messages";
import { PARALLEL_TUTORIAL_GLOBAL_CONFIG, parallelTutorialSteps, type TutorialPreset } from "./domain/parallel-tutorial";
import type { CommandContext, CommandScope } from "./domain/kernel-client";
import { useStructureMutations } from "./composables/useStructureMutations";
import { alignments as fixtureAlignments, sourceSegments, targetSegments } from "./data/governmentFixture";
import { createKernelClient, type AlignmentDto, type BookmarkPreviewDto, type ExportFormat, type LanguageSide, type WorkspaceProject, type ProjectSummaryDto, type ReplacePreviewRequest, type RevisionComparison, type SegmentDto, type WorkspaceMode } from "./domain/kernel-client";
import "./styles.css";

type NavId = "project" | "parallel" | "search" | "bookmarks" | "history" | "pipeline" | "research" | "settings";
type WorkspaceDraftHandle = { hasDirtyDraft: () => boolean; saveDraft: () => Promise<boolean>; discardDraft: () => void };
type ModeMenuId = WorkspaceMode | "annotation";
type ParallelWorkspaceExposed = {
  clearSelection: () => void;
  focusSegment: (segmentId: string) => Promise<boolean>;
  revealSegments: (sourceIds: string[], targetIds: string[]) => Promise<void>;
  openFind: () => void;
  navigateFind: (direction: 1 | -1) => void;
  prepareTutorial: (preset: TutorialPreset) => Promise<void>;
};
type NewProjectDialogExposed = { open: () => Promise<void> };
const detectedMacOS = /Mac|iPhone|iPad|iPod/i.test(navigator.platform || navigator.userAgent);
const isTauriRuntime = "__TAURI_INTERNALS__" in window;
const tutorialActive = ref(false);
const tutorialGuideOpen = ref(false);
const tutorialStep = ref(0);
const autoStartTutorialOnThisLaunch = (() => {
  if (!PARALLEL_TUTORIAL_GLOBAL_CONFIG.autoStartOnFirstLaunch) return false;
  try { return localStorage.getItem(PARALLEL_TUTORIAL_GLOBAL_CONFIG.firstLaunchStorageKey) !== "1"; }
  catch { return true; }
})();
const rememberTutorialLaunch = () => {
  try { localStorage.setItem(PARALLEL_TUTORIAL_GLOBAL_CONFIG.firstLaunchStorageKey, "1"); }
  catch { /* Storage can be unavailable in hardened webviews; the tutorial still works. */ }
};
const commandScope = (): CommandScope | null => !tutorialActive.value && projectSnapshot.value ? { project_id: projectSnapshot.value.project.project_id, base_revision_id: projectSnapshot.value.project.current_revision_id } : null;
const kernelClient = createKernelClient(commandScope);
const workspaceText = useWorkspaceText(kernelClient);
const workspaceIndex = useWorkspaceIndex();
const activeNav = ref<NavId>("parallel");
const selectedAlignmentId = ref("alignment-000004");
const busy = ref(false);
const statusMessage = ref<LocalizedMessage>(() => isTauriRuntime ? t("projectNone") : t("demoPreview"));
const annotationOpen = ref(false);
const assistantOpen = ref(false);
const rightPanelTab = ref<"agent" | "annotations">("agent");
const rightPanelOpen = computed(() => annotationOpen.value || assistantOpen.value);
const agentBusy = ref(false);
const agentActionError = ref<Error | string | null>(null);
const runtimeStatus = ref<AgentRuntimeStatus | null>(null);
const runtimeMessages = ref<AgentRuntimeMessage[]>([]);
const runtimeSessions = new Map<string, string>();
const runtimeRuns = ref<Record<string, { runId: string; state: string }>>({});
const runtimeStarting = ref<string | null>(null);
let unlistenRuntime: (() => void) | null = null;
let unlistenRuntimeResync: (() => void) | null = null;
let runtimeHistoryGeneration = 0;
const runtimeRunning = computed(() => runtimeStarting.value === agentProjectKey.value || ["running", "awaiting_approval"].includes(runtimeRuns.value[agentProjectKey.value]?.state ?? ""));
const pipelineRefreshKey = ref(0);
const pipelineVisited = ref(false);
watch(activeNav, tab => { if (tab === "pipeline") pipelineVisited.value = true; });
const guidance = ref<{ selector: string; label: string; key: number } | null>(null);
const gardenVisible = computed(() => settings.value.device.pet.enabled && settings.value.device.pet.presentation !== "hidden");
const gardenStrolling = ref(false);
const companionActivity = computed<CompanionActivity>(() => agentProposals.value.length ? "awaiting_approval" : agentBusy.value || runtimeRunning.value ? "running" : "idle");
const gardenOpen = computed(() => gardenStrolling.value || companionActivity.value !== "idle");
const pipelineProposals = ref<PipelineMethodProposal[]>([]);
const pipelineWorkspaceRef = ref<(WorkspaceDraftHandle & { revealNode: (nodeId: string) => Promise<boolean>; reload: () => Promise<void> }) | null>(null);
const researchWorkspaceRef = ref<WorkspaceDraftHandle | null>(null);
const pipelineDirty = ref(false);
const researchDirty = ref(false);
const researchFeature = ref<FeatureSnapshot | null>(null);
const researchFeatureBusy = ref(false);
const researchVisited = ref(false);
const researchHistoryAvailable = ref(false);
let researchHistoryGeneration = 0;
let featureRequestGeneration = 0;
let featureNavigationIntent: { projectId: string | null; navigationGeneration: number } | null = null;
let navigationGeneration = 0;
const workspaceLeave = ref<{ label: () => string; handle: WorkspaceDraftHandle; resolve: (value: boolean) => void } | null>(null);
const workspaceLeaveBusy = ref(false);
const historyTransitionPending = ref(false);
watch(activeNav, tab => { navigationGeneration++; if (tab === "research") researchVisited.value = true; });
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
const nativeReplacePreview = ref<UiReplacePreview | null>(null);
const nativeReplaceRequest = ref<ReplacePreviewRequest | null>(null);
const replacePreviewLoading = ref(false);
const replacePreviewError = ref<string | null>(null);
const displayedReplacePreviewError = computed(() => replacePreviewError.value === null ? null : formatError(replacePreviewError.value));
let replacePreviewGeneration = 0;
const workspaceOperationSelection = ref<OperationSelectionContext>({ segmentIds: [], alignmentIds: [] });
const annotationFilter = ref<AnnotationFilter>("all");
const selectedAnnotationId = ref<string | null>(null);
const selectedRevisionId = ref<string | null>(null);
const baseRevisionId = ref<string | null>(null);
type HistoryDiffState = Omit<HistoryDiff, "summary"> & {
  summaryCounts: { segments: number; order: number; alignments: number };
};
const historyDiffState = ref<HistoryDiffState | null>(null);
const historyDiff = computed<HistoryDiff | null>(() => historyDiffState.value ? {
  ...historyDiffState.value,
  segmentId: historyDiffState.value.segmentId || t("structureChange"),
  summary: t("diffSummary", {
    p0: historyDiffState.value.summaryCounts.segments,
    p1: historyDiffState.value.summaryCounts.order,
    p2: historyDiffState.value.summaryCounts.alignments,
  }),
} : null);
const orderBaseline = ref<Record<LanguageSide, string[]>>({ source: [], target: [] });
const pendingNav = ref<NavId | null>(null);
const parallelWorkspaceRef = ref<ParallelWorkspaceExposed | null>(null);
const newProjectDialogRef = ref<NewProjectDialogExposed | null>(null);
const bookmarkPreviews = ref<BookmarkPreviewDto[]>([]);
const sourceRows = ref<SegmentDto[]>(sourceSegments.map((segment) => ({ ...segment })));
const targetRows = ref<SegmentDto[]>(targetSegments.map((segment) => ({ ...segment })));
const alignmentRows = ref<AlignmentDto[]>(fixtureAlignments.map((alignment) => ({ ...alignment })));
const tutorialWorkspace = useTutorialWorkspace(sourceRows, targetRows, alignmentRows);
const tutorialBookmarkIds = ref<string[]>([]);
const tutorialAnnotations = ref<AnnotationItem[]>([]);
const projectSnapshot = ref<WorkspaceProject | null>(null);
const activeTargetDocumentId = ref<string>();
const comparisonOpen = ref(true);
const comparisonWorkspaceRef = ref<{ openFind: () => Promise<void>; navigateFind: (direction: number) => Promise<void> } | null>(null);
const hasMultipleTranslations = computed(() => (projectSnapshot.value?.documents.length ?? 0) > 2);
const showComparison = computed(() => hasMultipleTranslations.value && comparisonOpen.value && activeMode.value === "review");
const activeTargetDocument = computed(() => projectSnapshot.value?.documents.find(document => document.document_id === activeTargetDocumentId.value) ?? projectSnapshot.value?.documents[1]);
const workspaceWritable = computed(() => tutorialActive.value || projectSnapshot.value !== null || !isTauriRuntime);
const projectSummary = ref<ProjectSummaryDto>({ project_id: "fixture-project", name: "2024政府工作报告_中英对齐", source_label: "report_zh.txt", target_label: "report_en.txt", source_count: 8, target_count: 8, alignment_count: 8, source_unlinked_count: 0, target_unlinked_count: 0, revision_id: "0" });

const modeItems = computed<Array<{ id: WorkspaceMode; label: string; hint: string; icon: typeof Eye }>>(() => [
  { id: "review", label: t("modeReview"), hint: t("modeReviewHint"), icon: Eye },
  { id: "edit", label: t("modeEdit"), hint: t("modeEditHint"), icon: Pencil },
  { id: "order", label: t("modeOrder"), hint: t("modeOrderHint"), icon: ListOrdered },
  { id: "history", label: t("modeHistory"), hint: t("modeHistoryHint"), icon: History },
]);
const annotationModeItem = computed<{ id: "annotation"; label: string; hint: string; icon: typeof Eye }>(() => ({
  id: "annotation",
  label: t("modeAnnotation"),
  hint: t("modeAnnotationHint"),
  icon: MessageSquareText,
}));
const modeMenuItems = computed<Array<{ id: ModeMenuId; label: string; hint: string; icon: typeof Eye }>>(() => [
  ...modeItems.value,
  annotationModeItem.value,
]);
const navItems = computed<Array<{ id: NavId; label: string; hint?: string; icon: typeof Folder }>>(() => [
  { id: "project", label: t("navProject"), icon: Folder }, { id: "parallel", label: t("navParallel"), icon: PanelLeft },
  { id: "search", label: t("navSearch"), icon: Search }, { id: "bookmarks", label: t("navBookmarks"), icon: Star },
  { id: "history", label: t("navHistory"), icon: History },
  { id: "pipeline", label: t("navPipeline"), icon: Workflow },
  { id: "settings", label: t("navSettings"), icon: Settings2 },
]);
const visibleNavItems = computed(() => researchFeature.value?.status === "ready" || researchVisited.value || researchHistoryAvailable.value
  ? [...navItems.value.slice(0, -1), { id: "research" as const, label: t("navResearch"), icon: Search }, ...navItems.value.slice(-1)]
  : navItems.value);
const segmentLabels = computed(() => new Map([...sourceRows.value, ...targetRows.value].map((segment) => [segment.id, String(segment.order + 1).padStart(6, "0")])));
const alignmentLabels = computed(() => new Map(alignmentRows.value.map((alignment) => [alignment.id, segmentLabels.value.get(alignment.sourceIds[0] ?? alignment.targetIds[0] ?? "") ?? alignment.id.slice(0, 8)])));
const segmentLabel = (segmentId: string) => segmentLabels.value.get(segmentId) ?? segmentId.slice(0, 8);
const alignmentLabel = (alignmentId: string | null | undefined) => {
  const alignment = alignmentRows.value.find((candidate) => candidate.id === alignmentId);
  return alignment ? segmentLabel(alignment.sourceIds[0] ?? alignment.targetIds[0] ?? alignment.id) : "—";
};
const humanizeSummary = (summary: string) => revisionSummary(summary).replace(/[0-9a-f]{8}-[0-9a-f-]{27,}/gi, (id) => segmentLabels.value.get(id) ?? alignmentLabels.value.get(id) ?? id.slice(0, 8));
const revisionItems = computed<RevisionItem[]>(() => [...(projectSnapshot.value?.revisions ?? [])].reverse().map((revision) => ({ id: revision.revision_id, label: `R${revision.revision_id}`, timestamp: formatDate(revision.created_at), action: revisionAction(revision.change_set.operation), summary: humanizeSummary(revision.summary), current: revision.revision_id === projectSummary.value.revision_id })));
const canUndo = computed(() => tutorialActive.value ? tutorialWorkspace.canUndo.value : (projectSnapshot.value?.revisions.length ?? 0) > 1);
const canRedo = computed(() => tutorialActive.value ? tutorialWorkspace.canRedo.value : (() => { const revisions = projectSnapshot.value?.revisions ?? []; return revisions[revisions.length - 1]?.change_set.operation.startsWith("undo:") ?? false; })());
const annotations = computed<AnnotationItem[]>(() => tutorialActive.value
  ? tutorialAnnotations.value.map((annotation) => ({ ...annotation, createdAt: t("shellJustNow") }))
  : (projectSnapshot.value?.annotations ?? []).map((annotation, index) => ({ id: annotation.annotation_id, number: index + 1, status: annotation.status, title: annotation.title, body: annotation.body, createdAt: formatDate(annotation.updated_at, true), links: annotation.linked_segment_ids.map((segmentId) => { const segment = [...sourceRows.value, ...targetRows.value].find((candidate) => candidate.id === segmentId); return { side: segment?.side ?? "source", segmentId, label: segmentLabel(segmentId), text: segment?.text }; }) })));
const bookmarkedSegmentIds = computed(() => tutorialActive.value ? tutorialBookmarkIds.value : (projectSnapshot.value?.bookmarks ?? []).map((bookmark) => bookmark.segment_id));
const annotatedSegmentIds = computed(() => tutorialActive.value ? [...new Set(tutorialAnnotations.value.flatMap(annotation => annotation.links.map(link => link.segmentId)))] : [...new Set((projectSnapshot.value?.annotations ?? []).flatMap((annotation) => annotation.linked_segment_ids))]);
const notify = (message: LocalizedMessage) => { statusMessage.value = message; };
const {
  settings,
  capabilities,
  general,
  autoSaveDelayMs,
  cacheCleaning,
  lastCacheCleanupAt,
  trackpadOptimized,
  usesMacShortcuts,
  shortcutLabels,
  shortcutRows,
  settingsSaving,
  settingsSaveError,
  effectiveMotionMode,
  smoothNavigationEnabled,
  systemReducedMotion,
  motionSummary,
  initializeSettings,
  disposeSettings,
  flushSettings,
  applyGeneralSettings,
  applyUiSettings,
  applyAccessibilitySettings,
  applyInteractionSettings,
  applyMotionSettings,
  applyPersistenceSettings,
  rememberSettingsSection,
  resetAllSettings,
  clearProjectCache,
  maybeCleanupCache,
} = useAppSettings({
  detectedMacOS,
  kernelClient,
  onStatus: notify,
});
const requireOpenProject = () => {
  if (tutorialActive.value) return true;
  if (projectSnapshot.value) return true;
  notify(() => t("demoReadOnly"));
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
  guardProjectChange: guardBodyProjectChange,
} = useViewModeController({
  autosaveDelayMs: autoSaveDelayMs,
  scope: commandScope,
  persist: persistSegment,
  onStatus: notify,
});
const dirty = computed(() => hasDirtyDraft.value || isSavingDraft.value);
const hasWorkspaceDraft = computed(() => pipelineDirty.value || researchDirty.value);
const guardWorkspaceDrafts = async (): Promise<boolean> => {
  if (workspaceLeave.value) return false;
  const handles: Array<[() => string, WorkspaceDraftHandle | null]> = [[() => t("navPipeline"), pipelineWorkspaceRef.value], [() => t("navResearch"), researchWorkspaceRef.value]];
  for (const [getLabel, handle] of handles) {
    if (!handle?.hasDirtyDraft()) continue;
    const allowed = await new Promise<boolean>(resolve => { workspaceLeave.value = { label: getLabel, handle, resolve }; });
    if (!allowed) return false;
  }
  return true;
};
const resolveWorkspaceLeave = async (action: "save" | "discard" | "stay") => {
  const pending = workspaceLeave.value;
  if (!pending || workspaceLeaveBusy.value) return;
  if (action === "stay") { workspaceLeave.value = null; pending.resolve(false); return; }
  workspaceLeaveBusy.value = true;
  try {
    if (action === "save" && !await pending.handle.saveDraft()) return;
    if (action === "discard") pending.handle.discardDraft();
    if (pending.handle.hasDirtyDraft()) return;
    workspaceLeave.value = null; pending.resolve(true);
  } catch (cause) { const detail = errorMessage(cause); notify(() => t("shellDraftSaveFailed", { p0: formatError(detail) })); }
  finally { workspaceLeaveBusy.value = false; }
};
const guardProjectChange = async () => await guardWorkspaceDrafts() && await guardBodyProjectChange();
const processedTotal = computed(() => {
  if (tutorialActive.value) return Math.max(projectSummary.value.source_count, projectSummary.value.target_count);
  const snapshot = projectSnapshot.value;
  if (!snapshot || snapshot.documents.length <= 2) return Math.max(projectSummary.value.source_count, projectSummary.value.target_count);
  const sourceCount = snapshot.segment_orders[0]?.entries.length ?? 0;
  return snapshot.segment_orders.slice(1).reduce((sum, order) => sum + Math.max(sourceCount, order.entries.length), 0);
});
const progressPercentage = computed(() => Math.min(100, Math.round(100 * projectSummary.value.alignment_count / Math.max(1, processedTotal.value))));
const alignmentStatus = computed(() => projectSummary.value.source_unlinked_count + projectSummary.value.target_unlinked_count === 0 ? "1:1" : t("needsReview"));
const footerStatusMessage = computed(() => busy.value
  ? t("processing")
  : isSavingDraft.value
    ? t("autosaving")
    : dirty.value
      ? t("autosavePending", { p0: autoSaveDelayMs.value / 1000 })
      : typeof statusMessage.value === "function" ? statusMessage.value() : statusMessage.value);
let unlistenClose: (() => void) | null = null;

const closeWindowSafely = async () => {
  if (!await guardWorkspaceDrafts()) return;
  const saved = await persistDraft(true, "close");
  if (!saved) return;
  await flushSettings();
  if (projectSnapshot.value) {
    try { await kernelClient.flushProject(); } catch (error) { const detail = errorMessage(error); notify(() => t("closeSaveFailed", { p0: formatError(detail) })); return; }
  }
  const currentWindow = getCurrentWindow();
  unlistenClose?.();
  unlistenClose = null;
  await currentWindow.close();
};

const errorMessage = (error: unknown) => error instanceof Error ? error.message : error && typeof error === "object" && "message" in error ? String(error.message) : String(error);
const closeAnnotationPanel = () => {
  annotationOpen.value = false;
  assistantOpen.value = false;
  annotationContextSegmentId.value = null;
};
const applySnapshot = async (
  snapshot: WorkspaceProject,
  resetView = false,
  selectFirstAlignmentOnMissing = true,
) => {
  const previous = projectSnapshot.value?.project;
  if (!resetView && previous?.project_id === snapshot.project.project_id && BigInt(previous.current_revision_id) >= BigInt(snapshot.project.current_revision_id)) return;
  const previousAlignmentId = selectedAlignmentId.value;
  workspaceText.selectProject(snapshot);
  if (resetView || !snapshot.documents.slice(1).some(document => document.document_id === activeTargetDocumentId.value)) activeTargetDocumentId.value = snapshot.documents[1].document_id;
  if (resetView) comparisonOpen.value = true;
  const workspace = workspaceIndex.selectProject(snapshot, workspaceText.texts.value, activeTargetDocumentId.value);
  projectSnapshot.value = snapshot;
  agentWorkspace.setProjectSnapshot(snapshot);
  sourceRows.value = workspace.sourceSegments;
  targetRows.value = workspace.targetSegments;
  alignmentRows.value = workspace.alignments;
  selectedAlignmentId.value = !resetView && workspace.alignments.some((alignment) => alignment.id === previousAlignmentId)
    ? previousAlignmentId
    : selectFirstAlignmentOnMissing ? (workspace.alignments[0]?.id ?? "") : "";
  projectSummary.value = snapshot.summary;
  bookmarkPreviews.value = [];
  void refreshBookmarkPreviews();
  if (resetView) {
    orderBaseline.value = {
      source: workspace.sourceSegments.map((segment) => segment.id),
      target: workspace.targetSegments.map((segment) => segment.id),
    };
    activeNav.value = "parallel";
    forceMode(general.value.defaultWorkspace);
    annotationContextSegmentId.value = null;
    selectedAnnotationId.value = null;
    resetSearch();
  }
};
const refreshVisibleRows = () => {
  if (!projectSnapshot.value) return;
  workspaceIndex.updateTexts(workspaceText.texts.value);
};
watch(workspaceText.texts, refreshVisibleRows);
const findCurrentPair = (query: string, cancelled: () => boolean) => workspaceText.find(query, cancelled, [projectSnapshot.value?.documents[0].document_id ?? "", activeTargetDocument.value?.document_id ?? ""]);
const openComparisonPair = async (documentId: string, alignmentId?: string) => {
  if (busy.value || !await guardProjectChange() || !projectSnapshot.value?.documents.slice(1).some(document => document.document_id === documentId)) return;
  clearWorkspaceSelection();
  activeTargetDocumentId.value = documentId;
  const workspace = workspaceIndex.selectProject(projectSnapshot.value, workspaceText.texts.value, documentId);
  sourceRows.value = workspace.sourceSegments; targetRows.value = workspace.targetSegments; alignmentRows.value = workspace.alignments;
  orderBaseline.value = { source: workspace.sourceSegments.map(segment => segment.id), target: workspace.targetSegments.map(segment => segment.id) };
  selectedAlignmentId.value = alignmentId ?? workspace.alignments[0]?.id ?? "";
  comparisonOpen.value = false; forceMode("review");
};
const returnToComparison = async () => {
  if (busy.value || !await guardProjectChange()) return;
  clearWorkspaceSelection(); forceMode("review"); comparisonOpen.value = true;
};
const loadVisibleSegments = async (ids: string[]) => {
  if (tutorialActive.value || !projectSnapshot.value) return;
  try { await workspaceText.visible([...ids, ...(editSession.value ? [editSession.value.segmentId] : [])]); }
  catch (error) { const detail = errorMessage(error); notify(() => t("shellTextLoadFailed", { p0: formatError(detail) })); }
};
async function refreshBookmarkPreviews() {
  if (activeNav.value !== "bookmarks" || !projectSnapshot.value) return;
  const scope = projectSnapshot.value;
  try {
    const previews = await kernelClient.listBookmarks();
    if (scope === projectSnapshot.value) bookmarkPreviews.value = previews;
  } catch (error) { const detail = errorMessage(error); notify(() => t("shellBookmarksLoadFailed", { p0: formatError(detail) })); }
}
watch(activeNav, () => { void refreshBookmarkPreviews(); });
const acceptAgentSearch = async ({ spec, results }: { spec: SearchSpec | null; results: unknown }) => {
  if (spec) {
    searchQuery.value = spec.query;
    searchRegex.value = spec.regex;
    searchCaseSensitive.value = spec.case_sensitive;
    searchSide.value = spec.document_ids?.length === 1 && spec.document_ids[0] === projectSnapshot.value?.documents[0].document_id ? "source" : spec.document_ids?.length === 1 ? "target" : "both";
  }
  const response = results as SearchSegmentsResponse | null;
  if (!response || !Array.isArray(response.hits)) { searchResults.value = []; return; }
  if (response.revision_id !== projectSnapshot.value?.project.current_revision_id) return;
  const searchScope = projectSnapshot.value;
  const relations = new Map(searchScope!.alignments.map(alignment => [alignment.alignment_id, alignment]));
  const sourceDocumentId = searchScope!.documents[0].document_id;
  await workspaceText.ensure(response.hits.flatMap(hit => {
    const relation = relations.get(hit.alignment_id ?? "");
    return [hit.segment_id, ...(relation?.source_segment_ids.slice(0, 1) ?? []), ...(relation?.target_segment_ids.slice(0, 1) ?? [])];
  }));
  if (searchScope !== projectSnapshot.value) return;
  refreshVisibleRows();
  searchResults.value = response.hits.map(hit => {
    const relation = relations.get(hit.alignment_id ?? "");
    const isSource = hit.document_id === sourceDocumentId;
    const sourceId = isSource ? hit.segment_id : relation?.source_segment_ids[0] ?? null;
    const targetId = !isSource ? hit.segment_id : relation?.target_segment_ids[0] ?? null;
    const document = searchScope!.documents.find(document => document.document_id === hit.document_id);
    const order = searchScope!.segment_orders.find(order => order.document_id === hit.document_id)?.entries.findIndex(entry => entry.segment_id === hit.segment_id) ?? -1;
    const text = (id: string | null) => id === hit.segment_id ? hit.content : id ? workspaceText.texts.value.get(id)?.content ?? "" : "";
    return { id: hit.segment_id, label: `${document?.title ?? t("content")} · ${order + 1}`, sourceId, targetId, sourceText: text(sourceId), targetText: text(targetId), alignmentId: hit.alignment_id, alignmentLabel: alignmentLabel(hit.alignment_id) };
  });
};
const agentWorkspace = useAgentWorkspace({
  tab: () => activeNav.value,
  mode: () => activeMode.value,
  projectSnapshot: () => projectSnapshot.value,
  selection: () => activeNav.value === "parallel" && !tutorialActive.value && projectSnapshot.value ? { segmentIds: [...new Set([...workspaceOperationSelection.value.segmentIds, ...(editSession.value ? [editSession.value.segmentId] : [])])], alignmentIds: workspaceOperationSelection.value.alignmentIds } : { segmentIds: [], alignmentIds: [] },
  searchState: () => ({ query: searchQuery.value, regex: searchRegex.value, caseSensitive: searchCaseSensitive.value, languageId: searchSide.value === "source" ? projectSnapshot.value?.project.source_language ?? null : searchSide.value === "target" ? projectSnapshot.value?.project.target_language ?? null : null }),
  selectionSharingEnabled: () => settings.value.device.agent.shareSelection,
  canLeaveDraft: () => {
    if (!dirty.value && !hasWorkspaceDraft.value) return true;
    notify(() => t("shellAssistantSaveBeforeNavigate"));
    return false;
  },
  navigate: async (tab) => {
    if (!visibleNavItems.value.some(item => item.id === tab)) throw new Error(t("shellAssistantPageUnavailable"));
    const scope = projectSnapshot.value?.project;
    if (tab === "pipeline") await loadPipelineWorkspace();
    if (scope?.project_id !== projectSnapshot.value?.project.project_id || scope?.current_revision_id !== projectSnapshot.value?.project.current_revision_id) throw new Error(t("shellProjectStateChanged", { p0: t("shellActivityNavigate") }));
    setNav(tab as NavId);
    await nextTick();
    if (activeNav.value !== tab) throw new Error(t("shellEditBlocksNavigation"));
    guidance.value = { selector: `[data-nav-id="${CSS.escape(tab)}"]`, label: t("shellArrivedAt", { p0: visibleNavItems.value.find(item => item.id === tab)?.label ?? tab }), key: Date.now() };
  },
  reveal: async ({ segment_id, alignment_id, node_id }) => {
    const scope = projectSnapshot.value?.project;
    if (node_id) {
      await loadPipelineWorkspace();
      if (scope?.project_id !== projectSnapshot.value?.project.project_id || scope?.current_revision_id !== projectSnapshot.value?.project.current_revision_id) throw new Error(t("shellProjectStateChanged", { p0: t("shellActivityReveal") }));
      setNav("pipeline"); await nextTick();
      if (!await pipelineWorkspaceRef.value?.revealNode(node_id)) throw new Error(t("shellPipelineNodeMissing"));
      guidance.value = { selector: `[data-pipeline-node-id="${CSS.escape(node_id)}"]`, label: t("shellLocatedNode"), key: Date.now() };
      return;
    }
    const alignment = alignmentRows.value.find(item => item.id === alignment_id);
    const segmentId = segment_id ?? alignment?.sourceIds[0] ?? alignment?.targetIds[0];
    if (!segmentId || ![...sourceRows.value, ...targetRows.value].some(row => row.id === segmentId)) throw new Error(t("shellSegmentMissing"));
    setNav("parallel");
    await nextTick();
    if (!await parallelWorkspaceRef.value?.focusSegment(segmentId)) throw new Error(t("shellSegmentLocatePending"));
    guidance.value = { selector: `[data-segment-id="${CSS.escape(segmentId)}"]`, label: t("shellLocatedSegment"), key: Date.now() };
  },
  onProjectSnapshot: async identity => {
    if (tutorialActive.value) return;
    const view = await kernelClient.getCurrentProject();
    if (view.project.project_id !== identity.project.project_id || view.project.current_revision_id !== identity.project.current_revision_id) return;
    await applySnapshot(view);
  },
  onSearchState: acceptAgentSearch,
  onProjection: () => refreshResearchFeature(),
  onAppEvent: async event => {
    if (event.kind === "pipeline_changed") { pipelineRefreshKey.value++; await reloadPipelineProposals(); }
    if (["capabilities_changed", "slot_binding_changed"].includes(event.kind)) pipelineRefreshKey.value++;
    if (["feature_changed", "capabilities_changed", "preparation_progress", "slot_binding_changed"].includes(event.kind)) await refreshResearchFeature();
  },
});
const displayedAgentActionError = computed(() => {
  const error = agentActionError.value ?? agentWorkspace.error.value;
  return error === null ? null : formatError(error);
});
const agentProjectKey = computed(() => projectSnapshot.value?.project.project_id ?? "application");
watch(() => [projectSnapshot.value?.project.project_id, agentWorkspace.binding.value?.binding_id] as const, async ([projectId, binding]) => {
  const generation = ++researchHistoryGeneration;
  researchHistoryAvailable.value = false;
  if (!isTauriRuntime || !projectId || !binding) return;
  try {
    const history = await researchClient.listRuns(binding);
    if (generation === researchHistoryGeneration) researchHistoryAvailable.value = history.some(run => run.project_id === projectId);
  } catch (cause) { if (generation === researchHistoryGeneration) { const detail = errorMessage(cause); notify(() => t("shellResearchHistoryFailed", { p0: formatError(detail) })); } }
}, { immediate: true });
const agentDraft = computed({
  get: () => agentWorkspace.store.ensureProjectScope(agentProjectKey.value).drafts.composer ?? "",
  set: value => agentWorkspace.store.setDraft(agentProjectKey.value, "composer", value),
});
const agentActivityLabels = computed<Record<string, string>>(() => ({ navigation_requested: t("shellActivityNavigate"), reveal_requested: t("shellActivityReveal"), search_changed: t("shellActivitySearch"), proposal_changed: t("shellActivityProposal"), operation_changed: t("shellActivityOperation"), revision_advanced: t("shellActivitySaved"), project_changed: t("shellActivityProjectChanged") }));
const agentActivity = computed(() => agentWorkspace.store.activity.filter(event => event.kind !== "context_changed").map(event => ({ id: event.sequence, title: agentActivityLabels.value[event.kind] ?? event.kind, status: event.origin === "native" ? t("shellActivityNative") : t("shellActivityExternal") })));
const textProposals = computed<ReviewableProposal[]>(() => agentWorkspace.store.proposals.filter(proposal => proposal.status === "pending").flatMap(proposal => {
  const preview = proposal.preview as { base_revision_id?: string; items?: Array<{ segment_id: string; before: string; after: string }> } | undefined;
  if (!proposal.proposal_id || !preview?.items) return [];
  const selectedIds = (proposal.request as { selected_segment_ids?: string[] } | undefined)?.selected_segment_ids ?? [];
  const changes = preview.items.filter(item => !selectedIds.length || selectedIds.includes(item.segment_id));
  return [{ id: proposal.proposal_id, title: t("shellReviewTextReplace"), status: "pending", revision: preview.base_revision_id ?? proposal.base_revision_id ?? "—", changes: changes.map(item => ({ id: item.segment_id, before: item.before, after: item.after })) }];
}));
const agentProposals = computed<ReviewableProposal[]>(() => [...textProposals.value, ...pipelineProposals.value.filter(proposal => proposal.status === "pending").map(proposal => ({ id: proposal.proposal_id, title: t("shellReviewMethod", { p0: proposal.before.name }), status: proposal.status, revision: proposal.base_canonical_revision_id, changes: [{ id: proposal.method_id, before: JSON.stringify(proposal.before.current.plan.nodes.map(node => ({ operator: node.operator, config: node.config })), null, 2), after: JSON.stringify(proposal.request.plan.nodes.map(node => ({ operator: node.operator, config: node.config })), null, 2) }] }))]);
async function reloadPipelineProposals() {
  const binding = agentWorkspace.binding.value?.binding_id;
  if (!binding) { pipelineProposals.value = []; return; }
  try { const proposals = await pipelineClient.listProposals(binding); if (binding === agentWorkspace.binding.value?.binding_id) pipelineProposals.value = proposals; }
  catch { /* Initial binding can be replaced while opening a project. */ }
}
watch(() => agentWorkspace.binding.value?.binding_id, () => { pipelineProposals.value = []; void reloadPipelineProposals(); });
const reviewAgentProposal = async (id: string, approve: boolean) => {
  if (approve && dirty.value) { notify(() => t("shellAssistantSaveBeforeReview")); return; }
  const bindingId = agentWorkspace.binding.value?.binding_id;
  if (!bindingId) return;
  agentBusy.value = true;
  agentActionError.value = null;
  try {
    if (pipelineProposals.value.some(proposal => proposal.proposal_id === id)) {
      await (approve ? pipelineClient.approveUpdate(id, bindingId) : pipelineClient.rejectUpdate(id, bindingId));
      await reloadPipelineProposals();
    } else await agentClient.call(approve ? "proposal.approve" : "proposal.reject", { proposal_id: id }, bindingId);
    notify(() => t(approve ? "shellAssistantAccepted" : "shellAssistantRejected"));
  } catch (error) { agentActionError.value = rawErrorMessage(error); }
  finally { agentBusy.value = false; }
};
const openAgentSettings = () => { rememberSettingsSection("agent"); setNav("settings"); };
async function loadRuntimeHistory() {
  const generation = ++runtimeHistoryGeneration;
  const projectId = projectSnapshot.value?.project.project_id;
  if (!projectId || !isTauriRuntime) { runtimeMessages.value = []; return; }
  try {
    const history = await agentRuntimeClient.history({ project_id: projectId, session_id: runtimeSessions.get(projectId), limit: 80 });
    if (generation !== runtimeHistoryGeneration || projectId !== projectSnapshot.value?.project.project_id) return;
    runtimeSessions.set(projectId, history.session_id); runtimeMessages.value = history.messages;
  } catch (error) { if (generation === runtimeHistoryGeneration && projectId === projectSnapshot.value?.project.project_id) { runtimeMessages.value = []; agentActionError.value = rawErrorMessage(error); } }
}
async function handleRuntimeEvent(event: AgentRuntimeEvent) {
  runtimeSessions.set(event.project_id, event.session_id);
  runtimeRuns.value = { ...runtimeRuns.value, [event.project_id]: { runId: event.run_id, state: event.state } };
  if (event.project_id !== projectSnapshot.value?.project.project_id) return;
  if (event.kind === "run_failed") {
    const payload = event.payload as { error?: string };
    agentActionError.value = payload.error ?? new LocalizedError("shellAssistantFailed");
  }
  await loadRuntimeHistory();
}
async function initializeRuntime() {
  if (!isTauriRuntime) return;
  try {
    unlistenRuntime = await agentRuntimeClient.subscribe(event => { void handleRuntimeEvent(event); });
    unlistenRuntimeResync = await agentRuntimeClient.subscribeResync(() => { void refreshRuntimeState(); });
    await refreshRuntimeState();
  }
  catch (error) { agentActionError.value = rawErrorMessage(error); }
}
async function refreshRuntimeState() {
  try {
    const status = await agentRuntimeClient.status();
    runtimeStatus.value = status;
    runtimeRuns.value = Object.fromEntries(Object.entries(runtimeRuns.value).map(([id, run]) => [id, status.active_run_ids.includes(run.runId) ? run : { ...run, state: "interrupted" }]));
    await loadRuntimeHistory();
  } catch (error) { agentActionError.value = rawErrorMessage(error); }
}
async function sendAgentMessage(text: string) {
  const projectId = projectSnapshot.value?.project.project_id;
  if (!projectId || !runtimeStatus.value?.configured || runtimeRunning.value) return;
  agentActionError.value = null;
  const messageId = crypto.randomUUID();
  const sentDraft = agentWorkspace.store.ensureProjectScope(projectId).drafts.composer;
  runtimeStarting.value = projectId;
  try {
    await agentWorkspace.publishContext();
    if (projectId !== projectSnapshot.value?.project.project_id) throw new Error(t("shellProjectChangedResend"));
    const context = agentWorkspace.recordOutboundMessage(projectId, messageId, crypto.randomUUID(), text);
    const result = await agentRuntimeClient.start({ project_id: projectId, prompt: text, context, session_id: runtimeSessions.get(projectId) });
    runtimeSessions.set(projectId, result.session_id); runtimeRuns.value = { ...runtimeRuns.value, [projectId]: { runId: result.run_id, state: result.state } };
    agentWorkspace.store.markMessageSent(projectId, messageId);
    if (agentWorkspace.store.ensureProjectScope(projectId).drafts.composer === sentDraft) agentWorkspace.store.setDraft(projectId, "composer", "");
    await loadRuntimeHistory();
  } catch (error) { if (projectId === projectSnapshot.value?.project.project_id) agentActionError.value = rawErrorMessage(error); agentWorkspace.store.markMessageFailed(projectId, messageId, errorMessage(error)); }
  finally { if (runtimeStarting.value === projectId) runtimeStarting.value = null; }
}
async function cancelAgentRun() {
  const run = runtimeRuns.value[agentProjectKey.value];
  if (!run) return;
  try { await agentRuntimeClient.cancel(run.runId); } catch (error) { agentActionError.value = rawErrorMessage(error); }
}
watch(() => projectSnapshot.value?.project.project_id, () => { runtimeMessages.value = []; void loadRuntimeHistory(); });
watch(() => agentProposals.value.length, count => {
  if (count && settings.value.device.agent.openOnRequest) { assistantOpen.value = true; rightPanelTab.value = "agent"; }
});
onMounted(async () => {
  sideNavCollapsed.value = localStorage.getItem("jueming-nav-collapsed") === "true";
  const savedAnnotationWidthValue = localStorage.getItem("jueming-annotation-width");
  const savedAnnotationWidth = savedAnnotationWidthValue === null ? Number.NaN : Number(savedAnnotationWidthValue);
  if (Number.isFinite(savedAnnotationWidth)) annotationWidth.value = Math.min(560, Math.max(300, savedAnnotationWidth));
  await initializeSettings();
  if (isTauriRuntime) await agentWorkspace.start();
  await initializeRuntime();
  try {
    await applySnapshot(await kernelClient.getCurrentProject(), true);
  } catch {
    if (general.value.startupDestination === "last-project") {
      const recentProjectPath = isTauriRuntime ? localStorage.getItem("jueming-last-project-path") : null;
      if (recentProjectPath) {
        try {
          await applySnapshot(await kernelClient.openProject(recentProjectPath), true);
          const projectName = projectSummary.value.name;
          notify(() => t("projectReopened", { p0: projectName }));
        } catch {
          localStorage.removeItem("jueming-last-project-path");
        }
      }
    } else if (!autoStartTutorialOnThisLaunch && general.value.startupDestination === "project-picker" && isTauriRuntime) {
      await openProject();
    }
  }
  if (projectSnapshot.value) await maybeCleanupCache();
  else if (isTauriRuntime) {
    activeNav.value = "project";
    selectedAlignmentId.value = "";
    notify(() => t("projectOpenPrompt"));
  }
  if (isTauriRuntime) {
    unlistenClose = await getCurrentWindow().onCloseRequested((event) => { event.preventDefault(); void closeWindowSafely(); });
  }
  if (autoStartTutorialOnThisLaunch) {
    await nextTick();
    await startTutorial();
    if (tutorialActive.value) rememberTutorialLaunch();
  }
});
const previousRevisionId = () => { const revisions = projectSnapshot.value?.revisions ?? []; return revisions[revisions.length - 2]?.revision_id ?? projectSummary.value.revision_id; };
const applyModeContext = (mode: WorkspaceMode) => {
  activeNav.value = mode === "history" ? "history" : "parallel";
  if (mode === "history") void selectHistoryRevision(previousRevisionId());
};
const setMode = (mode: WorkspaceMode) => {
  if (tutorialActive.value && mode === "history") { notify(() => t("shellTutorialHistoryNotice")); return; }
  if (hasWorkspaceDraft.value) {
    void guardWorkspaceDrafts().then(allowed => { if (allowed) setMode(mode); });
    return;
  }
  pendingNav.value = null;
  if (requestMode(mode) === "applied") applyModeContext(mode);
};
const applyNavContext = (nav: NavId) => {
  activeNav.value = nav;
  if (nav === "history") { forceMode("history"); void selectHistoryRevision(previousRevisionId()); }
  else if (activeMode.value === "history") forceMode("review");
};
const setNav = (nav: NavId) => {
  if (tutorialActive.value && nav !== "parallel") { notify(() => t("shellTutorialFinishFirst")); return; }
  if (nav !== activeNav.value && hasWorkspaceDraft.value) {
    void guardWorkspaceDrafts().then(allowed => { if (allowed) setNav(nav); });
    return;
  }
  if (activeMode.value === "edit" && nav !== "parallel") {
    pendingNav.value = nav;
    if (requestMode(nav === "history" ? "history" : "review") === "guarded") return;
  }
  pendingNav.value = null;
  applyNavContext(nav);
};
type TutorialReturnState = {
  source: SegmentDto[];
  target: SegmentDto[];
  alignments: AlignmentDto[];
  summary: ProjectSummaryDto;
  selectedAlignmentId: string;
  activeNav: NavId;
  activeMode: WorkspaceMode;
  orderBaseline: Record<LanguageSide, string[]>;
  comparisonOpen: boolean;
};
let tutorialReturnState: TutorialReturnState | null = null;
const syncTutorialSummary = () => {
  if (!tutorialActive.value) return;
  const aligned = new Set(alignmentRows.value.flatMap(alignment => [...alignment.sourceIds, ...alignment.targetIds]));
  projectSummary.value = {
    ...projectSummary.value,
    source_count: sourceRows.value.length,
    target_count: targetRows.value.length,
    alignment_count: alignmentRows.value.length,
    source_unlinked_count: sourceRows.value.filter(segment => !aligned.has(segment.id)).length,
    target_unlinked_count: targetRows.value.filter(segment => !aligned.has(segment.id)).length,
    revision_id: String(tutorialWorkspace.revision.value),
  };
};
const prepareTutorialStep = async (index: number) => {
  if (!tutorialActive.value) return;
  tutorialStep.value = Math.min(parallelTutorialSteps.length - 1, Math.max(0, index));
  if (editSession.value) discardAndExit();
  const step = parallelTutorialSteps[tutorialStep.value];
  if (step.preset !== "context") closeAnnotationPanel();
  activeNav.value = "parallel";
  forceMode(step.mode);
  await nextTick();
  await parallelWorkspaceRef.value?.prepareTutorial(step.preset);
};
const resetTutorial = async () => {
  if (!tutorialActive.value) return;
  if (editSession.value) discardAndExit();
  tutorialWorkspace.reset();
  tutorialBookmarkIds.value = [];
  tutorialAnnotations.value = [];
  selectedAnnotationId.value = null;
  orderBaseline.value = { source: sourceRows.value.map(segment => segment.id), target: targetRows.value.map(segment => segment.id) };
  syncTutorialSummary();
  await prepareTutorialStep(tutorialStep.value);
  notify(() => t("shellTutorialReset"));
};
async function startTutorial() {
  if (tutorialActive.value) {
    tutorialGuideOpen.value = true;
    await prepareTutorialStep(tutorialStep.value);
    return;
  }
  if (busy.value || !await guardProjectChange()) return;
  tutorialReturnState = {
    source: sourceRows.value, target: targetRows.value, alignments: alignmentRows.value,
    summary: projectSummary.value, selectedAlignmentId: selectedAlignmentId.value,
    activeNav: activeNav.value, activeMode: activeMode.value,
    orderBaseline: orderBaseline.value, comparisonOpen: comparisonOpen.value,
  };
  closeAnnotationPanel();
  clearWorkspaceSelection();
  tutorialActive.value = true;
  tutorialGuideOpen.value = true;
  tutorialWorkspace.start();
  tutorialBookmarkIds.value = [];
  tutorialAnnotations.value = [];
  selectedAnnotationId.value = null;
  selectedAlignmentId.value = "tutorial-alignment-1";
  comparisonOpen.value = false;
  projectSummary.value = { project_id: "tutorial", name: "政府工作报告 · 平行视图引导", source_label: "政府工作报告_中文节选.txt", target_label: "Government_Work_Report_excerpt.txt", source_count: 8, target_count: 8, alignment_count: 6, source_unlinked_count: 1, target_unlinked_count: 1, revision_id: "0" };
  orderBaseline.value = { source: sourceRows.value.map(segment => segment.id), target: targetRows.value.map(segment => segment.id) };
  await prepareTutorialStep(0);
  notify(() => t("shellTutorialEntered"));
}
const dismissTutorialGuide = () => {
  if (!tutorialActive.value) return;
  tutorialGuideOpen.value = false;
  notify(() => t("shellTutorialDismissed"));
};
const closeTutorial = async () => {
  const previous = tutorialReturnState;
  if (!tutorialActive.value || !previous) return;
  if (editSession.value) discardAndExit();
  closeAnnotationPanel();
  clearWorkspaceSelection();
  tutorialGuideOpen.value = false;
  tutorialActive.value = false;
  sourceRows.value = previous.source;
  targetRows.value = previous.target;
  alignmentRows.value = previous.alignments;
  projectSummary.value = previous.summary;
  selectedAlignmentId.value = previous.selectedAlignmentId;
  orderBaseline.value = previous.orderBaseline;
  comparisonOpen.value = previous.comparisonOpen;
  forceMode(previous.activeMode);
  activeNav.value = previous.activeNav;
  tutorialReturnState = null;
  await nextTick();
  const returnedProjectName = projectSummary.value.name;
  notify(projectSnapshot.value ? () => t("shellTutorialReturned", { p0: returnedProjectName }) : () => t("shellTutorialExited"));
};
const acceptResearchFeature = (feature: FeatureSnapshot) => {
  const previous = researchFeature.value;
  researchFeature.value = feature;
  if ((previous?.status === "ready") !== (feature.status === "ready")) pipelineRefreshKey.value++;
  if (feature.status !== "ready" || previous?.status === "ready") return;
  notify(() => t("shellResearchReady"));
  const intent = featureNavigationIntent; featureNavigationIntent = null;
  if (intent && intent.navigationGeneration === navigationGeneration && intent.projectId === (projectSnapshot.value?.project.project_id ?? null) && activeNav.value === "settings" && !dirty.value && !hasWorkspaceDraft.value) setNav("research");
};
const refreshResearchFeature = async () => {
  if (!isTauriRuntime) return;
  const generation = ++featureRequestGeneration;
  try {
    const snapshot = await researchClient.capabilities();
    if (generation !== featureRequestGeneration) return;
    const feature = snapshot.features.find(item => item.feature_id === "translation_research");
    if (feature) acceptResearchFeature(feature);
  } catch (cause) { if (generation === featureRequestGeneration) { const detail = errorMessage(cause); notify(() => t("shellFeatureStatusFailed", { p0: formatError(detail) })); } }
};
const researchFeatureAction = async (action: "enable" | "disable" | "cancel" | "retry") => {
  if (!isTauriRuntime || researchFeatureBusy.value) return;
  if (action === "disable" && !await guardWorkspaceDrafts()) return;
  if (action === "enable" || action === "retry") featureNavigationIntent = { projectId: projectSnapshot.value?.project.project_id ?? null, navigationGeneration };
  else featureNavigationIntent = null;
  researchFeatureBusy.value = true; const requestGeneration = ++featureRequestGeneration;
  try {
    const result = await ({ enable: researchClient.featureEnable, disable: researchClient.featureDisable, cancel: researchClient.featureCancel, retry: researchClient.featureRetry })[action]();
    if (requestGeneration === featureRequestGeneration) acceptResearchFeature(result);
    await refreshResearchFeature();
  } catch (cause) { const detail = errorMessage(cause); notify(() => t("shellResearchActionFailed", { p0: formatError(detail) })); await refreshResearchFeature(); }
  finally { researchFeatureBusy.value = false; }
};
const updateResearchPreferences = async (value: { default_similarity?: string; auto_locate?: boolean }) => {
  if (!isTauriRuntime || researchFeatureBusy.value) return;
  researchFeatureBusy.value = true; featureRequestGeneration++;
  try { acceptResearchFeature(await researchClient.updatePreferences(value)); }
  catch (cause) { const detail = errorMessage(cause); notify(() => t("shellResearchPreferencesFailed", { p0: formatError(detail) })); }
  finally { researchFeatureBusy.value = false; }
};
const revealResearchSegment = async (segmentId: string) => {
  if (!await jumpToSegment(segmentId)) notify(() => t("shellSegmentChanged"));
};
const openResearchSettings = () => { rememberSettingsSection("research"); setNav("settings"); };
const refreshResearchOnFocus = () => { if (document.visibilityState === "visible") void refreshResearchFeature(); };
watch(activeNav, tab => { if (tab === "settings") void refreshResearchFeature(); });
onMounted(() => { void refreshResearchFeature(); window.addEventListener("focus", refreshResearchOnFocus); });
onBeforeUnmount(() => { featureRequestGeneration++; window.removeEventListener("focus", refreshResearchOnFocus); });
const openAnnotationPanel = (segmentId?: string, alignmentId?: string | null) => {
  if (alignmentId !== undefined) selectedAlignmentId.value = alignmentId ?? "";
  annotationContextSegmentId.value = segmentId ?? null;
  if (segmentId) {
    selectedAnnotationId.value = annotations.value.find((annotation) => annotation.links.some((link) => link.segmentId === segmentId))?.id ?? null;
  }
  annotationOpen.value = true;
  rightPanelTab.value = "annotations";
};
const openNewProject = async () => {
  if (tutorialActive.value) { notify(() => t("shellCloseTutorialBeforeNew")); return; }
  if (busy.value || !await guardProjectChange()) return;
  clearWorkspaceSelection();
  window.getSelection()?.removeAllRanges();
  await newProjectDialogRef.value?.open();
};
const openProject = async () => {
  if (tutorialActive.value) { notify(() => t("shellCloseTutorialBeforeOpen")); return; }
  if (busy.value) return;
  const selected = await open({ directory: true, multiple: false, title: t("projectOpenDialog") });
  if (!selected || !await guardProjectChange()) return;
  clearWorkspaceSelection();
  window.getSelection()?.removeAllRanges();
  busy.value = true;
  try { await applySnapshot(await kernelClient.openProject(selected), true); localStorage.setItem("jueming-last-project-path", selected); const projectName = projectSummary.value.name; notify(() => t("projectOpened", { p0: projectName })); }
  catch (error) { const detail = errorMessage(error); notify(() => t("projectOpenFailed", { p0: formatError(detail) })); }
  finally { busy.value = false; }
};
const handleProjectCreated = async (snapshot: WorkspaceProject, createdProjectPath: string) => {
  localStorage.setItem("jueming-last-project-path", createdProjectPath);
  try {
    await applySnapshot(snapshot, true);
    const projectName = projectSummary.value.name;
    notify(() => t("projectCreated", { p0: projectName }));
  } catch (error) {
    const detail = errorMessage(error);
    notify(() => t("projectLoadFailed", { p0: formatError(detail) }));
  }
};
const saveProject = async () => {
  if (tutorialActive.value) { notify(() => t("shellTutorialTemporarySaved")); return; }
  const handle = activeNav.value === "pipeline" ? pipelineWorkspaceRef.value : activeNav.value === "research" ? researchWorkspaceRef.value : null;
  if (handle?.hasDirtyDraft() && !await handle.saveDraft()) return;
  if (!(await persistDraft(false, "manual"))) return;
  try { await kernelClient.flushProject(); notify(() => t("savedLocally")); }
  catch (error) { const detail = errorMessage(error); notify(() => t("saveFailedDetail", { p0: formatError(detail) })); }
};
async function persistSegment(id: string, text: string, context?: CommandContext): Promise<CommandScope | void> {
  if (tutorialActive.value) {
    tutorialWorkspace.edit(id, text);
    syncTutorialSummary();
    notify(() => t("segmentAutosaved"));
    return;
  }
  if (!projectSnapshot.value) return;
  const result = await kernelClient.updateSegment(id, text, context);
  try { await applySnapshot(await kernelClient.getCurrentProject()); }
  catch (error) { const detail = errorMessage(error); notify(() => t("shellEditRefreshFailed", { p0: formatError(detail) })); }
  return { project_id: result.project_id, base_revision_id: result.committed_revision_id };
}
const requestSegmentEdit = async (segmentId: string, alignmentId: string) => {
  if (editSession.value?.segmentId === segmentId) return;
  if (editSession.value && editSession.value.segmentId !== segmentId && !await guardProjectChange()) return;
  const scope = projectSnapshot.value;
  if (!tutorialActive.value) {
    try { await workspaceText.ensure([segmentId]); } catch (error) { const detail = errorMessage(error); notify(() => formatError(detail)); return; }
  }
  if (scope !== projectSnapshot.value) return;
  refreshVisibleRows();
  const segment = [...sourceRows.value, ...targetRows.value].find((item) => item.id === segmentId);
  if (!segment) return;
  selectedAlignmentId.value = alignmentId;
  activeNav.value = "parallel";
  enterEdit(segmentId, alignmentId, segment.text);
};
const commitSegmentEdit = async (exitAfterSave: boolean) => {
  const saved = await persistDraft(exitAfterSave, "manual");
  if (saved && exitAfterSave) activeNav.value = "parallel";
};
const escapeSegmentEdit = async () => {
  const saved = await persistDraft(true, "escape");
  if (saved) activeNav.value = "parallel";
};
const cancelSegmentEdit = () => { discardAndExit(); activeNav.value = "parallel"; };
const moveSegment = async (side: LanguageSide, segmentId: string, direction: "up" | "down") => {
  if (!requireOpenProject()) return;
  if (tutorialActive.value) {
    const moved = tutorialWorkspace.move(side, segmentId, direction);
    notify(() => moved ? t("shellTutorialOrderMoved") : t(direction === "up" ? "firstSegment" : "lastSegment"));
    syncTutorialSummary();
    return;
  }
  const rows = side === "source" ? sourceRows : targetRows;
  const index = rows.value.findIndex((item) => item.id === segmentId);
  const targetIndex = direction === "up" ? index - 1 : index + 1;
  if (index < 0 || targetIndex < 0 || targetIndex >= rows.value.length) {
    notify(() => t(direction === "up" ? "firstSegment" : "lastSegment"));
    return;
  }
  const targetSegment = rows.value[targetIndex];
  if (!(await confirmCrossAlignmentReorder(side, {
    segmentId,
    targetId: targetSegment.id,
    edge: direction === "up" ? "before" : "after",
  }))) {
    notify(() => t("reorderCancelled"));
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
    notify(() => t("segmentMoved", { p0: t(side === "source" ? "sourceSide" : "targetSide"), p1: t(direction === "up" ? "moveUp" : "moveDown") }));
  } catch (error) {
    rows.value = previous;
    const detail = errorMessage(error); notify(() => t("reorderFailed", { p0: formatError(detail) }));
  }
};
const alignmentIdForSegment = (segmentId: string) => alignmentRows.value.find(
  (alignment) => [...alignment.sourceIds, ...alignment.targetIds].includes(segmentId),
)?.id ?? null;
const describeAlignmentForSegment = (segmentId: string) => {
  const alignmentId = alignmentIdForSegment(segmentId);
  return alignmentId ? `${t("alignment")} ${alignmentLabel(alignmentId)}` : t("unalignedArea");
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
  const message = t("crossAlignmentWarning", { p0: t(side === "source" ? "sourceSide" : "targetSide"), p1: segmentLabel(intent.segmentId), p2: describeAlignmentForSegment(intent.segmentId), p3: describeAlignmentForSegment(intent.targetId) });
  return isTauriRuntime
    ? confirm(message, { title: t("crossAlignmentTitle"), kind: "warning" })
    : window.confirm(t("crossAlignmentConfirm", { p0: message }));
};
const reorderSegment = async (side: LanguageSide, orderedSegmentIds: string[], intent: ReorderIntent) => {
  if (!requireOpenProject()) return;
  if (tutorialActive.value) {
    tutorialWorkspace.reorder(side, orderedSegmentIds);
    clearWorkspaceSelection();
    syncTutorialSummary();
    notify(() => t("shellTutorialDragSaved"));
    return;
  }
  if (!(await confirmCrossAlignmentReorder(side, intent))) {
    notify(() => t("reorderCancelled"));
    return;
  }
  busy.value = true;
  try { await applySnapshot(await kernelClient.reorderSegments(orderedSegmentIds)); clearWorkspaceSelection(); notify(() => t("dragSaved", { p0: t(side === "source" ? "sourceSide" : "targetSide") })); }
  catch (error) { const detail = errorMessage(error); notify(() => t("dragFailed", { p0: formatError(detail) })); }
  finally { busy.value = false; }
};
const resetOrder = async () => {
  if (!requireOpenProject()) return;
  if (tutorialActive.value) {
    tutorialWorkspace.resetOrder();
    clearWorkspaceSelection();
    syncTutorialSummary();
    notify(() => t("shellTutorialInitialOrderRestored"));
    return;
  }
  if (!orderBaseline.value.source.length && !orderBaseline.value.target.length) { notify(() => t("noOrderBaseline")); return; }
  busy.value = true;
  try {
    const currentOrders: Record<LanguageSide, string[]> = {
      source: sourceRows.value.map((segment) => segment.id),
      target: targetRows.value.map((segment) => segment.id),
    };
    for (const side of ["source", "target"] as const) {
      const baseline = orderBaseline.value[side];
      if (baseline.length && baseline.join("\u0000") !== currentOrders[side].join("\u0000")) {
        await applySnapshot(await kernelClient.reorderSegments(baseline));
      }
    }
    clearWorkspaceSelection();
    notify(() => t("orderRestored"));
  } catch (error) {
    await applySnapshot(await kernelClient.getCurrentProject());
    const detail = errorMessage(error); notify(() => t("orderRestoreFailed", { p0: formatError(detail) }));
  }
  finally { busy.value = false; }
};
const linkSegments = async (sourceSegmentIds: string[], targetSegmentIds: string[]) => {
  if (!requireOpenProject()) return;
  if (tutorialActive.value) {
    selectedAlignmentId.value = tutorialWorkspace.link(sourceSegmentIds, targetSegmentIds);
    clearWorkspaceSelection();
    syncTutorialSummary();
    const sourceCount = sourceSegmentIds.length;
    const targetCount = targetSegmentIds.length;
    notify(() => t("shellTutorialAlignmentCreated", { p0: sourceCount, p1: targetCount }));
    return;
  }
  busy.value = true;
  try {
    let snapshot: WorkspaceProject;
    try {
      snapshot = await kernelClient.linkSegments(sourceSegmentIds, targetSegmentIds, false);
    } catch (error) {
      const message = errorMessage(error);
      if (!message.includes("confirm replacement before linking") && !message.includes("AlignmentSelectionConflict")) throw error;
      const approved = "__TAURI_INTERNALS__" in window
        ? await confirm(t("replaceAlignmentWarning"), { title: t("replaceAlignmentTitle"), kind: "warning" })
        : window.confirm(t("replaceAlignmentConfirm"));
      if (!approved) return;
      snapshot = await kernelClient.linkSegments(sourceSegmentIds, targetSegmentIds, true);
    }
    await applySnapshot(snapshot);
    clearWorkspaceSelection();
    selectedAlignmentId.value = alignmentRows.value.find((alignment) => sourceSegmentIds.every((id) => alignment.sourceIds.includes(id)) && targetSegmentIds.every((id) => alignment.targetIds.includes(id)))?.id ?? selectedAlignmentId.value;
    const sourceCount = sourceSegmentIds.length;
    const targetCount = targetSegmentIds.length;
    notify(() => t("alignmentCreated", { p0: sourceCount, p1: targetCount }));
  } catch (error) { const detail = errorMessage(error); notify(() => t("linkFailed", { p0: formatError(detail) })); }
  finally { busy.value = false; }
};
const unlinkAlignment = async (alignmentId: string) => {
  if (busy.value || !requireOpenProject()) return;
  if (tutorialActive.value) {
    tutorialWorkspace.unlink(alignmentId);
    clearWorkspaceSelection();
    syncTutorialSummary();
    notify(() => t("shellTutorialAlignmentRemoved"));
    return;
  }
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
    notify(() => t("alignmentRemoved"));
  }
  catch (error) { const detail = errorMessage(error); notify(() => t("unlinkFailed", { p0: formatError(detail) })); }
  finally { busy.value = false; }
};
const {
  insertAlignmentGap: insertCanonicalAlignmentGap,
  mergeSegmentContent: mergeCanonicalSegmentContent,
  splitSegmentContent: splitCanonicalSegmentContent,
  groupAlignments: groupCanonicalAlignments,
  ungroupAlignment: ungroupCanonicalAlignment,
} = useStructureMutations({
  kernelClient,
  targetDocumentId: () => activeTargetDocumentId.value,
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
const insertAlignmentGap = async (segmentId: string, edge: "before" | "after") => {
  if (!tutorialActive.value) return insertCanonicalAlignmentGap(segmentId, edge);
  tutorialWorkspace.insertGap(segmentId);
  clearWorkspaceSelection(); syncTutorialSummary();
  notify(() => t("shellTutorialGapInserted"));
};
const mergeSegmentContent = async (segmentIds: string[], content: string) => {
  if (!tutorialActive.value) return mergeCanonicalSegmentContent(segmentIds, content);
  tutorialWorkspace.merge(segmentIds, content);
  clearWorkspaceSelection(); syncTutorialSummary();
  const count = segmentIds.length; notify(() => t("shellTutorialSegmentsMerged", { p0: count }));
};
const splitSegmentContent = async (segmentId: string, parts: string[]) => {
  if (!tutorialActive.value) return splitCanonicalSegmentContent(segmentId, parts);
  tutorialWorkspace.split(segmentId, parts);
  clearWorkspaceSelection(); syncTutorialSummary();
  const count = parts.length; notify(() => t("shellTutorialSegmentsSplit", { p0: count }));
};
const groupAlignments = async (alignmentIds: string[], unlinkedSegmentIds: string[]) => {
  if (!tutorialActive.value) return groupCanonicalAlignments(alignmentIds, unlinkedSegmentIds);
  selectedAlignmentId.value = tutorialWorkspace.group(alignmentIds, unlinkedSegmentIds);
  clearWorkspaceSelection(); syncTutorialSummary();
  notify(() => t("shellTutorialGrouped"));
};
const ungroupAlignment = async (alignmentId: string, sourceGroups: string[][], targetGroups: string[][]) => {
  if (!tutorialActive.value) return ungroupCanonicalAlignment(alignmentId, sourceGroups, targetGroups);
  tutorialWorkspace.ungroup(alignmentId, sourceGroups, targetGroups);
  clearWorkspaceSelection(); syncTutorialSummary();
  notify(() => t("shellTutorialUngrouped"));
};
const performHistoryChange = async (kind: "undo" | "redo" | "restore", revisionId?: string) => {
  if (busy.value || historyTransitionPending.value || !requireOpenProject()) return;
  if ((kind === "undo" && !canUndo.value) || (kind === "redo" && !canRedo.value)) return;
  if (tutorialActive.value) {
    if (kind === "undo") tutorialWorkspace.undo();
    else if (kind === "redo") tutorialWorkspace.redo();
    else { notify(() => t("shellTutorialNoRevisionRestore")); return; }
    clearWorkspaceSelection(); syncTutorialSummary();
    notify(() => t(kind === "undo" ? "shellTutorialUndo" : "shellTutorialRedo"));
    return;
  }
  const projectId = projectSnapshot.value!.project.project_id;
  const bindingId = agentWorkspace.binding.value?.binding_id;
  const sameProject = () => projectSnapshot.value?.project.project_id === projectId && agentWorkspace.binding.value?.binding_id === bindingId;
  const actionLabel = () => kind === "undo" ? t("undo") : kind === "redo" ? t("redo") : t("restoreVersion");
  let acquiredBusy = false;
  historyTransitionPending.value = true;
  try {
    if (!await guardWorkspaceDrafts()) return;
    // Only an actual Edit session needs to leave its mode; history browsing and Order retain their mode.
    if (editSession.value && !await guardBodyProjectChange()) return;
    if (!sameProject()) { notify(() => t("shellActionRetry", { p0: actionLabel() })); return; }
    if (kind === "restore") {
      const approved = isTauriRuntime
        ? await confirm(t("restoreWarning"), { title: t("restoreConfirm", { p0: revisionId ?? "" }), kind: "warning" })
        : window.confirm(t("restoreConfirm", { p0: revisionId ?? "" }));
      if (!approved) return;
    }
    if (busy.value || !sameProject()) return;
    busy.value = true; acquiredBusy = true;
    // A research draft save may commit before its revision event reaches the UI.
    const latest = await kernelClient.getCurrentProject();
    if (!sameProject() || latest.project.project_id !== projectId) { notify(() => t("shellActionRetry", { p0: actionLabel() })); return; }
    if (hasWorkspaceDraft.value || dirty.value) { notify(() => t("shellNewDraftRetry", { p0: actionLabel() })); return; }
    await applySnapshot(latest);
    if ((kind === "undo" && !canUndo.value) || (kind === "redo" && !canRedo.value)) { notify(() => t("shellActionUnavailable", { p0: actionLabel() })); return; }
    if (kind === "restore" && !latest.revisions.some(revision => revision.revision_id === revisionId)) { notify(() => t("shellRestoreUnavailable")); return; }
    const snapshot = kind === "undo" ? await kernelClient.undo() : kind === "redo" ? await kernelClient.redo() : await kernelClient.restoreRevision(revisionId!);
    await applySnapshot(snapshot);
    const restoredRevisionId = revisionId;
    notify(kind === "restore" ? () => t("revisionRestored", { p0: restoredRevisionId ?? "" }) : () => t("shellActionSaved", { p0: actionLabel() }));
    if (kind === "restore") await selectHistoryRevision(revisionId!);
  } catch (error) { const detail = errorMessage(error); notify(() => t("shellActionFailed", { p0: actionLabel(), p1: formatError(detail) })); }
  finally { if (acquiredBusy) busy.value = false; historyTransitionPending.value = false; }
};
const performUndo = () => performHistoryChange("undo");
const performRedo = () => performHistoryChange("redo");
const comparisonToDiff = (comparison: RevisionComparison): HistoryDiffState => {
  const sourceDocumentId = projectSnapshot.value?.documents[0]?.document_id;
  const targetDocumentId = projectSnapshot.value?.documents[1]?.document_id;
  const sourceChange = comparison.segment_changes.find((change) => (change.before ?? change.after)?.document_id === sourceDocumentId);
  const targetChange = comparison.segment_changes.find((change) => (change.before ?? change.after)?.document_id === targetDocumentId);
  return {
    segmentId: (sourceChange ?? targetChange) ? segmentLabel((sourceChange ?? targetChange)!.segment_id) : "",
    sourceOld: sourceChange?.before?.content ?? "", sourceNew: sourceChange?.after?.content ?? "",
    targetOld: targetChange?.before?.content ?? "", targetNew: targetChange?.after?.content ?? "",
    deletedLines: comparison.segment_changes.filter((change) => change.before).length,
    addedLines: comparison.segment_changes.filter((change) => change.after).length,
    summaryCounts: {
      segments: comparison.segment_changes.length,
      order: comparison.order_changes.length,
      alignments: comparison.alignment_changes.length,
    },
  };
};
const compareHistory = async (fromRevisionId: string, toRevisionId: string) => {
  if (fromRevisionId === toRevisionId) return;
  busy.value = true;
  try { baseRevisionId.value = fromRevisionId; selectedRevisionId.value = toRevisionId; historyDiffState.value = comparisonToDiff(await kernelClient.compareRevision(fromRevisionId, toRevisionId)); }
  catch (error) { const detail = errorMessage(error); notify(() => t("compareFailed", { p0: formatError(detail) })); }
  finally { busy.value = false; }
};
const selectHistoryRevision = async (revisionId: string) => {
  const current = projectSummary.value.revision_id;
  const fallback = projectSnapshot.value?.revisions.find((revision) => revision.revision_id !== revisionId)?.revision_id;
  selectedRevisionId.value = revisionId;
  await compareHistory(revisionId === current ? (fallback ?? current) : current, revisionId);
};
const restoreHistory = (revisionId: string) => performHistoryChange("restore", revisionId);
const makeSearchRequest = (options: SearchQueryOptions) => {
  const snapshot = projectSnapshot.value;
  if (!snapshot) throw new Error(t("openProjectFirst"));
  return { project_id: snapshot.project.project_id, query: options.query, regex: options.regex, case_sensitive: options.caseSensitive, language_id: null, document_ids: options.side === "source" ? [snapshot.documents[0].document_id] : options.side === "target" ? [activeTargetDocument.value!.document_id] : null, base_revision_id: snapshot.project.current_revision_id };
};
const runSearch = async (options: SearchQueryOptions) => {
  busy.value = true;
  const searchProjectId = projectSnapshot.value?.project.project_id;
  try {
    const request = makeSearchRequest(options);
    if (isTauriRuntime) {
      const binding = agentWorkspace.binding.value?.binding_id;
      if (!binding) throw new Error(t("shellProjectConnectionPending"));
      const spec: SearchSpec = { query: request.query, regex: request.regex, case_sensitive: request.case_sensitive, language_id: request.language_id, document_ids: request.document_ids };
      const result = await agentClient.call<{ total: number }>("search.execute", { spec, expected_revision_id: request.base_revision_id, page_size: 1 }, binding);
      if (searchProjectId === projectSnapshot.value?.project.project_id) { const count = result.data.total; notify(() => t("searchFound", { p0: count })); }
      return;
    }
    const response = await kernelClient.searchSegments(request);
    await acceptAgentSearch({ spec: null, results: response });
    const count = response.hits.length; notify(() => t("searchFound", { p0: count }));
  } catch (error) { if (searchProjectId === projectSnapshot.value?.project.project_id) { searchResults.value = []; const detail = errorMessage(error); notify(() => t("searchFailed", { p0: formatError(detail) })); } }
  finally { busy.value = false; }
};
const currentReplaceRequest = (): ReplacePreviewRequest => ({ ...makeSearchRequest({ query: searchQuery.value, side: searchSide.value, regex: searchRegex.value, caseSensitive: searchCaseSensitive.value }), replacement: replacement.value });
const previewReplacement = async () => {
  const generation = ++replacePreviewGeneration;
  nativeReplacePreview.value = null; nativeReplaceRequest.value = null;
  replacePreviewLoading.value = true; replacePreviewError.value = null;
  try {
    const request = currentReplaceRequest();
    const response = await kernelClient.previewReplace(request);
    if (generation !== replacePreviewGeneration || request.project_id !== projectSnapshot.value?.project.project_id || request.base_revision_id !== projectSnapshot.value?.project.current_revision_id) return;
    const sourceIds = new Set(sourceRows.value.map(row => row.id));
    const items = response.items.map(item => ({ resultId: item.segment_id, before: item.before, after: item.after }));
    nativeReplaceRequest.value = request;
    nativeReplacePreview.value = { options: { query: request.query, side: searchSide.value, regex: request.regex, caseSensitive: request.case_sensitive }, replacement: request.replacement, resultIds: response.items.map(item => item.segment_id), source: items.filter(item => sourceIds.has(item.resultId)), target: items.filter(item => !sourceIds.has(item.resultId)) };
    const count = response.items.length; notify(() => t("replacePreviewCount", { p0: count }));
  } catch (error) { if (generation === replacePreviewGeneration) { const detail = rawErrorMessage(error); replacePreviewError.value = detail; notify(() => t("replacePreviewFailed", { p0: formatError(detail) })); } }
  finally { if (generation === replacePreviewGeneration) replacePreviewLoading.value = false; }
};
watch(() => [searchQuery.value, searchSide.value, searchRegex.value, searchCaseSensitive.value, replacement.value, projectSnapshot.value?.project.project_id, projectSnapshot.value?.project.current_revision_id], () => {
  replacePreviewGeneration++; nativeReplacePreview.value = null; nativeReplaceRequest.value = null; replacePreviewLoading.value = false; replacePreviewError.value = null;
}, { flush: "sync" });
const applyReplacement = async (preview: UiReplacePreview) => {
  const request = isTauriRuntime ? nativeReplaceRequest.value : currentReplaceRequest();
  if (!request || (isTauriRuntime && preview !== nativeReplacePreview.value)) { notify(() => t("shellReplacePreviewStale")); return; }
  busy.value = true;
  try { await applySnapshot(await kernelClient.applyReplace(request, preview.resultIds)); const count = preview.resultIds.length; notify(() => t("replaceApplied", { p0: count })); await runSearch({ query: searchQuery.value, side: searchSide.value, regex: searchRegex.value, caseSensitive: searchCaseSensitive.value }); }
  catch (error) { const detail = errorMessage(error); notify(() => t("replaceFailed", { p0: formatError(detail) })); }
  finally { busy.value = false; }
};
const jumpToSegment = async (segmentId: string, alignmentId: string | null = null) => {
  if (!await guardProjectChange()) return false;
  const snapshot = projectSnapshot.value;
  if (!tutorialActive.value && snapshot && snapshot.documents.length > 2) {
    const relation = snapshot.alignments.find(alignment => alignment.alignment_id === alignmentId);
    const targetSegmentId = relation?.target_segment_ids[0];
    const segment = snapshot.segments.find(segment => segment.segment_id === (targetSegmentId ?? segmentId));
    const targetId = segment?.document_id === snapshot.documents[0].document_id ? activeTargetDocument.value!.document_id : segment?.document_id;
    if (targetId) await openComparisonPair(targetId, alignmentId ?? undefined);
    comparisonOpen.value = false;
  }
  const resolvedAlignmentId = alignmentId ?? alignmentRows.value.find((alignment) => [...alignment.sourceIds, ...alignment.targetIds].includes(segmentId))?.id ?? "";
  selectedAlignmentId.value = resolvedAlignmentId;
  activeNav.value = "parallel";
  await nextTick();
  return parallelWorkspaceRef.value?.focusSegment(segmentId) ?? false;
};
const selectSearchResult = (result: SearchResult) => { void jumpToSegment(result.id, result.alignmentId ?? null); };
const toggleBookmark = async (segmentId: string, alignmentId: string | null) => {
  if (tutorialActive.value) {
    tutorialBookmarkIds.value = tutorialBookmarkIds.value.includes(segmentId)
      ? tutorialBookmarkIds.value.filter(id => id !== segmentId)
      : [...tutorialBookmarkIds.value, segmentId];
    const added = tutorialBookmarkIds.value.includes(segmentId); notify(() => t(added ? "shellTutorialBookmarkAdded" : "shellTutorialBookmarkRemoved"));
    return;
  }
  const existing = projectSnapshot.value?.bookmarks.find((bookmark) => bookmark.segment_id === segmentId);
  const segment = [...sourceRows.value, ...targetRows.value].find((candidate) => candidate.id === segmentId);
  const content = segment?.text.replace(/\s+/g, " ").trim() ?? t("currentSegment");
  try { await applySnapshot(existing ? await kernelClient.deleteBookmark(existing.bookmark_id) : await kernelClient.createBookmark(segmentId, alignmentId, t("bookmarkDefaultLabel", { p0: segmentLabel(segmentId), p1: content.slice(0, 48) }))); notify(() => t(existing ? "bookmarkRemoved" : "bookmarkAdded")); }
  catch (error) { const detail = errorMessage(error); notify(() => t("bookmarkFailed", { p0: formatError(detail) })); }
};
const openBookmark = (segmentId: string, alignmentId: string | null) => { void jumpToSegment(segmentId, alignmentId); };
const openAnnotationLink = (segmentId: string) => { void jumpToSegment(segmentId); };
const removeBookmark = async (bookmarkId: string) => {
  if (tutorialActive.value) { tutorialBookmarkIds.value = tutorialBookmarkIds.value.filter(id => id !== bookmarkId); notify(() => t("shellTutorialBookmarkRemoved")); return; }
  try { await applySnapshot(await kernelClient.deleteBookmark(bookmarkId)); notify(() => t("bookmarkRemoved")); } catch (error) { const detail = errorMessage(error); notify(() => t("bookmarkRemoveFailed", { p0: formatError(detail) })); }
};
const resetSearch = () => { searchQuery.value = ""; replacement.value = ""; searchResults.value = []; };
const copyHistoryValue = async (value: string) => { try { await navigator.clipboard.writeText(value); notify(() => t("copied")); } catch { notify(() => t("clipboardUnavailable")); } };
const toggleSideNav = () => { sideNavCollapsed.value = !sideNavCollapsed.value; localStorage.setItem("jueming-nav-collapsed", String(sideNavCollapsed.value)); };
const annotationRequest = (draft: AnnotationDraft) => ({ title: draft.title, body: draft.body, status: draft.status, linked_segment_ids: draft.links.map((link) => link.segmentId), alignment_id: selectedAlignmentId.value || null });
const createAnnotation = async (draft: AnnotationDraft, done: (saved: boolean) => void) => {
  const contextSegment = annotationContextSegmentId.value
    ? [...sourceRows.value, ...targetRows.value].find((segment) => segment.id === annotationContextSegmentId.value)
    : undefined;
  const alignment = alignmentRows.value.find((item) => item.id === selectedAlignmentId.value);
  const fallbackLinks = contextSegment
    ? [{ side: contextSegment.side, segmentId: contextSegment.id }]
    : [...(alignment?.sourceIds ?? []).map((segmentId) => ({ side: "source" as const, segmentId })), ...(alignment?.targetIds ?? []).map((segmentId) => ({ side: "target" as const, segmentId }))];
  const linked = draft.links.length ? draft : { ...draft, links: fallbackLinks };
  if (tutorialActive.value) {
    tutorialAnnotations.value = [...tutorialAnnotations.value, { id: `tutorial-annotation-${Date.now()}`, number: tutorialAnnotations.value.length + 1, status: linked.status, title: linked.title, body: linked.body, createdAt: "", links: linked.links.map(link => ({ ...link })) }];
    notify(() => t("shellTutorialAnnotationAdded")); done(true); return;
  }
  try { await applySnapshot(await kernelClient.createAnnotation(annotationRequest(linked), draft.commandContext)); notify(() => t("annotationCreated")); done(true); }
  catch (error) { const detail = errorMessage(error); notify(() => t("annotationCreateFailed", { p0: formatError(detail) })); done(false); }
};
const editAnnotation = async (annotationId: string, draft: AnnotationDraft, done: (saved: boolean) => void) => {
  if (tutorialActive.value) { tutorialAnnotations.value = tutorialAnnotations.value.map(annotation => annotation.id === annotationId ? { ...annotation, title: draft.title, body: draft.body, status: draft.status, links: draft.links.map(link => ({ ...link })) } : annotation); notify(() => t("shellTutorialAnnotationUpdated")); done(true); return; }
  try { await applySnapshot(await kernelClient.updateAnnotation(annotationId, annotationRequest(draft), draft.commandContext)); notify(() => t("annotationUpdated")); done(true); } catch (error) { const detail = errorMessage(error); notify(() => t("annotationUpdateFailed", { p0: formatError(detail) })); done(false); }
};
const deleteAnnotation = async (annotationId: string) => {
  if (tutorialActive.value) { tutorialAnnotations.value = tutorialAnnotations.value.filter(annotation => annotation.id !== annotationId).map((annotation, index) => ({ ...annotation, number: index + 1 })); notify(() => t("shellTutorialAnnotationDeleted")); return; }
  const approved = "__TAURI_INTERNALS__" in window ? await confirm(t("annotationDeleteWarning"), { title: t("annotationDeleteConfirm"), kind: "warning" }) : window.confirm(t("annotationDeleteConfirm")); if (!approved) return; try { await applySnapshot(await kernelClient.deleteAnnotation(annotationId)); notify(() => t("annotationDeleted")); } catch (error) { const detail = errorMessage(error); notify(() => t("annotationDeleteFailed", { p0: formatError(detail) })); }
};
const resolveAnnotation = async (annotationId: string) => {
  if (tutorialActive.value) { tutorialAnnotations.value = tutorialAnnotations.value.map(annotation => annotation.id === annotationId ? { ...annotation, status: "resolved" } : annotation); notify(() => t("shellTutorialAnnotationResolved")); return; }
  try { await applySnapshot(await kernelClient.resolveAnnotation(annotationId)); notify(() => t("annotationResolved")); } catch (error) { const detail = errorMessage(error); notify(() => t("annotationResolveFailed", { p0: formatError(detail) })); }
};
const exportProject = async (format: ExportFormat) => { const outputPath = await saveDialog({ title: t("exportDialog", { p0: format.toUpperCase() }), defaultPath: `${projectSummary.value.name}.${format}`, filters: [{ name: format.toUpperCase(), extensions: [format] }] }); if (!outputPath) return; busy.value = true; try { await kernelClient.exportProject(format, outputPath); const path = outputPath; notify(() => t("exported", { p0: path })); } catch (error) { const detail = errorMessage(error); notify(() => t("exportFailed", { p0: formatError(detail) })); } finally { busy.value = false; } };
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
const discardPendingTransition = async () => {
  const mode = pendingTransition.value?.mode;
  const nav = pendingNav.value;
  if (await confirmPendingWithDiscard()) finishPendingTransition(mode, nav);
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
const exportSelect = (event: Event) => {
  const select = event.target as HTMLSelectElement;
  const format = select.value;
  // Export is an action: reset so the same format can be chosen again after cancel.
  select.value = "";
  if (format === "txt" || format === "json" || format === "xml") void exportProject(format);
};
const modeSelect = (event: Event) => {
  const select = event.target as HTMLSelectElement;
  const mode = select.value as ModeMenuId;
  if (mode === "annotation") openAnnotationPanel();
  else setMode(mode);
  select.value = activeMode.value;
};
const handleShortcut = (event: KeyboardEvent) => {
  if (event.isComposing || event.key === "Process") return;
  const key = event.key.toLowerCase();
  if (key === "escape" && (event.target as HTMLElement | null)?.tagName === "SELECT") return;
  if (key === "escape" && activeMode.value === "edit" && editSession.value) { event.preventDefault(); void escapeSegmentEdit(); return; }
  const primaryModifier = usesMacShortcuts.value ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
  if (!primaryModifier || event.altKey) return;
  const target = event.target as HTMLElement | null;
  const editingText = target?.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target?.tagName ?? "");
  if (key === "n" && !event.shiftKey) { event.preventDefault(); openNewProject(); return; }
  if (key === "o" && !event.shiftKey) { event.preventDefault(); void openProject(); return; }
  if (key === "s" && !event.shiftKey) { event.preventDefault(); void saveProject(); return; }
  if (key === "f" && event.shiftKey) { event.preventDefault(); setNav("search"); return; }
  if (key === "f" && !event.shiftKey && activeMode.value === "review" && activeNav.value === "parallel") { event.preventDefault(); void (showComparison.value ? comparisonWorkspaceRef.value : parallelWorkspaceRef.value)?.openFind(); return; }
  if (key === "g" && activeMode.value === "review" && activeNav.value === "parallel") { event.preventDefault(); void (showComparison.value ? comparisonWorkspaceRef.value : parallelWorkspaceRef.value)?.navigateFind(event.shiftKey ? -1 : 1); return; }
  if (editingText) return;
  if (key === "z" && !event.shiftKey) { event.preventDefault(); void performUndo(); }
  else if ((usesMacShortcuts.value && key === "z" && event.shiftKey) || (!usesMacShortcuts.value && (key === "y" || (key === "z" && event.shiftKey)))) { event.preventDefault(); void performRedo(); }
};
onMounted(() => window.addEventListener("keydown", handleShortcut));
onBeforeUnmount(() => { agentWorkspace.dispose(); unlistenRuntime?.(); unlistenRuntimeResync?.(); runtimeHistoryGeneration++; window.removeEventListener("keydown", handleShortcut); stopAnnotationResize(); unlistenClose?.(); disposeSettings(); void flushSettings(); });
</script>

<template>
  <div class="app-shell" :class="{ 'app-shell--tutorial': tutorialActive }">
    <header class="app-toolbar">
      <div class="toolbar-left">
        <button class="toolbar-button" type="button" :disabled="tutorialActive" :title="t('newProjectShortcut', { p0: shortcutLabels.newProject })" @click="openNewProject"><FilePlus2 :size="19" />{{ t('new') }}</button>
        <button class="toolbar-button" type="button" :disabled="tutorialActive" :title="t('openProjectShortcut', { p0: shortcutLabels.openProject })" @click="openProject"><FolderOpen :size="19" />{{ t('open') }}</button>
        <button class="toolbar-button" type="button" :title="t('saveProjectShortcut', { p0: shortcutLabels.save })" @click="saveProject"><Save :size="19" />{{ t('save') }}</button>
        <div class="toolbar-divider"></div>
        <div class="toolbar-actions">
          <label class="toolbar-export"><Download :size="19" aria-hidden="true" /><select :aria-label="t('exportProject')" :disabled="busy || tutorialActive" value="" @change="exportSelect"><option value="" disabled hidden>{{ t('export') }}</option><option value="txt">{{ t('exportTxt') }}</option><option value="json">{{ t('exportJson') }}</option><option value="xml">{{ t('exportXml') }}</option></select></label>
          <div class="toolbar-divider"></div>
          <button class="toolbar-button" data-tutorial="history-buttons" type="button" :disabled="!canUndo || busy || historyTransitionPending" :title="t('undoShortcut', { p0: shortcutLabels.undo })" @click="performUndo"><Undo2 :size="19" />{{ t('undo') }}</button>
          <button class="toolbar-button" type="button" :disabled="!canRedo || busy || historyTransitionPending" :title="t('redoShortcut', { p0: shortcutLabels.redo })" @click="performRedo"><Redo2 :size="19" />{{ t('redo') }}</button>
          <div class="toolbar-divider"></div>
          <button class="toolbar-button" type="button" :disabled="tutorialActive" @click="setNav('settings')"><Settings2 :size="19" />{{ t('navSettings') }}</button>
        </div>
      </div>
      <div class="global-panel-buttons">
        <button class="toolbar-button toolbar-button--help" data-tutorial="help-button" type="button" :aria-label="tutorialActive ? t('shellHelpReopen') : t('shellHelpOpen')" :title="t('shellHelpTitle')" :aria-pressed="tutorialGuideOpen" @click="startTutorial"><HelpCircle :size="18" /></button>
        <button v-if="tutorialActive" class="toolbar-button" type="button" :aria-label="t('shellExitDemoAria')" :title="t('shellExitDemoTitle')" @click="closeTutorial"><X :size="18" />{{ t('shellExitDemo') }}</button>
        <button class="toolbar-button" type="button" :disabled="tutorialActive" :aria-pressed="rightPanelOpen && rightPanelTab === 'agent'" @click="assistantOpen = true; rightPanelTab = 'agent'"><Sparkles :size="18" />{{ t('assistant') }}<span v-if="agentProposals.length" class="pending-count">{{ agentProposals.length }}</span></button>
        <button class="toolbar-button" type="button" :aria-pressed="rightPanelOpen && rightPanelTab === 'annotations'" @click="openAnnotationPanel()"><MessageSquareText :size="18" />{{ t('annotations') }}</button>
      </div>
      <div class="mode-control"><select :value="activeMode" :aria-label="t('switchMode')" @change="modeSelect"><option v-for="item in modeMenuItems" :key="item.id" :value="item.id" :disabled="tutorialActive && item.id === 'history'">{{ item.label }} {{ item.hint }}</option></select></div>
    </header>
    <div class="content-grid" :class="{ 'content-grid--collapsed': sideNavCollapsed, 'content-grid--annotation': rightPanelOpen, 'content-grid--resizing': annotationResizing }" :style="{ '--annotation-width': `${annotationWidth}px` }">
      <nav class="side-nav" :class="{ 'side-nav--collapsed': sideNavCollapsed }"><button v-for="item in visibleNavItems" :key="item.id" :data-nav-id="item.id" class="nav-item" :class="{ 'nav-item--active': activeNav === item.id }" type="button" :disabled="tutorialActive && item.id !== 'parallel'" :title="item.label" @click="setNav(item.id)"><component :is="item.icon" :size="22" :stroke-width="activeNav === item.id ? 2.2 : 1.8" /><span class="nav-label">{{ item.label }}<small v-if="item.hint">{{ item.hint }}</small></span></button><CompanionHabitat :in-garden="gardenStrolling" v-if="gardenVisible && !tutorialActive && !sideNavCollapsed" :settings="settings.device.pet" :activity="companionActivity" :dirty-editor="dirty" :motion="effectiveMotionMode" /><div class="nav-collapse"><button class="nav-item" type="button" :title="sideNavCollapsed ? t('expandSidebar') : t('collapseSidebar')" @click="toggleSideNav"><ChevronDown :size="21" :style="{ transform: sideNavCollapsed ? 'rotate(-90deg)' : 'rotate(90deg)' }" /><span class="nav-label">{{ sideNavCollapsed ? t('expand') : t('collapse') }}</span></button></div></nav>
      <main class="main-stage">
        <PipelineWorkspace v-if="pipelineVisited" :active="activeNav === 'pipeline'" ref="pipelineWorkspaceRef" v-show="activeNav === 'pipeline'" :binding-id="agentWorkspace.binding.value?.binding_id ?? null" :project-id="projectSnapshot?.project.project_id ?? null" :revision-id="projectSummary.revision_id" :source-segments="sourceRows" :refresh-key="pipelineRefreshKey" :available="isTauriRuntime" @status="notify" @dirty="pipelineDirty = $event" />
        <ResearchWorkspace v-if="researchVisited" ref="researchWorkspaceRef" v-show="activeNav === 'research'" :active="activeNav === 'research'" :binding-id="agentWorkspace.binding.value?.binding_id ?? null" :project-id="projectSnapshot?.project.project_id ?? null" :revision-id="projectSummary.revision_id" :available="isTauriRuntime" :feature="researchFeature" :translation-label="projectSummary.target_label" :before-group-change="guardWorkspaceDrafts" @status="notify" @dirty="researchDirty = $event" @reveal="revealResearchSegment" @pipeline="setNav('pipeline')" @settings="openResearchSettings" />
        <div v-if="activeNav === 'parallel'" class="parallel-stage">
          <ComparisonWorkspace ref="comparisonWorkspaceRef" :find-in-view="workspaceText.find" v-if="showComparison && projectSnapshot" :project="projectSnapshot" :texts="workspaceText.texts.value" :writable="workspaceWritable && !busy" @visible-segments="loadVisibleSegments" @edit-pair="openComparisonPair" @unlink="unlinkAlignment" />
          <div v-if="!tutorialActive && hasMultipleTranslations && !showComparison" class="comparison-return"><button class="secondary-button" type="button" @click="returnToComparison">{{ t('shellReturnComparison') }}</button><span>{{ projectSnapshot?.documents[0].title }} ↔ {{ activeTargetDocument?.title }}</span></div>
        <ParallelWorkspace :source-title="tutorialActive ? t('shellSourceSegments') : projectSnapshot?.documents[0].title" :target-title="tutorialActive ? t('targetSegments') : activeTargetDocument?.title" :revision-key="tutorialActive ? `tutorial-${tutorialWorkspace.revision.value}` : projectSnapshot?.project.current_revision_id" :find-in-view="!tutorialActive && projectSnapshot ? findCurrentPair : undefined" :ensure-segments="tutorialActive ? undefined : workspaceText.ensure" @visible-segments="loadVisibleSegments" @selection-context="workspaceOperationSelection = $event" ref="parallelWorkspaceRef" v-if="!showComparison" :mode="activeMode" :source-segments="sourceRows" :target-segments="targetRows" :alignments="alignmentRows" :selected-alignment-id="selectedAlignmentId" :bookmarked-segment-ids="bookmarkedSegmentIds" :annotated-segment-ids="annotatedSegmentIds" :edit-session="editSession" :trackpad-optimized="trackpadOptimized" :smooth-navigation="smoothNavigationEnabled" :writable="workspaceWritable" :readonly-action-label="!workspaceWritable ? t('shellEnterDemo') : ''" @readonly-action="startTutorial" @select="selectedAlignmentId = $event" @request-edit="requestSegmentEdit" @edit-draft="updateDraft" @commit-edit="commitSegmentEdit" @cancel-edit="cancelSegmentEdit" @escape-edit="escapeSegmentEdit" @move="moveSegment" @reorder="reorderSegment" @insert-gap="insertAlignmentGap" @reset-order="resetOrder" @link="linkSegments" @unlink="unlinkAlignment" @merge-segments="mergeSegmentContent" @split-segment="splitSegmentContent" @group="groupAlignments" @ungroup="ungroupAlignment" @bookmark="toggleBookmark" @annotation="openAnnotationPanel" @status="notify" />
        </div>
        <SearchReplaceWorkspace :authoritative-results="isTauriRuntime" :native-preview="nativeReplacePreview" :preview-loading="replacePreviewLoading" :preview-error="displayedReplacePreviewError" v-else-if="activeNav === 'search'" v-model:query="searchQuery" v-model:side="searchSide" v-model:regex="searchRegex" v-model:case-sensitive="searchCaseSensitive" v-model:replacement="replacement" :results="searchResults" :project-label="projectSummary.name" :loading="busy" @search="runSearch" @select-result="selectSearchResult" @replace-preview="previewReplacement" @apply-replace="applyReplacement" @reset="resetSearch" />
        <HistoryWorkspace v-else-if="activeNav === 'history'" :revisions="revisionItems" :diff="historyDiff" :base-revision-id="baseRevisionId" :selected-revision-id="selectedRevisionId" :current-revision-id="projectSummary.revision_id" :loading="busy || historyTransitionPending" @select-revision="selectHistoryRevision" @compare="compareHistory" @restore="restoreHistory" @copy-value="copyHistoryValue" />
        <SettingsWorkspace
          v-else-if="activeNav === 'settings'"
          v-model:settings="settings"
          :capabilities="capabilities"
          :research-feature="researchFeature"
          :research-available="isTauriRuntime"
          :research-busy="researchFeatureBusy"
          @research-enable="researchFeatureAction('enable')"
          @research-disable="researchFeatureAction('disable')"
          @research-cancel="researchFeatureAction('cancel')"
          @research-retry="researchFeatureAction('retry')"
          @research-open="setNav('research')"
          @research-preferences="updateResearchPreferences"
          :uses-mac-shortcuts="usesMacShortcuts"
          :shortcut-rows="shortcutRows"
          :cache-cleaning="cacheCleaning"
          :last-cache-cleanup-at="lastCacheCleanupAt"
          :settings-saving="settingsSaving"
          :settings-save-error="settingsSaveError"
          :effective-motion-mode="effectiveMotionMode"
          :motion-summary="motionSummary"
          :system-reduced-motion="systemReducedMotion"
          :project-open="Boolean(projectSnapshot)"
          app-version="0.1.1"
          @apply-general="applyGeneralSettings"
          @apply-ui="applyUiSettings"
          @apply-accessibility="applyAccessibilitySettings"
          @apply-interaction="applyInteractionSettings"
          @apply-motion="applyMotionSettings"
          @apply-persistence="applyPersistenceSettings"
          @clear-cache="clearProjectCache(true)"
          @remember-section="rememberSettingsSection"
          @reset-all="resetAllSettings"
        >
          <template #agent-settings><AgentConnectionSettings /><AgentModelSettings @changed="runtimeStatus = $event" /></template>
        </SettingsWorkspace>
        <BookmarksWorkspace v-else-if="activeNav === 'bookmarks'" :bookmarks="projectSnapshot?.bookmarks ?? []" :previews="bookmarkPreviews" :source-segments="sourceRows" :target-segments="targetRows" :alignments="alignmentRows" :segment-labels="segmentLabels" :alignment-labels="alignmentLabels" @open="openBookmark" @remove="removeBookmark" />
        <section v-else-if="activeNav === 'project'" class="aux-view project-view"><Folder :size="28" /><h2>{{ t('shellProjectTitle') }}</h2><p>{{ projectSnapshot ? projectSummary.name : t('projectNone') }}</p><div v-if="projectSnapshot" class="project-summary-card"><span>{{ projectSummary.source_count }}</span><small>{{ t('shellSourceSegments') }}</small><span>{{ projectSummary.target_count }}</span><small>{{ t('shellFirstTargetSegments') }}</small><span>{{ projectSummary.alignment_count }}</span><small>{{ t('alignment') }}</small></div><div class="project-actions"><button type="button" class="primary-button" @click="openProject">{{ t('openProject') }}</button><button type="button" class="secondary-button" @click="openNewProject">{{ t('newProject') }}</button><button v-if="!projectSnapshot" type="button" class="secondary-button" @click="startTutorial">{{ t('shellEnterDemo') }}</button></div></section>
      </main>
      <aside class="annotation-drawer" :class="{ 'annotation-drawer--open': rightPanelOpen }" :aria-hidden="!rightPanelOpen" :inert="!rightPanelOpen">
        <div class="annotation-resize-handle" role="separator" tabindex="0" :aria-label="t('shellResizeGlobalPanel')" aria-orientation="vertical" :aria-valuemin="annotationMinimumWidth" :aria-valuemax="annotationMaximumWidth()" :aria-valuenow="Math.round(annotationWidth)" :title="t('shellResizeGlobalPanelHint')" @pointerdown="startAnnotationResize" @dblclick="resetAnnotationWidth" @keydown="resizeAnnotationWithKeyboard"></div>
        <GlobalSidePanel v-model:active-tab="rightPanelTab" :project-label="tutorialActive ? t('shellTutorialProject') : projectSnapshot ? projectSummary.name : t('projectNone')" :annotation-count="annotations.length" :pending-count="agentProposals.length" @close="closeAnnotationPanel">
          <template #agent><AgentPanel v-model:draft="agentDraft" :tab-label="visibleNavItems.find(item => item.id === activeNav)?.label ?? activeNav" :selected-text="agentWorkspace.context.value?.selected_text ?? ''" :connected="Boolean(agentWorkspace.binding.value)" :runtime-available="Boolean(runtimeStatus?.configured && projectSnapshot)" :running="agentBusy || runtimeRunning" :reviewing="agentBusy" :show-context="settings.device.agent.showContext" :messages="runtimeMessages.filter(message => message.role !== 'tool').map(message => ({ id: message.message_id, role: message.role, content: message.content }))" :activity="agentActivity" :proposals="agentProposals" :error="displayedAgentActionError" @settings="openAgentSettings" @send="sendAgentMessage" @cancel="cancelAgentRun" @clear-selection="agentWorkspace.clearSelectionContext()" @approve="reviewAgentProposal($event, true)" @reject="reviewAgentProposal($event, false)" /></template>
          <template #annotations><AnnotationPanel :command-scope="commandScope()" v-model:active-filter="annotationFilter" :scope-key="tutorialActive ? 'tutorial' : agentProjectKey" :annotations="annotations" :selected-id="selectedAnnotationId" :readonly="(!projectSnapshot && !tutorialActive) || activeNav === 'history'" @select="selectedAnnotationId = $event" @open-link="openAnnotationLink" @create="createAnnotation" @edit="editAnnotation" @delete="deleteAnnotation" @resolve="resolveAnnotation" @close="closeAnnotationPanel" /></template>
        </GlobalSidePanel>
      </aside>
      <CompanionGarden v-model:strolling="gardenStrolling" v-if="gardenVisible && !tutorialActive" :settings="settings.device.pet" :activity="companionActivity" :dirty-editor="dirty" :motion="effectiveMotionMode" :proposal="agentProposals[0] ?? null" :busy="agentBusy" :guidance="guidance?.label ?? ''" @approve="reviewAgentProposal($event, true)" @reject="reviewAgentProposal($event, false)" @open-agent="assistantOpen = true; rightPanelTab = 'agent'" />
    </div>
    <GuidanceOverlay :target="guidance" :animated="effectiveMotionMode === 'standard' && settings.device.pet.presentation === 'animated'" :visible="!tutorialActive && settings.device.pet.butterflyMotion && settings.device.pet.presentation !== 'hidden'" />
    <ParallelTutorial :open="tutorialActive && tutorialGuideOpen" :step="tutorialStep" :steps="parallelTutorialSteps" @update:step="prepareTutorialStep" @reset="resetTutorial" @dismiss="dismissTutorialGuide" @close="closeTutorial" />
    <footer class="bottom-status">
      <div class="footer-context">
        <button v-if="gardenVisible && !tutorialActive" class="garden-status-button" type="button" :class="{ 'garden-status-button--open': gardenOpen }" :aria-label="gardenOpen ? t('shellGardenClose') : t('shellGardenOpen')" :title="gardenOpen ? t('shellGardenClose') : t('shellGardenOpen')" :aria-pressed="gardenOpen" @click="gardenStrolling = !gardenStrolling">
          <img :src="gardenPawIcon" alt="" aria-hidden="true" />
        </button>
        <div class="footer-context-item footer-context-item--project">
          <strong class="footer-context-label">{{ t('shellFooterProject') }}</strong>
          <span class="footer-ellipsis" :title="projectSummary.name">{{ projectSummary.name }}</span>
        </div>
        <div class="footer-context-separator" aria-hidden="true"></div>
        <div class="footer-context-item footer-context-item--files">
          <strong class="footer-context-label">{{ t('shellFooterFiles') }}</strong>
          <span class="footer-file-name footer-file-name--source" :title="projectSummary.source_label">{{ projectSummary.source_label }}</span>
          <Link2 :size="13" aria-hidden="true" />
          <span class="footer-file-name footer-file-name--target" :title="projectSummary.target_label">{{ projectSummary.target_label }}</span>
        </div>
        <div class="footer-context-separator" aria-hidden="true"></div>
        <div class="footer-context-item footer-context-item--alignment">
          <span class="footer-context-label">{{ t('shellFooterAlignment') }}</span>
          <span class="footer-status-pill">{{ alignmentStatus }}</span>
        </div>
      </div>
      <div class="footer-save-state">
        <span :key="footerStatusMessage" class="save-state" :class="{ 'save-state--dirty': dirty }" :title="footerStatusMessage">
          <Monitor :size="14" aria-hidden="true" />
          <span class="footer-ellipsis">{{ footerStatusMessage }}</span>
        </span>
      </div>
      <div class="footer-progress">
        <span class="footer-processed"><span class="footer-processed-label">{{ t('shellFooterProcessed') }}</span><strong>{{ projectSummary.alignment_count }} / {{ processedTotal }}</strong></span>
        <span class="footer-progress-label">{{ t('shellFooterProgress') }}</span>
        <div class="progress-track" role="progressbar" :aria-label="t('shellFooterProgressAria')" aria-valuemin="0" aria-valuemax="100" :aria-valuenow="progressPercentage"><span :style="{ width: `${progressPercentage}%` }"></span></div>
        <strong class="footer-progress-percent">{{ progressPercentage }}%</strong>
      </div>
    </footer>
    <div v-if="pendingTransition" class="modal-backdrop mode-guard-backdrop"><section class="mode-guard" role="dialog" aria-modal="true" aria-labelledby="mode-guard-title"><span class="eyebrow">{{ t('unsavedEyebrow') }}</span><h2 id="mode-guard-title">{{ t('unsavedTitle') }}</h2><p>{{ t('unsavedDescription') }}</p><div><button class="secondary-button" type="button" @click="stayInEdit">{{ t('keepEditing') }}</button><button class="secondary-button danger-button" type="button" @click="discardPendingTransition">{{ t('discardDraft') }}</button><button class="primary-button" type="button" @click="savePendingTransition"><Check :size="15" />{{ t('saveAndSwitch') }}</button></div></section></div>
    <div v-if="workspaceLeave" class="modal-backdrop mode-guard-backdrop"><section class="mode-guard" role="dialog" aria-modal="true" aria-labelledby="workspace-guard-title"><h2 id="workspace-guard-title">{{ t('shellWorkspaceDraftTitle', { p0: workspaceLeave.label() }) }}</h2><p>{{ t('shellWorkspaceDraftDescription') }}</p><div><button class="secondary-button" type="button" :disabled="workspaceLeaveBusy" @click="resolveWorkspaceLeave('stay')">{{ t('keepEditing') }}</button><button class="secondary-button danger-button" type="button" :disabled="workspaceLeaveBusy" @click="resolveWorkspaceLeave('discard')">{{ t('discardDraft') }}</button><button class="primary-button" type="button" :disabled="workspaceLeaveBusy" @click="resolveWorkspaceLeave('save')">{{ workspaceLeaveBusy ? t('shellSaving') : t('shellSaveAndContinue') }}</button></div></section></div>
    <NewProjectDialog ref="newProjectDialogRef" v-model:busy="busy" :kernel-client="kernelClient" :before-create="guardProjectChange" @created="handleProjectCreated" @status="notify" />
  </div>
</template>

<style scoped>
.annotation-drawer { position: relative; z-index: 6; width: var(--annotation-width); height: 100%; min-width: 0; opacity: 0; visibility: hidden; transform: translateX(100%); transition: opacity 180ms ease, transform 240ms cubic-bezier(.2, .8, .2, 1), visibility 0s linear 240ms; }
.annotation-drawer--open { opacity: 1; visibility: visible; transform: translateX(0); transition-delay: 0s; }
.content-grid--resizing .annotation-drawer { transition: none; }
.annotation-resize-handle { position: absolute; z-index: 8; top: 16px; bottom: 16px; left: 0; width: 1px; background: #ccd9cd; cursor: col-resize; touch-action: none; }
.annotation-resize-handle::before { position: absolute; inset: 0 -5px; content: ""; }
.annotation-resize-handle:hover, .annotation-resize-handle:focus-visible, .content-grid--resizing .annotation-resize-handle { background: var(--green-700); box-shadow: 0 0 0 1px rgb(47 129 67 / 12%); }
.search-view { display: flex; flex-direction: column; height: 100%; min-height: 0; padding: 25px 28px; }.search-bar { display: flex; align-items: center; gap: 10px; }.search-bar > label { width: 42px; font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }.search-input-wrap { display: flex; align-items: center; width: min(570px, 48vw); height: 42px; padding: 0 12px; border: 1px solid #c9d1ca; border-radius: 7px 0 0 7px; color: var(--ink-500); }.search-input-wrap input { flex: 1; min-width: 0; padding: 0 9px; border: 0; outline: none; }.search-input-wrap button { padding: 3px; border: 0; background: transparent; color: var(--ink-500); cursor: pointer; }.primary-search, .primary-button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; height: 42px; padding: 0 22px; border: 1px solid var(--green-900); border-radius: 0 6px 6px 0; color: #fff; background: var(--green-900); cursor: pointer; }.filter-button { height: 42px; margin-left: -10px; padding: 0 12px; border: 1px solid #c9d1ca; border-radius: 0 7px 7px 0; background: #fff; cursor: pointer; }.check-label { display: inline-flex; align-items: center; gap: 6px; margin-left: 22px; color: var(--ink-700); font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }.current-project { display: inline-flex; align-items: center; gap: 4px; margin-left: auto; color: var(--green-900); font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }.search-summary { padding: 15px 5px 12px; color: var(--ink-700); font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }.search-summary strong { color: var(--green-900); }.search-summary span { color: var(--ink-500); }.search-table { min-height: 0; overflow: auto; border: 1px solid var(--line); border-radius: 7px; }.search-table-head, .search-row { display: grid; grid-template-columns: 85px 1.2fr 95px 1.45fr 90px; align-items: center; gap: 14px; padding: 0 17px; }.search-table-head { height: 43px; color: var(--ink-700); background: #f8faf8; font-size: var(--jm-font-size-callout); line-height: var(--jm-line-height-callout); }.search-row { width: 100%; min-height: 57px; border: 0; border-top: 1px solid var(--line); background: #fff; color: var(--ink-900); font-size: var(--jm-font-size-body); text-align: left; cursor: pointer; line-height: var(--jm-line-height-body); }.search-row:hover { background: #f5fbf3; }.search-row > span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }.search-row mark { width: max-content; padding: 2px 7px; color: #72591b; background: #fff1c9; font-weight: var(--jm-font-weight-semibold); }
.aux-view { display: flex; flex-direction: column; align-items: flex-start; justify-content: center; gap: 13px; max-width: 540px; height: 100%; margin: auto; padding: 32px; color: var(--green-900); }.aux-view h2 { margin: 0; color: var(--ink-900); font-size: var(--jm-font-size-title-2); line-height: var(--jm-line-height-title-2); font-weight: var(--jm-font-weight-regular); }.aux-view p { margin: 0; color: var(--ink-700); line-height: 1.6; }.secondary-button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; height: 38px; padding: 0 17px; border: 1px solid #bdcabf; border-radius: 6px; color: var(--ink-700); background: #fff; cursor: pointer; }.aux-view .primary-button { border-radius: 6px; }.project-summary-card { display: grid; grid-template-columns: auto auto; gap: 3px 14px; margin: 4px 0; padding: 14px 18px; border: 1px solid var(--line); border-radius: 8px; color: var(--ink-900); }.project-summary-card span { font-size: var(--jm-font-size-title-1); color: var(--green-900); line-height: var(--jm-line-height-title-1); }.project-summary-card small { color: var(--ink-500); font-size: var(--jm-font-size-subheadline); line-height: var(--jm-line-height-subheadline); }
.bookmark-list { display: grid; gap: 8px; width: min(560px, 72vw); max-height: 420px; overflow: auto; }.bookmark-list article { display: grid; grid-template-columns: 1fr 36px; align-items: center; border: 1px solid var(--line); border-radius: 7px; background: var(--paper); }.bookmark-list button { display: flex; align-items: center; gap: 11px; min-width: 0; padding: 11px 13px; border: 0; color: var(--green-900); background: transparent; text-align: left; cursor: pointer; }.bookmark-list button:last-child { justify-content: center; padding-inline: 8px; color: var(--ink-500); }.bookmark-list span { display: grid; gap: 3px; }.bookmark-list small { color: var(--ink-500); font-family: var(--jm-font-mono); }.bookmark-list em { overflow: hidden; color: var(--ink-700); font-size: var(--jm-font-size-callout); font-style: normal; line-height: 1.4; text-overflow: ellipsis; white-space: nowrap; }.empty-copy { padding: 20px; border: 1px dashed #c8d3c9; border-radius: 7px; }.project-actions { display: flex; align-items: center; gap: 9px; }.project-actions > button { height: 42px; }
.modal-backdrop { position: fixed; z-index: 20; inset: 0; display: grid; place-items: center; background: rgb(31 42 34 / 22%); }.eyebrow { color: var(--green-700); font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-semibold); letter-spacing: .1em; line-height: var(--jm-line-height-subheadline); }
.annotation-panel { position: fixed; z-index: 10; top: 116px; right: 0; bottom: 58px; display: flex; flex-direction: column; width: 355px; overflow: auto; border-left: 1px solid #ccd9cd; background: #fbfdfb; box-shadow: -12px 0 30px rgb(32 51 35 / 10%); }.annotation-panel header { display: flex; align-items: flex-start; justify-content: space-between; padding: 19px 18px 12px; }.annotation-panel header button { padding: 5px; border: 0; background: transparent; color: var(--ink-500); cursor: pointer; }.annotation-panel h2 { margin: 4px 0 0; color: var(--ink-900); font-size: var(--jm-font-size-title-3); line-height: var(--jm-line-height-title-3); font-weight: var(--jm-font-weight-semibold); }.annotation-panel h2 small { display: inline-block; margin-left: 3px; padding: 2px 6px; border-radius: 10px; color: var(--green-900); background: var(--green-100); font-size: var(--jm-font-size-subheadline); line-height: var(--jm-line-height-subheadline); }.annotation-filter { display: flex; gap: 4px; padding: 0 13px 13px; border-bottom: 1px solid var(--line); }.annotation-filter button { padding: 7px 8px; border: 1px solid transparent; border-radius: 5px; color: var(--ink-500); background: transparent; font-size: var(--jm-font-size-callout); cursor: pointer; line-height: var(--jm-line-height-callout); }.annotation-filter button.active { border-color: #b3d5b7; color: var(--green-900); background: #f3faf1; }.annotation-card { display: grid; grid-template-columns: 25px 1fr; gap: 7px; margin: 13px 13px 0; padding: 13px 11px; border: 1px solid #d3dde5; border-radius: 8px; background: #fff; }.annotation-card--draft { border-color: #d3c4ec; }.annotation-number { display: grid; place-items: center; width: 22px; height: 22px; border-radius: 6px; color: #fff; background: #9864d5; font-size: var(--jm-font-size-callout); line-height: var(--jm-line-height-callout); }.annotation-number--green { background: #3c9a5a; }.annotation-state { display: flex; align-items: center; gap: 7px; color: var(--ink-900); font-size: var(--jm-font-size-callout); font-weight: var(--jm-font-weight-semibold); line-height: var(--jm-line-height-callout); }.annotation-state span { color: var(--ink-500); font-weight: var(--jm-font-weight-medium); }.annotation-state time { margin-left: auto; color: var(--ink-500); font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-regular); line-height: var(--jm-line-height-subheadline); }.annotation-card h3 { margin: 12px 0 7px; color: var(--ink-900); font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); font-weight: var(--jm-font-weight-semibold); }.annotation-card p { margin: 0; color: var(--ink-700); font-size: var(--jm-font-size-body); line-height: 1.55; }.annotation-links { display: flex; flex-direction: column; gap: 5px; margin-top: 10px; padding: 8px; border-radius: 5px; background: #f8faf8; color: var(--ink-700); font-size: var(--jm-font-size-subheadline); line-height: var(--jm-line-height-subheadline); }.annotation-links b { float: right; color: var(--ink-500); font-weight: var(--jm-font-weight-medium); }.annotation-card footer { display: flex; gap: 15px; margin-top: 10px; padding-top: 9px; border-top: 1px solid var(--line); }.annotation-card footer button, .resolve-button, .new-annotation { display: inline-flex; align-items: center; gap: 5px; border: 0; color: var(--ink-700); background: transparent; font-size: var(--jm-font-size-callout); cursor: pointer; line-height: var(--jm-line-height-callout); }.resolve-button { margin-top: 11px; padding: 6px 9px; border: 1px solid #b5dab9; border-radius: 5px; color: var(--green-900); background: #f2faf1; }.new-annotation { justify-content: center; margin: 13px; padding: 10px; border: 1px solid #abd1af; border-radius: 6px; color: var(--green-900); background: #f4fbf2; }
.mode-guard-backdrop { z-index: 30; }.mode-guard { width: min(460px, calc(100vw - 60px)); padding: 24px; border: 1px solid #d6c58f; border-radius: 10px; background: #fff; box-shadow: 0 22px 70px rgb(29 48 32 / 23%); }.mode-guard h2 { margin: 5px 0 9px; color: var(--ink-900); font-size: var(--jm-font-size-title-2); line-height: var(--jm-line-height-title-2); font-weight: var(--jm-font-weight-regular); }.mode-guard p { margin: 0; color: var(--ink-700); font-size: var(--jm-font-size-body); line-height: 1.6; }.mode-guard > div { display: flex; justify-content: flex-end; gap: 9px; margin-top: 20px; }.mode-guard .primary-button { height: 38px; border-radius: 6px; }.danger-button { border-color: #dab7b7; color: #9b4e4e; }
</style>

<style scoped>
.parallel-stage > .workspace { flex: 1; height: auto; }
.parallel-stage { display: flex; flex-direction: column; height: 100%; min-height: 0; }
.comparison-return { display: flex; align-items: center; gap: 12px; padding: 8px 16px; border-bottom: 1px solid var(--line); background: var(--surface-subtle); }
.comparison-return span { color: var(--text-muted); font-size: 12px; }
</style>
