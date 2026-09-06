<script setup lang="ts">
import { computed, defineAsyncComponent, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { confirm, open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Check, ChevronDown, Download, Eye, FilePlus2, Folder, FolderOpen, History, Link2, ListOrdered, MessageSquareText, Monitor, PanelLeft, Pencil, Redo2, Save, Search, Settings2, Sparkles, Star, Undo2, Workflow } from "@lucide/vue";
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
import { pipelineClient } from "./domain/pipeline-client";
import type { PipelineMethodProposal } from "./domain/pipeline-types";
import CompanionHabitat from "./components/CompanionHabitat.vue";
import CompanionGarden from "./components/CompanionGarden.vue";
import GuidanceOverlay from "./components/GuidanceOverlay.vue";
import type { CompanionActivity } from "./domain/companion";
import type { ReviewableProposal } from "./components/AgentActionBar.vue";
import { useAgentWorkspace } from "./composables/useAgentWorkspace";
import { agentClient } from "./domain/agent-client";
import type { SearchSpec } from "./domain/agent-types";
import type { SearchSegmentsResponse } from "./domain/kernel-client";
import { useAppSettings } from "./composables/useAppSettings";
import type { ReorderIntent } from "./composables/useOrderDragAndDrop";
import { useViewModeController } from "./composables/useViewModeController";
import { useStructureMutations } from "./composables/useStructureMutations";
import { alignments as fixtureAlignments, sourceSegments, targetSegments } from "./data/governmentFixture";
import { createKernelClient, snapshotToWorkspace, type AlignmentDto, type BookmarkPreviewDto, type ExportFormat, type LanguageSide, type ProjectSnapshot, type ProjectSummaryDto, type ReplacePreviewRequest, type RevisionComparison, type SegmentDto, type WorkspaceMode } from "./domain/kernel-client";
import "./styles.css";

type NavId = "project" | "parallel" | "search" | "bookmarks" | "history" | "pipeline" | "settings";
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
const statusMessage = ref(isTauriRuntime ? "尚未打开工程" : "演示预览 · 浏览器模式");
const exportOpen = ref(false);
const annotationOpen = ref(false);
const assistantOpen = ref(false);
const rightPanelTab = ref<"agent" | "annotations">("agent");
const rightPanelOpen = computed(() => annotationOpen.value || assistantOpen.value);
const agentBusy = ref(false);
const agentActionError = ref<string | null>(null);
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
const pipelineProposals = ref<PipelineMethodProposal[]>([]);
const pipelineWorkspaceRef = ref<{ revealNode: (nodeId: string) => Promise<boolean>; reload: () => Promise<void>; hasDirtyDraft: () => boolean } | null>(null);
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
let replacePreviewGeneration = 0;
const workspaceOperationSelection = ref<OperationSelectionContext>({ segmentIds: [], alignmentIds: [] });
const annotationFilter = ref<AnnotationFilter>("all");
const selectedAnnotationId = ref<string | null>(null);
const selectedRevisionId = ref<string | null>(null);
const baseRevisionId = ref<string | null>(null);
const historyDiff = ref<HistoryDiff | null>(null);
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
const projectSummary = ref<ProjectSummaryDto>({ project_id: "fixture-project", name: "2024政府工作报告_中英对齐", source_label: "report_zh.txt", target_label: "report_en.txt", source_count: 8, target_count: 8, alignment_count: 8, source_unlinked_count: 0, target_unlinked_count: 0, revision_id: "0" });

const modeItems: Array<{ id: WorkspaceMode; label: string; hint: string; icon: typeof Eye }> = [
  { id: "review", label: "审阅排序", hint: "Review + Order", icon: Eye },
  { id: "edit", label: "编辑模式", hint: "Edit", icon: Pencil },
  { id: "order", label: "排序模式", hint: "Order", icon: ListOrdered },
  { id: "history", label: "历史模式", hint: "History", icon: History },
];
const annotationModeItem: { id: "annotation"; label: string; hint: string; icon: typeof Eye } = {
  id: "annotation",
  label: "批注模式",
  hint: "Annotation",
  icon: MessageSquareText,
};
const modeMenuItems: Array<{ id: ModeMenuId; label: string; hint: string; icon: typeof Eye }> = [
  ...modeItems,
  annotationModeItem,
];
const navItems: Array<{ id: NavId; label: string; hint?: string; icon: typeof Folder }> = [
  { id: "project", label: "项目", icon: Folder }, { id: "parallel", label: "平行视图", icon: PanelLeft },
  { id: "search", label: "搜索", icon: Search }, { id: "bookmarks", label: "书签", icon: Star },
  { id: "history", label: "历史", icon: History },
  { id: "pipeline", label: "Pipeline", hint: "处理流程", icon: Workflow },
  { id: "settings", label: "设置", icon: Settings2 },
];
const currentMode = computed(() => {
  if (activeNav.value === "search") return { id: activeMode.value, label: "搜索", hint: "Search", icon: Search };
  return modeItems.find((item) => item.id === activeMode.value) ?? modeItems[0];
});
const segmentLabels = computed(() => new Map([...sourceRows.value, ...targetRows.value].map((segment) => [segment.id, String(segment.order + 1).padStart(6, "0")])));
const alignmentLabels = computed(() => new Map(alignmentRows.value.map((alignment) => [alignment.id, segmentLabels.value.get(alignment.sourceIds[0] ?? alignment.targetIds[0] ?? "") ?? alignment.id.slice(0, 8)])));
const segmentLabel = (segmentId: string) => segmentLabels.value.get(segmentId) ?? segmentId.slice(0, 8);
const alignmentLabel = (alignmentId: string | null | undefined) => {
  const alignment = alignmentRows.value.find((candidate) => candidate.id === alignmentId);
  return alignment ? segmentLabel(alignment.sourceIds[0] ?? alignment.targetIds[0] ?? alignment.id) : "—";
};
const humanizeSummary = (summary: string) => summary.replace(/[0-9a-f]{8}-[0-9a-f-]{27,}/gi, (id) => segmentLabels.value.get(id) ?? alignmentLabels.value.get(id) ?? id.slice(0, 8));
const revisionItems = computed<RevisionItem[]>(() => [...(projectSnapshot.value?.revisions ?? [])].reverse().map((revision) => ({ id: revision.revision_id, label: `R${revision.revision_id}`, timestamp: new Date(revision.created_at).toLocaleString("zh-CN", { hour12: false }), action: revision.change_set.operation.replace(/_/g, " "), summary: humanizeSummary(revision.summary), current: revision.revision_id === projectSummary.value.revision_id })));
const canUndo = computed(() => (projectSnapshot.value?.revisions.length ?? 0) > 1);
const canRedo = computed(() => { const revisions = projectSnapshot.value?.revisions ?? []; return revisions[revisions.length - 1]?.change_set.operation.startsWith("undo:") ?? false; });
const annotations = computed<AnnotationItem[]>(() => (projectSnapshot.value?.annotations ?? []).map((annotation, index) => ({ id: annotation.annotation_id, number: index + 1, status: annotation.status, title: annotation.title, body: annotation.body, createdAt: new Date(annotation.updated_at).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }), links: annotation.linked_segment_ids.map((segmentId) => { const segment = [...sourceRows.value, ...targetRows.value].find((candidate) => candidate.id === segmentId); return { side: segment?.side ?? "source", segmentId, label: segmentLabel(segmentId), text: segment?.text }; }) })));
const bookmarkedSegmentIds = computed(() => (projectSnapshot.value?.bookmarks ?? []).map((bookmark) => bookmark.segment_id));
const annotatedSegmentIds = computed(() => [...new Set((projectSnapshot.value?.annotations ?? []).flatMap((annotation) => annotation.linked_segment_ids))]);
const notify = (message: string) => { statusMessage.value = message; };
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
  if (projectSnapshot.value) return true;
  notify("当前是演示预览，请先新建或打开本地 .jm 工程");
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
const processedTotal = computed(() => Math.max(projectSummary.value.source_count, projectSummary.value.target_count));
const progressPercentage = computed(() => Math.round(100 * projectSummary.value.alignment_count / Math.max(1, processedTotal.value)));
const alignmentStatus = computed(() => projectSummary.value.source_unlinked_count + projectSummary.value.target_unlinked_count === 0 ? "1:1" : "待校对");
const footerStatusMessage = computed(() => busy.value
  ? "处理中…"
  : isSavingDraft.value
    ? "自动保存中…"
    : dirty.value
      ? `待自动保存（${autoSaveDelayMs.value / 1000}s）`
      : statusMessage.value);
let unlistenClose: (() => void) | null = null;

const closeWindowSafely = async () => {
  const saved = await persistDraft(true, "close");
  if (!saved) return;
  await flushSettings();
  if (projectSnapshot.value) {
    try { await kernelClient.flushProject(); } catch (error) { notify(`关闭前保存失败：${errorMessage(error)}`); return; }
  }
  const currentWindow = getCurrentWindow();
  unlistenClose?.();
  unlistenClose = null;
  await currentWindow.close();
};

const errorMessage = (error: unknown) => error instanceof Error ? error.message : String(error);
const closeAnnotationPanel = () => {
  annotationOpen.value = false;
  assistantOpen.value = false;
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
  agentWorkspace.setProjectSnapshot(snapshot);
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
    forceMode(general.value.defaultWorkspace);
    annotationContextSegmentId.value = null;
    selectedAnnotationId.value = null;
    resetSearch();
  }
};
const acceptAgentSearch = ({ spec, results }: { spec: SearchSpec | null; results: unknown }) => {
  if (spec) {
    searchQuery.value = spec.query;
    searchRegex.value = spec.regex;
    searchCaseSensitive.value = spec.case_sensitive;
    searchSide.value = spec.language_id === projectSnapshot.value?.project.source_language ? "source" : spec.language_id === projectSnapshot.value?.project.target_language ? "target" : "both";
  }
  const response = results as SearchSegmentsResponse | null;
  if (!response || !Array.isArray(response.hits)) { searchResults.value = []; return; }
  if (response.revision_id !== projectSnapshot.value?.project.current_revision_id) return;
  const byId = new Map([...sourceRows.value, ...targetRows.value].map(row => [row.id, row]));
  searchResults.value = response.hits.map(hit => {
    const alignment = alignmentRows.value.find(item => item.id === hit.alignment_id);
    const sourceId = hit.language_id === projectSnapshot.value?.project.source_language ? hit.segment_id : alignment?.sourceIds[0] ?? null;
    const targetId = hit.language_id === projectSnapshot.value?.project.target_language ? hit.segment_id : alignment?.targetIds[0] ?? null;
    return { id: hit.segment_id, label: segmentLabel(hit.segment_id), sourceId, targetId, sourceText: sourceId ? byId.get(sourceId)?.text ?? "" : "", targetText: targetId ? byId.get(targetId)?.text ?? "" : "", alignmentId: hit.alignment_id, alignmentLabel: alignmentLabel(hit.alignment_id) };
  });
};
const agentWorkspace = useAgentWorkspace({
  tab: () => activeNav.value,
  mode: () => activeMode.value,
  projectSnapshot: () => projectSnapshot.value,
  selection: () => activeNav.value === "parallel" && projectSnapshot.value ? { segmentIds: [...new Set([...workspaceOperationSelection.value.segmentIds, ...(editSession.value ? [editSession.value.segmentId] : [])])], alignmentIds: workspaceOperationSelection.value.alignmentIds } : { segmentIds: [], alignmentIds: [] },
  searchState: () => ({ query: searchQuery.value, regex: searchRegex.value, caseSensitive: searchCaseSensitive.value, languageId: searchSide.value === "source" ? projectSnapshot.value?.project.source_language ?? null : searchSide.value === "target" ? projectSnapshot.value?.project.target_language ?? null : null }),
  selectionSharingEnabled: () => settings.value.device.agent.shareSelection,
  canLeaveDraft: () => {
    if (!dirty.value) return true;
    notify("请先保存或放弃当前编辑，再让助手跳转");
    return false;
  },
  navigate: async (tab) => {
    if (!navItems.some(item => item.id === tab)) throw new Error("助手请求了不可用的页面");
    const scope = projectSnapshot.value?.project;
    if (tab === "pipeline") await loadPipelineWorkspace();
    if (scope?.project_id !== projectSnapshot.value?.project.project_id || scope?.current_revision_id !== projectSnapshot.value?.project.current_revision_id) throw new Error("工程状态已变化，请重新请求跳转");
    setNav(tab as NavId);
    await nextTick();
    if (activeNav.value !== tab) throw new Error("当前编辑尚未允许切换页面");
    guidance.value = { selector: `[data-nav-id="${CSS.escape(tab)}"]`, label: `已到达${navItems.find(item => item.id === tab)?.label ?? tab}`, key: Date.now() };
  },
  reveal: async ({ segment_id, alignment_id, node_id }) => {
    const scope = projectSnapshot.value?.project;
    if (node_id) {
      await loadPipelineWorkspace();
      if (scope?.project_id !== projectSnapshot.value?.project.project_id || scope?.current_revision_id !== projectSnapshot.value?.project.current_revision_id) throw new Error("工程状态已变化，请重新请求定位");
      setNav("pipeline"); await nextTick();
      if (!await pipelineWorkspaceRef.value?.revealNode(node_id)) throw new Error("未找到对应的 Pipeline 节点");
      guidance.value = { selector: `[data-pipeline-node-id="${CSS.escape(node_id)}"]`, label: "已定位到这个节点", key: Date.now() };
      return;
    }
    const alignment = alignmentRows.value.find(item => item.id === alignment_id);
    const segmentId = segment_id ?? alignment?.sourceIds[0] ?? alignment?.targetIds[0];
    if (!segmentId || ![...sourceRows.value, ...targetRows.value].some(row => row.id === segmentId)) throw new Error("当前工程中未找到该句段");
    setNav("parallel");
    await nextTick();
    if (!await parallelWorkspaceRef.value?.focusSegment(segmentId)) throw new Error("句段尚未完成定位");
    guidance.value = { selector: `[data-segment-id="${CSS.escape(segmentId)}"]`, label: "已定位到这个句段", key: Date.now() };
  },
  onProjectSnapshot: async snapshot => { await applySnapshot(snapshot); },
  onSearchState: acceptAgentSearch,
  onAppEvent: async event => {
    if (event.kind === "pipeline_changed") { pipelineRefreshKey.value++; await reloadPipelineProposals(); }
  },
});
const agentProjectKey = computed(() => projectSnapshot.value?.project.project_id ?? "application");
const agentDraft = computed({
  get: () => agentWorkspace.store.ensureProjectScope(agentProjectKey.value).drafts.composer ?? "",
  set: value => agentWorkspace.store.setDraft(agentProjectKey.value, "composer", value),
});
const agentActivityLabels: Record<string, string> = { navigation_requested: "跳转页面", reveal_requested: "定位内容", search_changed: "更新搜索", proposal_changed: "更新修改提案", operation_changed: "操作状态", revision_advanced: "工程已保存", project_changed: "工程已切换" };
const agentActivity = computed(() => agentWorkspace.store.activity.filter(event => event.kind !== "context_changed").map(event => ({ id: event.sequence, title: agentActivityLabels[event.kind] ?? event.kind, status: event.origin === "native" ? "应用内" : "外部助手" })));
const textProposals = computed<ReviewableProposal[]>(() => agentWorkspace.store.proposals.filter(proposal => proposal.status === "pending").flatMap(proposal => {
  const preview = proposal.preview as { base_revision_id?: string; items?: Array<{ segment_id: string; before: string; after: string }> } | undefined;
  if (!proposal.proposal_id || !preview?.items) return [];
  const selectedIds = (proposal.request as { selected_segment_ids?: string[] } | undefined)?.selected_segment_ids ?? [];
  const changes = preview.items.filter(item => !selectedIds.length || selectedIds.includes(item.segment_id));
  return [{ id: proposal.proposal_id, title: "审核文本替换", status: "pending", revision: preview.base_revision_id ?? proposal.base_revision_id ?? "—", changes: changes.map(item => ({ id: item.segment_id, before: item.before, after: item.after })) }];
}));
const agentProposals = computed<ReviewableProposal[]>(() => [...textProposals.value, ...pipelineProposals.value.filter(proposal => proposal.status === "pending").map(proposal => ({ id: proposal.proposal_id, title: `审核方法 · ${proposal.before.name}`, status: proposal.status, revision: proposal.base_canonical_revision_id, changes: [{ id: proposal.method_id, before: JSON.stringify(proposal.before.current.plan.nodes.map(node => ({ operator: node.operator, config: node.config })), null, 2), after: JSON.stringify(proposal.request.plan.nodes.map(node => ({ operator: node.operator, config: node.config })), null, 2) }] }))]);
async function reloadPipelineProposals() {
  const binding = agentWorkspace.binding.value?.binding_id;
  if (!binding) { pipelineProposals.value = []; return; }
  try { const proposals = await pipelineClient.listProposals(binding); if (binding === agentWorkspace.binding.value?.binding_id) pipelineProposals.value = proposals; }
  catch { /* Initial binding can be replaced while opening a project. */ }
}
watch(() => agentWorkspace.binding.value?.binding_id, () => { pipelineProposals.value = []; void reloadPipelineProposals(); });
const reviewAgentProposal = async (id: string, approve: boolean) => {
  if (approve && dirty.value) { notify("请先保存当前编辑，再审核助手修改"); return; }
  const bindingId = agentWorkspace.binding.value?.binding_id;
  if (!bindingId) return;
  agentBusy.value = true;
  agentActionError.value = null;
  try {
    if (pipelineProposals.value.some(proposal => proposal.proposal_id === id)) {
      await (approve ? pipelineClient.approveUpdate(id, bindingId) : pipelineClient.rejectUpdate(id, bindingId));
      await reloadPipelineProposals();
    } else await agentClient.call(approve ? "proposal.approve" : "proposal.reject", { proposal_id: id }, bindingId);
    notify(approve ? "助手修改已保存为新版本" : "已拒绝助手修改");
  } catch (error) { agentActionError.value = errorMessage(error); }
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
  } catch (error) { if (generation === runtimeHistoryGeneration && projectId === projectSnapshot.value?.project.project_id) { runtimeMessages.value = []; agentActionError.value = errorMessage(error); } }
}
async function handleRuntimeEvent(event: AgentRuntimeEvent) {
  runtimeSessions.set(event.project_id, event.session_id);
  runtimeRuns.value = { ...runtimeRuns.value, [event.project_id]: { runId: event.run_id, state: event.state } };
  if (event.project_id !== projectSnapshot.value?.project.project_id) return;
  if (event.kind === "run_failed") {
    const payload = event.payload as { error?: string };
    agentActionError.value = payload.error ?? "助手执行失败，请检查模型配置";
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
  catch (error) { agentActionError.value = errorMessage(error); }
}
async function refreshRuntimeState() {
  try {
    const status = await agentRuntimeClient.status();
    runtimeStatus.value = status;
    runtimeRuns.value = Object.fromEntries(Object.entries(runtimeRuns.value).map(([id, run]) => [id, status.active_run_ids.includes(run.runId) ? run : { ...run, state: "interrupted" }]));
    await loadRuntimeHistory();
  } catch (error) { agentActionError.value = errorMessage(error); }
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
    if (projectId !== projectSnapshot.value?.project.project_id) throw new Error("工程已切换，请在当前工程重新发送");
    const context = agentWorkspace.recordOutboundMessage(projectId, messageId, crypto.randomUUID(), text);
    const result = await agentRuntimeClient.start({ project_id: projectId, prompt: text, context, session_id: runtimeSessions.get(projectId) });
    runtimeSessions.set(projectId, result.session_id); runtimeRuns.value = { ...runtimeRuns.value, [projectId]: { runId: result.run_id, state: result.state } };
    agentWorkspace.store.markMessageSent(projectId, messageId);
    if (agentWorkspace.store.ensureProjectScope(projectId).drafts.composer === sentDraft) agentWorkspace.store.setDraft(projectId, "composer", "");
    await loadRuntimeHistory();
  } catch (error) { if (projectId === projectSnapshot.value?.project.project_id) agentActionError.value = errorMessage(error); agentWorkspace.store.markMessageFailed(projectId, messageId, errorMessage(error)); }
  finally { if (runtimeStarting.value === projectId) runtimeStarting.value = null; }
}
async function cancelAgentRun() {
  const run = runtimeRuns.value[agentProjectKey.value];
  if (!run) return;
  try { await agentRuntimeClient.cancel(run.runId); } catch (error) { agentActionError.value = errorMessage(error); }
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
          notify(`已重新打开 ${projectSummary.value.name}`);
        } catch {
          localStorage.removeItem("jueming-last-project-path");
        }
      }
    } else if (general.value.startupDestination === "project-picker" && isTauriRuntime) {
      await openProject();
    }
  }
  if (projectSnapshot.value) await maybeCleanupCache();
  else if (isTauriRuntime) {
    activeNav.value = "project";
    selectedAlignmentId.value = "";
    notify("尚未打开工程，请新建或打开本地 .jm 工程");
  }
  if (isTauriRuntime) {
    unlistenClose = await getCurrentWindow().onCloseRequested((event) => { event.preventDefault(); void closeWindowSafely(); });
  }
});
const previousRevisionId = () => { const revisions = projectSnapshot.value?.revisions ?? []; return revisions[revisions.length - 2]?.revision_id ?? projectSummary.value.revision_id; };
const applyModeContext = (mode: WorkspaceMode) => {
  activeNav.value = mode === "history" ? "history" : "parallel";
  exportOpen.value = false;
  if (mode === "history") void selectHistoryRevision(previousRevisionId());
};
const setMode = (mode: WorkspaceMode) => {
  pendingNav.value = null;
  if (requestMode(mode) === "applied") applyModeContext(mode);
};
const applyNavContext = (nav: NavId) => {
  activeNav.value = nav;
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
  if (alignmentId !== undefined) selectedAlignmentId.value = alignmentId ?? "";
  annotationContextSegmentId.value = segmentId ?? null;
  if (segmentId) {
    selectedAnnotationId.value = annotations.value.find((annotation) => annotation.links.some((link) => link.segmentId === segmentId))?.id ?? null;
  }
  annotationOpen.value = true;
  rightPanelTab.value = "annotations";
  exportOpen.value = false;
};
const openNewProject = () => { void newProjectDialogRef.value?.open(); };
const openProject = async () => {
  const selected = await open({ directory: true, multiple: false, title: "打开决明工程（.jm 文件夹）" });
  if (!selected) return;
  busy.value = true;
  try { await applySnapshot(await kernelClient.openProject(selected), true); localStorage.setItem("jueming-last-project-path", selected); notify(`已打开 ${projectSummary.value.name}`); }
  catch (error) { notify(`打开失败：${errorMessage(error)}`); }
  finally { busy.value = false; }
};
const handleProjectCreated = async (snapshot: ProjectSnapshot, createdProjectPath: string) => {
  localStorage.setItem("jueming-last-project-path", createdProjectPath);
  try {
    await applySnapshot(snapshot, true);
    notify(`已创建并保存 ${projectSummary.value.name}`);
  } catch (error) {
    notify(`工程已创建，但加载工作区失败：${errorMessage(error)}`);
  }
};
const saveProject = async () => {
  if (!(await persistDraft(false, "manual"))) return;
  try { await kernelClient.flushProject(); notify("本地存储 · 已保存"); }
  catch (error) { notify(`保存失败：${errorMessage(error)}`); }
};
async function persistSegment(id: string, text: string) {
  const segment = [...sourceRows.value, ...targetRows.value].find((item) => item.id === id);
  if (!segment || segment.text === text) return;
  await kernelClient.updateSegment(id, text);
  await applySnapshot(await kernelClient.getCurrentProject());
  notify("句段已自动保存并写入本地历史");
}
const requestSegmentEdit = (segmentId: string, alignmentId: string) => {
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
  const rows = side === "source" ? sourceRows : targetRows;
  const index = rows.value.findIndex((item) => item.id === segmentId);
  const targetIndex = direction === "up" ? index - 1 : index + 1;
  if (index < 0 || targetIndex < 0 || targetIndex >= rows.value.length) {
    notify(direction === "up" ? "已经是第一句" : "已经是最后一句");
    return;
  }
  const targetSegment = rows.value[targetIndex];
  if (!(await confirmCrossAlignmentReorder(side, {
    segmentId,
    targetId: targetSegment.id,
    edge: direction === "up" ? "before" : "after",
  }))) {
    notify("已取消跨 Alignment 排序");
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
    notify(`${side === "source" ? "中文" : "英文"} Segment 已${direction === "up" ? "上移" : "下移"}，Alignment 关系保持稳定`);
  } catch (error) {
    rows.value = previous;
    notify(`排序失败：${errorMessage(error)}`);
  }
};
const alignmentIdForSegment = (segmentId: string) => alignmentRows.value.find(
  (alignment) => [...alignment.sourceIds, ...alignment.targetIds].includes(segmentId),
)?.id ?? null;
const describeAlignmentForSegment = (segmentId: string) => {
  const alignmentId = alignmentIdForSegment(segmentId);
  return alignmentId ? `Alignment ${alignmentLabel(alignmentId)}` : "未对齐区域";
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
  const message = `把${side === "source" ? "中文" : "英文"} Segment ${segmentLabel(intent.segmentId)} 从${describeAlignmentForSegment(intent.segmentId)}拖到${describeAlignmentForSegment(intent.targetId)}附近，会打断当前对齐块的连续顺序。Alignment 关系不会删除，但阅读顺序可能交叉。是否继续？`;
  return isTauriRuntime
    ? confirm(message, { title: "即将打断对齐块", kind: "warning" })
    : window.confirm(`即将打断对齐块\n\n${message}`);
};
const reorderSegment = async (side: LanguageSide, orderedSegmentIds: string[], intent: ReorderIntent) => {
  if (!requireOpenProject()) return;
  if (!(await confirmCrossAlignmentReorder(side, intent))) {
    notify("已取消跨 Alignment 排序");
    return;
  }
  busy.value = true;
  try { await applySnapshot(await kernelClient.reorderSegments(orderedSegmentIds)); clearWorkspaceSelection(); notify(`${side === "source" ? "中文" : "英文"}列拖拽排序已自动保存，Alignment 关系保持稳定`); }
  catch (error) { notify(`拖拽排序失败：${errorMessage(error)}`); }
  finally { busy.value = false; }
};
const resetOrder = async () => {
  if (!requireOpenProject()) return;
  if (!orderBaseline.value.source.length && !orderBaseline.value.target.length) { notify("当前会话没有可恢复的排序基线"); return; }
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
    notify("已恢复进入当前审阅排序工作区时的句段顺序");
  } catch (error) {
    await applySnapshot(await kernelClient.getCurrentProject());
    notify(`恢复顺序失败：${errorMessage(error)}`);
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
      const message = errorMessage(error);
      if (!message.includes("confirm replacement before linking") && !message.includes("AlignmentSelectionConflict")) throw error;
      const approved = "__TAURI_INTERNALS__" in window
        ? await confirm("部分句段已经属于 Alignment。替换现有关系会创建新的 Revision。", { title: "替换现有 Alignment？", kind: "warning" })
        : window.confirm("部分句段已经属于 Alignment，是否替换现有关系？");
      if (!approved) return;
      snapshot = await kernelClient.linkSegments(sourceSegmentIds, targetSegmentIds, true);
    }
    await applySnapshot(snapshot);
    clearWorkspaceSelection();
    selectedAlignmentId.value = alignmentRows.value.find((alignment) => sourceSegmentIds.every((id) => alignment.sourceIds.includes(id)) && targetSegmentIds.every((id) => alignment.targetIds.includes(id)))?.id ?? selectedAlignmentId.value;
    notify(`已建立 ${sourceSegmentIds.length}:${targetSegmentIds.length} Alignment`);
  } catch (error) { notify(`Link 失败：${errorMessage(error)}`); }
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
    notify("已解除 Alignment，句段保持未对齐");
  }
  catch (error) { notify(`Unlink 失败：${errorMessage(error)}`); }
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
const performUndo = async () => { busy.value = true; try { await applySnapshot(await kernelClient.undo()); notify("已撤销并保存为新的 Revision"); } catch (error) { notify(`撤销失败：${errorMessage(error)}`); } finally { busy.value = false; } };
const performRedo = async () => { busy.value = true; try { await applySnapshot(await kernelClient.redo()); notify("已重做并保存为新的 Revision"); } catch (error) { notify(`重做失败：${errorMessage(error)}`); } finally { busy.value = false; } };
const comparisonToDiff = (comparison: RevisionComparison): HistoryDiff => {
  const sourceDocumentId = projectSnapshot.value?.documents[0]?.document_id;
  const targetDocumentId = projectSnapshot.value?.documents[1]?.document_id;
  const sourceChange = comparison.segment_changes.find((change) => (change.before ?? change.after)?.document_id === sourceDocumentId);
  const targetChange = comparison.segment_changes.find((change) => (change.before ?? change.after)?.document_id === targetDocumentId);
  return {
    segmentId: (sourceChange ?? targetChange) ? segmentLabel((sourceChange ?? targetChange)!.segment_id) : "结构变更",
    sourceOld: sourceChange?.before?.content ?? "", sourceNew: sourceChange?.after?.content ?? "",
    targetOld: targetChange?.before?.content ?? "", targetNew: targetChange?.after?.content ?? "",
    deletedLines: comparison.segment_changes.filter((change) => change.before).length,
    addedLines: comparison.segment_changes.filter((change) => change.after).length,
    summary: `文本 ${comparison.segment_changes.length} 项、顺序 ${comparison.order_changes.length} 项、Alignment ${comparison.alignment_changes.length} 项变更。`,
  };
};
const compareHistory = async (fromRevisionId: string, toRevisionId: string) => {
  if (fromRevisionId === toRevisionId) return;
  busy.value = true;
  try { baseRevisionId.value = fromRevisionId; selectedRevisionId.value = toRevisionId; historyDiff.value = comparisonToDiff(await kernelClient.compareRevision(fromRevisionId, toRevisionId)); }
  catch (error) { notify(`版本比较失败：${errorMessage(error)}`); }
  finally { busy.value = false; }
};
const selectHistoryRevision = async (revisionId: string) => {
  const current = projectSummary.value.revision_id;
  const fallback = projectSnapshot.value?.revisions.find((revision) => revision.revision_id !== revisionId)?.revision_id;
  selectedRevisionId.value = revisionId;
  await compareHistory(revisionId === current ? (fallback ?? current) : revisionId, current);
};
const restoreHistory = async (revisionId: string) => {
  const approved = "__TAURI_INTERNALS__" in window ? await confirm("恢复会追加一个新 Revision，现有历史不会删除。", { title: `恢复 R${revisionId}？`, kind: "warning" }) : window.confirm(`恢复 R${revisionId}？`);
  if (!approved) return;
  busy.value = true;
  try { await applySnapshot(await kernelClient.restoreRevision(revisionId)); notify(`已将 R${revisionId} 恢复为新版本`); await selectHistoryRevision(revisionId); }
  catch (error) { notify(`恢复失败：${errorMessage(error)}`); }
  finally { busy.value = false; }
};
const makeSearchRequest = (options: SearchQueryOptions) => {
  const snapshot = projectSnapshot.value;
  if (!snapshot) throw new Error("请先新建或打开本地工程");
  return { project_id: snapshot.project.project_id, query: options.query, regex: options.regex, case_sensitive: options.caseSensitive, language_id: options.side === "source" ? snapshot.project.source_language : options.side === "target" ? snapshot.project.target_language : null, base_revision_id: snapshot.project.current_revision_id };
};
const runSearch = async (options: SearchQueryOptions) => {
  busy.value = true;
  const searchProjectId = projectSnapshot.value?.project.project_id;
  try {
    const request = makeSearchRequest(options);
    if (isTauriRuntime) {
      const binding = agentWorkspace.binding.value?.binding_id;
      if (!binding) throw new Error("工程连接尚未就绪，请稍后重试");
      const spec: SearchSpec = { query: request.query, regex: request.regex, case_sensitive: request.case_sensitive, language_id: request.language_id };
      const result = await agentClient.call<{ total: number }>("search.execute", { spec, expected_revision_id: request.base_revision_id, page_size: 1 }, binding);
      if (searchProjectId === projectSnapshot.value?.project.project_id) notify(`找到 ${result.data.total} 条结果`);
      return;
    }
    const response = await kernelClient.searchSegments(request);
    const sourceById = new Map(sourceRows.value.map((segment) => [segment.id, segment.text]));
    const targetById = new Map(targetRows.value.map((segment) => [segment.id, segment.text]));
    searchResults.value = response.hits.map((hit) => {
      const alignment = hit.alignment_id ? alignmentRows.value.find((candidate) => candidate.id === hit.alignment_id) : undefined;
      const sourceId = hit.language_id === projectSnapshot.value?.project.source_language ? hit.segment_id : alignment?.sourceIds[0] ?? null;
      const targetId = hit.language_id === projectSnapshot.value?.project.target_language ? hit.segment_id : alignment?.targetIds[0] ?? null;
      return { id: hit.segment_id, label: segmentLabel(hit.segment_id), sourceId, targetId, sourceText: sourceId ? sourceById.get(sourceId) ?? "" : "", targetText: targetId ? targetById.get(targetId) ?? "" : "", alignmentId: hit.alignment_id, alignmentLabel: alignmentLabel(hit.alignment_id) };
    });
    notify(`找到 ${response.hits.length} 条结果`);
  } catch (error) { if (searchProjectId === projectSnapshot.value?.project.project_id) { searchResults.value = []; notify(`搜索失败：${errorMessage(error)}`); } }
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
    notify(`替换预览包含 ${response.items.length} 个句段`);
  } catch (error) { if (generation === replacePreviewGeneration) { replacePreviewError.value = errorMessage(error); notify(`替换预览失败：${errorMessage(error)}`); } }
  finally { if (generation === replacePreviewGeneration) replacePreviewLoading.value = false; }
};
watch(() => [searchQuery.value, searchSide.value, searchRegex.value, searchCaseSensitive.value, replacement.value, projectSnapshot.value?.project.project_id, projectSnapshot.value?.project.current_revision_id], () => {
  replacePreviewGeneration++; nativeReplacePreview.value = null; nativeReplaceRequest.value = null; replacePreviewLoading.value = false; replacePreviewError.value = null;
}, { flush: "sync" });
const applyReplacement = async (preview: UiReplacePreview) => {
  const request = isTauriRuntime ? nativeReplaceRequest.value : currentReplaceRequest();
  if (!request || (isTauriRuntime && preview !== nativeReplacePreview.value)) { notify("替换条件或工程版本已变化，请重新预览"); return; }
  busy.value = true;
  try { await applySnapshot(await kernelClient.applyReplace(request, preview.resultIds)); notify(`已原子替换 ${preview.resultIds.length} 个句段`); await runSearch({ query: searchQuery.value, side: searchSide.value, regex: searchRegex.value, caseSensitive: searchCaseSensitive.value }); }
  catch (error) { notify(`替换失败：${errorMessage(error)}`); }
  finally { busy.value = false; }
};
const jumpToSegment = async (segmentId: string, alignmentId: string | null = null) => {
  const resolvedAlignmentId = alignmentId ?? alignmentRows.value.find((alignment) => [...alignment.sourceIds, ...alignment.targetIds].includes(segmentId))?.id ?? "";
  selectedAlignmentId.value = resolvedAlignmentId;
  activeNav.value = "parallel";
  activeMode.value = "review";
  await nextTick();
  return parallelWorkspaceRef.value?.focusSegment(segmentId) ?? false;
};
const selectSearchResult = (result: SearchResult) => { void jumpToSegment(result.id, result.alignmentId ?? null); };
const toggleBookmark = async (segmentId: string, alignmentId: string | null) => {
  const existing = projectSnapshot.value?.bookmarks.find((bookmark) => bookmark.segment_id === segmentId);
  const segment = [...sourceRows.value, ...targetRows.value].find((candidate) => candidate.id === segmentId);
  const content = segment?.text.replace(/\s+/g, " ").trim() ?? "当前句段";
  try { await applySnapshot(existing ? await kernelClient.deleteBookmark(existing.bookmark_id) : await kernelClient.createBookmark(segmentId, alignmentId, `书签 ${segmentLabel(segmentId)} · ${content.slice(0, 48)}`)); notify(existing ? "已移除书签" : "已添加书签"); }
  catch (error) { notify(`书签操作失败：${errorMessage(error)}`); }
};
const openBookmark = (segmentId: string, alignmentId: string | null) => { void jumpToSegment(segmentId, alignmentId); };
const openAnnotationLink = (segmentId: string) => { void jumpToSegment(segmentId); };
const removeBookmark = async (bookmarkId: string) => { try { await applySnapshot(await kernelClient.deleteBookmark(bookmarkId)); notify("已移除书签"); } catch (error) { notify(`移除书签失败：${errorMessage(error)}`); } };
const resetSearch = () => { searchQuery.value = ""; replacement.value = ""; searchResults.value = []; };
const copyHistoryValue = async (value: string) => { try { await navigator.clipboard.writeText(value); notify("已复制到剪贴板"); } catch { notify("复制失败：系统剪贴板不可用"); } };
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
  try { await applySnapshot(await kernelClient.createAnnotation(annotationRequest(linked))); notify("批注已创建并自动保存"); }
  catch (error) { notify(`新建批注失败：${errorMessage(error)}`); }
};
const editAnnotation = async (annotationId: string, draft: AnnotationDraft) => { try { await applySnapshot(await kernelClient.updateAnnotation(annotationId, annotationRequest(draft))); notify("批注已更新"); } catch (error) { notify(`更新批注失败：${errorMessage(error)}`); } };
const deleteAnnotation = async (annotationId: string) => { const approved = "__TAURI_INTERNALS__" in window ? await confirm("删除批注会保留在 Revision 历史中。", { title: "删除批注？", kind: "warning" }) : window.confirm("删除批注？"); if (!approved) return; try { await applySnapshot(await kernelClient.deleteAnnotation(annotationId)); notify("批注已删除"); } catch (error) { notify(`删除批注失败：${errorMessage(error)}`); } };
const resolveAnnotation = async (annotationId: string) => { try { await applySnapshot(await kernelClient.resolveAnnotation(annotationId)); notify("批注已标记为解决"); } catch (error) { notify(`解决批注失败：${errorMessage(error)}`); } };
const exportProject = async (format: ExportFormat) => { exportOpen.value = false; const outputPath = await saveDialog({ title: `导出 ${format.toUpperCase()}`, defaultPath: `${projectSummary.value.name}.${format}`, filters: [{ name: format.toUpperCase(), extensions: [format] }] }); if (!outputPath) return; busy.value = true; try { await kernelClient.exportProject(format, outputPath); notify(`已导出 ${outputPath}`); } catch (error) { notify(`导出失败：${errorMessage(error)}`); } finally { busy.value = false; } };
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
onBeforeUnmount(() => { agentWorkspace.dispose(); unlistenRuntime?.(); unlistenRuntimeResync?.(); runtimeHistoryGeneration++; window.removeEventListener("keydown", handleShortcut); stopAnnotationResize(); unlistenClose?.(); disposeSettings(); void flushSettings(); });
</script>

<template>
  <div class="app-shell" :class="{ 'app-shell--garden': gardenVisible }">
    <header class="app-toolbar"><div class="toolbar-left"><button class="toolbar-button" type="button" :title="`新建工程（${shortcutLabels.newProject}）`" @click="openNewProject"><FilePlus2 :size="19" />新建</button><button class="toolbar-button" type="button" :title="`打开本地工程（${shortcutLabels.openProject}）`" @click="openProject"><FolderOpen :size="19" />打开</button><button class="toolbar-button" type="button" :title="`保存工程（${shortcutLabels.save}）`" @click="saveProject"><Save :size="19" />保存</button><div class="toolbar-divider"></div><div class="toolbar-actions"><div class="toolbar-button--export-wrap"><button class="toolbar-button toolbar-button--export" type="button" title="导出工程" @click="exportOpen = !exportOpen"><Download :size="19" />导出<ChevronDown :size="15" /></button><div v-if="exportOpen" class="export-menu"><button type="button" @click="exportProject('txt')">TXT 文本</button><button type="button" @click="exportProject('json')">JSON 工程</button><button type="button" @click="exportProject('xml')">XML 对齐</button></div></div><div class="toolbar-divider"></div><button class="toolbar-button" type="button" :disabled="!canUndo || busy" :title="`撤销（${shortcutLabels.undo}）`" @click="performUndo"><Undo2 :size="19" />撤销</button><button class="toolbar-button" type="button" :disabled="!canRedo || busy" :title="`重做（${shortcutLabels.redo}）`" @click="performRedo"><Redo2 :size="19" />重做</button><div class="toolbar-divider"></div><button class="toolbar-button" type="button" @click="setNav('settings')"><Settings2 :size="19" />设置</button></div></div><div class="global-panel-buttons"><button class="toolbar-button" type="button" :aria-pressed="rightPanelOpen && rightPanelTab === 'agent'" @click="assistantOpen = true; rightPanelTab = 'agent'"><Sparkles :size="18" />助手<span v-if="agentProposals.length" class="pending-count">{{ agentProposals.length }}</span></button><button class="toolbar-button" type="button" :aria-pressed="rightPanelOpen && rightPanelTab === 'annotations'" @click="openAnnotationPanel()"><MessageSquareText :size="18" />批注</button></div><div class="mode-control" :class="`mode-control--${activeMode}`"><component :is="currentMode.icon" :size="17" /><span>{{ currentMode.label }} <small>{{ currentMode.hint }}</small></span><ChevronDown :size="15" /><select :value="activeMode" aria-label="切换工作模式" @change="modeSelect"><option v-for="item in modeMenuItems" :key="item.id" :value="item.id">{{ item.label }} {{ item.hint }}</option></select></div></header>
    <div class="content-grid" :class="{ 'content-grid--collapsed': sideNavCollapsed, 'content-grid--annotation': rightPanelOpen, 'content-grid--resizing': annotationResizing }" :style="{ '--annotation-width': `${annotationWidth}px` }">
      <nav class="side-nav" :class="{ 'side-nav--collapsed': sideNavCollapsed }"><button v-for="item in navItems" :key="item.id" :data-nav-id="item.id" class="nav-item" :class="{ 'nav-item--active': activeNav === item.id }" type="button" :title="item.label" @click="setNav(item.id)"><component :is="item.icon" :size="22" :stroke-width="activeNav === item.id ? 2.2 : 1.8" /><span class="nav-label">{{ item.label }}<small v-if="item.hint">{{ item.hint }}</small></span></button><CompanionHabitat :in-garden="gardenStrolling" v-if="gardenVisible && !sideNavCollapsed" :settings="settings.device.pet" :activity="companionActivity" :dirty-editor="dirty" :motion="effectiveMotionMode" /><div class="nav-collapse"><button class="nav-item" type="button" :title="sideNavCollapsed ? '展开侧栏' : '收起侧栏'" @click="toggleSideNav"><ChevronDown :size="21" :style="{ transform: sideNavCollapsed ? 'rotate(-90deg)' : 'rotate(90deg)' }" /><span class="nav-label">{{ sideNavCollapsed ? '展开' : '收起' }}</span></button></div></nav>
      <main class="main-stage">
        <PipelineWorkspace v-if="pipelineVisited" ref="pipelineWorkspaceRef" v-show="activeNav === 'pipeline'" :binding-id="agentWorkspace.binding.value?.binding_id ?? null" :project-id="projectSnapshot?.project.project_id ?? null" :revision-id="projectSummary.revision_id" :source-segments="sourceRows" :refresh-key="pipelineRefreshKey" :available="isTauriRuntime" @status="notify" />
        <ParallelWorkspace @selection-context="workspaceOperationSelection = $event" ref="parallelWorkspaceRef" v-if="activeNav === 'parallel'" :mode="activeMode" :source-segments="sourceRows" :target-segments="targetRows" :alignments="alignmentRows" :selected-alignment-id="selectedAlignmentId" :bookmarked-segment-ids="bookmarkedSegmentIds" :annotated-segment-ids="annotatedSegmentIds" :edit-session="editSession" :trackpad-optimized="trackpadOptimized" :smooth-navigation="smoothNavigationEnabled" :writable="workspaceWritable" @select="selectedAlignmentId = $event" @request-edit="requestSegmentEdit" @edit-draft="updateDraft" @commit-edit="commitSegmentEdit" @cancel-edit="cancelSegmentEdit" @escape-edit="escapeSegmentEdit" @move="moveSegment" @reorder="reorderSegment" @insert-gap="insertAlignmentGap" @reset-order="resetOrder" @link="linkSegments" @unlink="unlinkAlignment" @merge-segments="mergeSegmentContent" @split-segment="splitSegmentContent" @group="groupAlignments" @ungroup="ungroupAlignment" @bookmark="toggleBookmark" @annotation="openAnnotationPanel" @status="notify" />
        <SearchReplaceWorkspace :authoritative-results="isTauriRuntime" :native-preview="nativeReplacePreview" :preview-loading="replacePreviewLoading" :preview-error="replacePreviewError" v-else-if="activeNav === 'search'" v-model:query="searchQuery" v-model:side="searchSide" v-model:regex="searchRegex" v-model:case-sensitive="searchCaseSensitive" v-model:replacement="replacement" :results="searchResults" :project-label="projectSummary.name" :loading="busy" @search="runSearch" @select-result="selectSearchResult" @replace-preview="previewReplacement" @apply-replace="applyReplacement" @reset="resetSearch" />
        <HistoryWorkspace v-else-if="activeNav === 'history'" :revisions="revisionItems" :diff="historyDiff" :base-revision-id="baseRevisionId" :selected-revision-id="selectedRevisionId" :current-revision-id="projectSummary.revision_id" :loading="busy" @select-revision="selectHistoryRevision" @compare="compareHistory" @restore="restoreHistory" @copy-value="copyHistoryValue" />
        <SettingsWorkspace
          v-else-if="activeNav === 'settings'"
          v-model:settings="settings"
          :capabilities="capabilities"
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
        <section v-else-if="activeNav === 'project'" class="aux-view project-view"><Folder :size="28" /><h2>项目</h2><p>{{ projectSummary.name }}</p><div class="project-summary-card"><span>{{ projectSummary.source_count }}</span><small>中文句段</small><span>{{ projectSummary.target_count }}</span><small>English segments</small><span>{{ projectSummary.alignment_count }}</span><small>Alignment</small></div><div class="project-actions"><button type="button" class="primary-button" @click="openProject">打开工程</button><button type="button" class="secondary-button" @click="openNewProject">新建工程</button></div></section>
      </main>
      <aside class="annotation-drawer" :class="{ 'annotation-drawer--open': rightPanelOpen }" :aria-hidden="!rightPanelOpen" :inert="!rightPanelOpen">
        <div class="annotation-resize-handle" role="separator" tabindex="0" aria-label="调整全局侧栏宽度" aria-orientation="vertical" :aria-valuemin="annotationMinimumWidth" :aria-valuemax="annotationMaximumWidth()" :aria-valuenow="Math.round(annotationWidth)" title="拖动调整宽度，双击恢复默认" @pointerdown="startAnnotationResize" @dblclick="resetAnnotationWidth" @keydown="resizeAnnotationWithKeyboard"></div>
        <GlobalSidePanel v-model:active-tab="rightPanelTab" :project-label="projectSnapshot ? projectSummary.name : '尚未打开工程'" :annotation-count="annotations.length" :pending-count="agentProposals.length" @close="closeAnnotationPanel">
          <template #agent><AgentPanel v-model:draft="agentDraft" :tab-label="navItems.find(item => item.id === activeNav)?.label ?? activeNav" :selected-text="agentWorkspace.context.value?.selected_text ?? ''" :connected="Boolean(agentWorkspace.binding.value)" :runtime-available="Boolean(runtimeStatus?.configured && projectSnapshot)" :running="agentBusy || runtimeRunning" :reviewing="agentBusy" :show-context="settings.device.agent.showContext" :messages="runtimeMessages.filter(message => message.role !== 'tool').map(message => ({ id: message.message_id, role: message.role, content: message.content }))" :activity="agentActivity" :proposals="agentProposals" :error="agentActionError ?? agentWorkspace.error.value" @settings="openAgentSettings" @send="sendAgentMessage" @cancel="cancelAgentRun" @clear-selection="agentWorkspace.clearSelectionContext()" @approve="reviewAgentProposal($event, true)" @reject="reviewAgentProposal($event, false)" /></template>
          <template #annotations><AnnotationPanel v-model:active-filter="annotationFilter" :scope-key="agentProjectKey" :annotations="annotations" :selected-id="selectedAnnotationId" :readonly="!projectSnapshot || activeNav === 'history'" @select="selectedAnnotationId = $event" @open-link="openAnnotationLink" @create="createAnnotation" @edit="editAnnotation" @delete="deleteAnnotation" @resolve="resolveAnnotation" @close="closeAnnotationPanel" /></template>
        </GlobalSidePanel>
      </aside>
    </div>
    <CompanionGarden v-model:strolling="gardenStrolling" v-if="gardenVisible" :settings="settings.device.pet" :activity="companionActivity" :dirty-editor="dirty" :motion="effectiveMotionMode" :proposal="agentProposals[0] ?? null" :busy="agentBusy" :guidance="guidance?.label ?? ''" @approve="reviewAgentProposal($event, true)" @reject="reviewAgentProposal($event, false)" @open-agent="assistantOpen = true; rightPanelTab = 'agent'" />
    <GuidanceOverlay :target="guidance" :animated="effectiveMotionMode === 'standard' && settings.device.pet.presentation === 'animated'" :visible="settings.device.pet.butterflyMotion && settings.device.pet.presentation !== 'hidden'" />
    <footer class="bottom-status">
      <div class="footer-context">
        <div class="footer-context-item footer-context-item--project">
          <strong class="footer-context-label">项目</strong>
          <span class="footer-ellipsis" :title="projectSummary.name">{{ projectSummary.name }}</span>
        </div>
        <div class="footer-context-separator" aria-hidden="true"></div>
        <div class="footer-context-item footer-context-item--files">
          <strong class="footer-context-label">文件</strong>
          <span class="footer-file-name footer-file-name--source" :title="projectSummary.source_label">{{ projectSummary.source_label }}</span>
          <Link2 :size="13" aria-hidden="true" />
          <span class="footer-file-name footer-file-name--target" :title="projectSummary.target_label">{{ projectSummary.target_label }}</span>
        </div>
        <div class="footer-context-separator" aria-hidden="true"></div>
        <div class="footer-context-item footer-context-item--alignment">
          <span class="footer-context-label">对齐</span>
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
        <span class="footer-processed"><span class="footer-processed-label">已处理</span><strong>{{ projectSummary.alignment_count }} / {{ processedTotal }}</strong></span>
        <span class="footer-progress-label">进度</span>
        <div class="progress-track" role="progressbar" aria-label="工程处理进度" aria-valuemin="0" aria-valuemax="100" :aria-valuenow="progressPercentage"><span :style="{ width: `${progressPercentage}%` }"></span></div>
        <strong class="footer-progress-percent">{{ progressPercentage }}%</strong>
      </div>
    </footer>
    <div v-if="pendingTransition" class="modal-backdrop mode-guard-backdrop"><section class="mode-guard" role="dialog" aria-modal="true" aria-labelledby="mode-guard-title"><span class="eyebrow">UNSAVED EDIT</span><h2 id="mode-guard-title">当前句段还有未保存编辑</h2><p>保存会创建一个完整 Revision；放弃只撤销最近一次自动保存之后的草稿。</p><div><button class="secondary-button" type="button" @click="stayInEdit">继续编辑</button><button class="secondary-button danger-button" type="button" @click="discardPendingTransition">放弃草稿</button><button class="primary-button" type="button" @click="savePendingTransition"><Check :size="15" />保存并切换</button></div></section></div>
    <NewProjectDialog ref="newProjectDialogRef" v-model:busy="busy" :kernel-client="kernelClient" @created="handleProjectCreated" @status="notify" />
  </div>
</template>

<style scoped>
.toolbar-button--export-wrap { position: relative; }.export-menu { position: absolute; z-index: 8; top: 41px; left: 0; width: 142px; padding: 6px; border: 1px solid var(--line); border-radius: 7px; background: #fff; box-shadow: 0 10px 25px rgb(35 55 38 / 14%); }.export-menu button { display: block; width: 100%; padding: 8px 10px; border: 0; border-radius: 4px; background: transparent; text-align: left; cursor: pointer; }.export-menu button:hover { color: var(--green-900); background: var(--green-050); }.mode-control small { margin-left: 2px; color: var(--green-700); font-size: var(--jm-font-size-callout); font-weight: var(--jm-font-weight-regular); line-height: var(--jm-line-height-callout); }.mode-control--edit { border-color: #e6d19d; color: #916714; background: #fffaf0; }.mode-control--edit small { color: #af8321; }.mode-control--history { border-color: #c7d4df; color: #536b7a; background: #f8fbfd; }
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
