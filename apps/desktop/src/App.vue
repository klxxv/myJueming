<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { confirm, open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Check, ChevronDown, Download, Eye, FilePlus2, Folder, FolderOpen, History, Link2, Maximize2, MessageSquareText, Minimize2, Monitor, PanelLeft, Pencil, Redo2, Save, Search, Settings2, Star, Undo2, X } from "@lucide/vue";
import ParallelWorkspace from "./components/ParallelWorkspace.vue";
import AnnotationPanel, { type AnnotationDraft, type AnnotationFilter, type AnnotationItem } from "./components/AnnotationPanel.vue";
import HistoryWorkspace, { type HistoryDiff, type RevisionItem } from "./components/HistoryWorkspace.vue";
import SearchReplaceWorkspace, { type ReplacePreview as UiReplacePreview, type SearchQueryOptions, type SearchResult } from "./components/SearchReplaceWorkspace.vue";
import { alignments as fixtureAlignments, sourceSegments, targetSegments } from "./data/governmentFixture";
import { createKernelClient, makeImportProfile, snapshotToWorkspace, type AlignmentDto, type Encoding, type ExportFormat, type ImportPreviewResponse, type ProjectSnapshot, type ProjectSummaryDto, type ReplacePreviewRequest, type RevisionComparison, type SegmentationMode, type SegmentDto, type WorkspaceMode } from "./domain/kernel-client";
import brandIcon from "../../../assets/brand/jueming-aligner-icon-source.png";
import "./styles.css";

type NavId = "project" | "parallel" | "search" | "bookmarks" | "annotations" | "history" | "settings";
const kernelClient = createKernelClient();
const activeMode = ref<WorkspaceMode>("review");
const activeNav = ref<NavId>("parallel");
const selectedAlignmentId = ref("alignment-000004");
const dirty = ref(false);
const busy = ref(false);
const statusMessage = ref("本地存储 · 已保存");
const importOpen = ref(false);
const exportOpen = ref(false);
const annotationOpen = ref(false);
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
const historyDiff = ref<HistoryDiff | null>(null);
const theme = ref<"light" | "eye">("light");
const fontScale = ref(100);
const orderBaseline = ref<string[]>([]);
const previewTab = ref<"source" | "target">("source");
const projectName = ref("阿古顿巴_中英对齐");
const projectDirectory = ref("");
const sourcePath = ref("");
const targetPath = ref("");
const sourceEncoding = ref<Encoding>("utf8");
const targetEncoding = ref<Encoding>("utf8");
const sourceSegmentation = ref<SegmentationMode>("non_empty_line");
const targetSegmentation = ref<SegmentationMode>("non_empty_line");
const sourcePreview = ref<ImportPreviewResponse | null>(null);
const targetPreview = ref<ImportPreviewResponse | null>(null);
const sourceRows = ref<SegmentDto[]>(sourceSegments.map((segment) => ({ ...segment })));
const targetRows = ref<SegmentDto[]>(targetSegments.map((segment) => ({ ...segment })));
const alignmentRows = ref<AlignmentDto[]>(fixtureAlignments.map((alignment) => ({ ...alignment })));
const projectSnapshot = ref<ProjectSnapshot | null>(null);
const projectSummary = ref<ProjectSummaryDto>({ project_id: "fixture-project", name: "2024政府工作报告_中英对齐", source_label: "report_zh.txt", target_label: "report_en.txt", source_count: 8, target_count: 8, alignment_count: 8, source_unlinked_count: 0, target_unlinked_count: 0, revision_id: "0" });

const modeItems: Array<{ id: WorkspaceMode; label: string; hint: string; icon: typeof Eye }> = [
  { id: "review", label: "审阅模式", hint: "Review", icon: Eye }, { id: "edit", label: "编辑模式", hint: "Edit", icon: Pencil },
  { id: "order", label: "顺序模式", hint: "Order", icon: PanelLeft }, { id: "history", label: "历史模式", hint: "History", icon: History },
];
const navItems: Array<{ id: NavId; label: string; hint?: string; icon: typeof Folder }> = [
  { id: "project", label: "项目", icon: Folder }, { id: "parallel", label: "平行视图", icon: PanelLeft },
  { id: "search", label: "搜索", icon: Search }, { id: "bookmarks", label: "书签", icon: Star },
  { id: "annotations", label: "批注", hint: "Annotation", icon: MessageSquareText }, { id: "history", label: "历史", icon: History }, { id: "settings", label: "设置", icon: Settings2 },
];
const currentMode = computed(() => {
  if (activeNav.value === "search") return { id: activeMode.value, label: "搜索", hint: "Search", icon: Search };
  if (activeNav.value === "annotations") return { id: activeMode.value, label: "批注模式", hint: "Annotation", icon: MessageSquareText };
  return modeItems.find((item) => item.id === activeMode.value) ?? modeItems[0];
});
const activePreview = computed(() => previewTab.value === "source" ? sourcePreview.value : targetPreview.value);
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
const bookmarkedAlignmentIds = computed(() => (projectSnapshot.value?.bookmarks ?? []).flatMap((bookmark) => bookmark.alignment_id ? [bookmark.alignment_id] : []));

const notify = (message: string) => { statusMessage.value = message; };
const windowAction = async (action: "minimize" | "maximize" | "close") => {
  if (!("__TAURI_INTERNALS__" in window)) return;
  const currentWindow = getCurrentWindow();
  if (action === "minimize") await currentWindow.minimize();
  else if (action === "maximize") await currentWindow.toggleMaximize();
  else await currentWindow.close();
};
const errorMessage = (error: unknown) => error instanceof Error ? error.message : String(error);
const applySnapshot = async (snapshot: ProjectSnapshot, resetView = false) => {
  const workspace = snapshotToWorkspace(snapshot);
  projectSnapshot.value = snapshot;
  sourceRows.value = workspace.sourceSegments;
  targetRows.value = workspace.targetSegments;
  alignmentRows.value = workspace.alignments;
  selectedAlignmentId.value = workspace.alignments[0]?.id ?? "";
  projectSummary.value = await kernelClient.getProjectSummary();
  dirty.value = false;
  if (resetView) {
    activeNav.value = "parallel";
    activeMode.value = "review";
    annotationOpen.value = false;
    selectedAnnotationId.value = null;
    resetSearch();
  }
};
onMounted(async () => {
  const savedTheme = localStorage.getItem("jueming-theme");
  const savedScale = Number(localStorage.getItem("jueming-font-scale"));
  sideNavCollapsed.value = localStorage.getItem("jueming-nav-collapsed") === "true";
  if (savedTheme === "eye") theme.value = "eye";
  if (savedScale >= 85 && savedScale <= 130) fontScale.value = savedScale;
  applyUiSettings();
  try { await applySnapshot(await kernelClient.getCurrentProject(), true); } catch { /* Browser preview and first launch use the deterministic fixture. */ }
});
const previousRevisionId = () => { const revisions = projectSnapshot.value?.revisions ?? []; return revisions[revisions.length - 2]?.revision_id ?? projectSummary.value.revision_id; };
const setMode = (mode: WorkspaceMode) => { if (mode === "order" && activeMode.value !== "order") orderBaseline.value = sourceRows.value.map((segment) => segment.id); activeMode.value = mode; activeNav.value = mode === "history" ? "history" : "parallel"; annotationOpen.value = false; exportOpen.value = false; if (mode === "history") void selectHistoryRevision(previousRevisionId()); };
const setNav = (nav: NavId) => { activeNav.value = nav; annotationOpen.value = nav === "annotations"; if (nav === "history") { activeMode.value = "history"; void selectHistoryRevision(previousRevisionId()); } else if (activeMode.value === "history") activeMode.value = "review"; };
const resetImport = () => { sourcePath.value = ""; targetPath.value = ""; projectDirectory.value = ""; sourcePreview.value = null; targetPreview.value = null; previewTab.value = "source"; };
const openImport = async (kind: "new" | "open") => {
  if (kind === "new") { resetImport(); importOpen.value = true; return; }
  const selected = await open({ directory: true, multiple: false, title: "打开决明工程（.jm 文件夹）" });
  if (!selected) return;
  busy.value = true;
  try { await applySnapshot(await kernelClient.openProject(selected), true); notify(`已打开 ${projectSummary.value.name}`); }
  catch (error) { notify(`打开失败：${errorMessage(error)}`); }
  finally { busy.value = false; }
};
const chooseProjectDirectory = async () => { const selected = await open({ directory: true, multiple: false, title: "选择工程保存位置" }); if (selected) projectDirectory.value = selected; };
const chooseTextFile = async (side: "source" | "target") => {
  const selected = await open({ directory: false, multiple: false, title: side === "source" ? "选择中文原文" : "选择英文译文", filters: [{ name: "Text", extensions: ["txt"] }] });
  if (!selected) return;
  if (side === "source") sourcePath.value = selected; else targetPath.value = selected;
  if (/阿古顿巴/i.test(selected)) { sourceEncoding.value = "gb18030"; sourceSegmentation.value = "legacy_tagged_line"; targetSegmentation.value = "legacy_tagged_line"; }
  if (/Akhu Tenpa/i.test(selected)) targetSegmentation.value = "legacy_tagged_line";
  await previewFile(side);
};
const previewFile = async (side: "source" | "target") => {
  const path = side === "source" ? sourcePath.value : targetPath.value;
  if (!path) return;
  const encoding = side === "source" ? sourceEncoding.value : targetEncoding.value;
  const segmentation = side === "source" ? sourceSegmentation.value : targetSegmentation.value;
  busy.value = true;
  try {
    const preview = await kernelClient.previewImport({ input: { kind: "file", path }, profile: makeImportProfile(encoding, segmentation) });
    if (side === "source") sourcePreview.value = preview; else targetPreview.value = preview;
    previewTab.value = side;
    notify(`${side === "source" ? "原文" : "译文"}预览完成：${preview.preview.segments.length} 段`);
  } catch (error) { notify(`预览失败：${errorMessage(error)}`); }
  finally { busy.value = false; }
};
const projectPath = () => `${projectDirectory.value.replace(/[\\/]+$/, "")}\\${projectName.value.trim().replace(/[<>:\"/\\|?*]/g, "_")}.jm`;
const createProjectFromPreview = async () => {
  if (!sourcePath.value || !targetPath.value || !projectDirectory.value || !projectName.value.trim()) { notify("请完整选择原文、译文、保存位置并填写项目名"); return; }
  busy.value = true;
  try {
    const snapshot = await kernelClient.createProject({
      project_path: projectPath(), name: projectName.value.trim(),
      source: { language_id: "zh-CN", title: "中文（原文）", input: { kind: "file", path: sourcePath.value }, profile: makeImportProfile(sourceEncoding.value, sourceSegmentation.value) },
      target: { language_id: "en", title: "English（译文）", input: { kind: "file", path: targetPath.value }, profile: makeImportProfile(targetEncoding.value, targetSegmentation.value) },
    });
    await applySnapshot(snapshot, true); importOpen.value = false; notify(`已创建并保存 ${projectSummary.value.name}`);
  } catch (error) { notify(`创建失败：${errorMessage(error)}`); }
  finally { busy.value = false; }
};
const saveProject = async () => { try { await kernelClient.flushProject(); dirty.value = false; notify("本地存储 · 已保存"); } catch (error) { notify(`保存失败：${errorMessage(error)}`); } };
const saveSegment = async (id: string, text: string) => { const segment = [...sourceRows.value, ...targetRows.value].find((item) => item.id === id); if (!segment) return; const previous = segment.text; segment.text = text; try { await kernelClient.updateSegment(id, text); await applySnapshot(await kernelClient.getCurrentProject()); notify("句段已自动保存并写入本地历史"); } catch (error) { segment.text = previous; notify(`编辑失败：${errorMessage(error)}`); } };
const moveSegment = async (alignmentId: string, direction: "up" | "down") => {
  const alignment = alignmentRows.value.find((item) => item.id === alignmentId); const movedId = alignment?.sourceIds[0]; const index = sourceRows.value.findIndex((item) => item.id === movedId); const targetIndex = direction === "up" ? index - 1 : index + 1;
  if (index < 0 || targetIndex < 0 || targetIndex >= sourceRows.value.length) { notify(direction === "up" ? "已经是第一句" : "已经是最后一句"); return; }
  const previous = sourceRows.value.map((segment) => ({ ...segment })); const next = [...sourceRows.value]; [next[index], next[targetIndex]] = [next[targetIndex], next[index]]; sourceRows.value = next.map((segment, order) => ({ ...segment, order }));
  try { const before = next[targetIndex - 1]?.id; const after = next[targetIndex + 1]?.id; await kernelClient.moveSegment(movedId!, before, after); await applySnapshot(await kernelClient.getCurrentProject()); notify(`已${direction === "up" ? "上移" : "下移"}并自动保存，关系锚点保持稳定`); }
  catch (error) { sourceRows.value = previous; notify(`排序失败：${errorMessage(error)}`); }
};
const reorderSegment = async (segmentId: string, beforeSegmentId: string) => {
  busy.value = true;
  try { await kernelClient.moveSegment(segmentId, undefined, beforeSegmentId); await applySnapshot(await kernelClient.getCurrentProject()); notify("拖拽排序已自动保存，Alignment 关系保持稳定"); }
  catch (error) { notify(`拖拽排序失败：${errorMessage(error)}`); }
  finally { busy.value = false; }
};
const resetOrder = async () => {
  if (!orderBaseline.value.length) { notify("当前会话没有可恢复的排序基线"); return; }
  busy.value = true;
  try { await applySnapshot(await kernelClient.reorderSegments(orderBaseline.value)); notify("已恢复进入顺序模式时的句段顺序"); }
  catch (error) { notify(`恢复顺序失败：${errorMessage(error)}`); }
  finally { busy.value = false; }
};
const linkSegments = async (sourceSegmentIds: string[], targetSegmentIds: string[]) => {
  busy.value = true;
  try {
    let snapshot: ProjectSnapshot;
    try {
      snapshot = await kernelClient.linkSegments(sourceSegmentIds, targetSegmentIds, false);
    } catch (error) {
      const message = errorMessage(error);
      if (!message.includes("AlignmentOccupiedError")) throw error;
      const approved = "__TAURI_INTERNALS__" in window
        ? await confirm("部分句段已经属于 Alignment。替换现有关系会创建新的 Revision。", { title: "替换现有 Alignment？", kind: "warning" })
        : window.confirm("部分句段已经属于 Alignment，是否替换现有关系？");
      if (!approved) return;
      snapshot = await kernelClient.linkSegments(sourceSegmentIds, targetSegmentIds, true);
    }
    await applySnapshot(snapshot);
    notify(`已建立 ${sourceSegmentIds.length}:${targetSegmentIds.length} Alignment`);
  } catch (error) { notify(`Link 失败：${errorMessage(error)}`); }
  finally { busy.value = false; }
};
const unlinkAlignment = async (alignmentId: string) => {
  busy.value = true;
  try { await applySnapshot(await kernelClient.unlinkAlignment(alignmentId)); notify("已解除 Alignment，句段保持未对齐"); }
  catch (error) { notify(`Unlink 失败：${errorMessage(error)}`); }
  finally { busy.value = false; }
};
const mergeAlignments = async (alignmentIds: string[]) => {
  busy.value = true;
  try { await applySnapshot(await kernelClient.mergeAlignments(alignmentIds)); notify(`已合并 ${alignmentIds.length} 个 Alignment`); }
  catch (error) { notify(`Merge 失败：${errorMessage(error)}`); }
  finally { busy.value = false; }
};
const splitAlignment = async (alignmentId: string, sourceGroups: string[][], targetGroups: string[][]) => {
  busy.value = true;
  try { await applySnapshot(await kernelClient.splitAlignment(alignmentId, sourceGroups, targetGroups)); notify("已拆分 Alignment"); }
  catch (error) { notify(`Split 失败：${errorMessage(error)}`); }
  finally { busy.value = false; }
};
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
    notify(`找到 ${response.hits.length} 条结果`);
  } catch (error) { searchResults.value = []; notify(`搜索失败：${errorMessage(error)}`); }
  finally { busy.value = false; }
};
const currentReplaceRequest = (): ReplacePreviewRequest => ({ ...makeSearchRequest({ query: searchQuery.value, side: searchSide.value, regex: searchRegex.value, caseSensitive: searchCaseSensitive.value }), replacement: replacement.value });
const previewReplacement = async () => { try { const response = await kernelClient.previewReplace(currentReplaceRequest()); notify(`替换预览包含 ${response.items.length} 个句段`); } catch (error) { notify(`替换预览失败：${errorMessage(error)}`); } };
const applyReplacement = async (preview: UiReplacePreview) => { busy.value = true; try { await applySnapshot(await kernelClient.applyReplace(currentReplaceRequest(), preview.resultIds)); notify(`已原子替换 ${preview.resultIds.length} 个句段`); await runSearch({ query: searchQuery.value, side: searchSide.value, regex: searchRegex.value, caseSensitive: searchCaseSensitive.value }); } catch (error) { notify(`替换失败：${errorMessage(error)}`); } finally { busy.value = false; } };
const selectSearchResult = (result: SearchResult) => { selectedAlignmentId.value = result.alignmentId ?? ""; activeNav.value = "parallel"; activeMode.value = "review"; annotationOpen.value = false; };
const toggleBookmark = async (segmentId: string, alignmentId: string) => {
  const existing = projectSnapshot.value?.bookmarks.find((bookmark) => bookmark.segment_id === segmentId && bookmark.alignment_id === alignmentId);
  try { await applySnapshot(existing ? await kernelClient.deleteBookmark(existing.bookmark_id) : await kernelClient.createBookmark(segmentId, alignmentId, `书签 ${segmentLabel(segmentId)}`)); notify(existing ? "已移除书签" : "已添加书签"); }
  catch (error) { notify(`书签操作失败：${errorMessage(error)}`); }
};
const openBookmark = (alignmentId: string | null) => { if (!alignmentId) return; selectedAlignmentId.value = alignmentId; activeNav.value = "parallel"; activeMode.value = "review"; annotationOpen.value = false; };
const removeBookmark = async (bookmarkId: string) => { try { await applySnapshot(await kernelClient.deleteBookmark(bookmarkId)); notify("已移除书签"); } catch (error) { notify(`移除书签失败：${errorMessage(error)}`); } };
const resetSearch = () => { searchQuery.value = ""; replacement.value = ""; searchResults.value = []; };
const copyHistoryValue = async (value: string) => { try { await navigator.clipboard.writeText(value); notify("已复制到剪贴板"); } catch { notify("复制失败：系统剪贴板不可用"); } };
const toggleSideNav = () => { sideNavCollapsed.value = !sideNavCollapsed.value; localStorage.setItem("jueming-nav-collapsed", String(sideNavCollapsed.value)); };
const annotationRequest = (draft: AnnotationDraft) => ({ title: draft.title, body: draft.body, status: draft.status, linked_segment_ids: draft.links.map((link) => link.segmentId), alignment_id: selectedAlignmentId.value || null });
const createAnnotation = async (draft: AnnotationDraft) => { const alignment = alignmentRows.value.find((item) => item.id === selectedAlignmentId.value); const linked = draft.links.length ? draft : { ...draft, links: [...(alignment?.sourceIds ?? []).map((segmentId) => ({ side: "source" as const, segmentId })), ...(alignment?.targetIds ?? []).map((segmentId) => ({ side: "target" as const, segmentId }))] }; try { await applySnapshot(await kernelClient.createAnnotation(annotationRequest(linked))); notify("批注已创建并自动保存"); } catch (error) { notify(`新建批注失败：${errorMessage(error)}`); } };
const editAnnotation = async (annotationId: string, draft: AnnotationDraft) => { try { await applySnapshot(await kernelClient.updateAnnotation(annotationId, annotationRequest(draft))); notify("批注已更新"); } catch (error) { notify(`更新批注失败：${errorMessage(error)}`); } };
const deleteAnnotation = async (annotationId: string) => { const approved = "__TAURI_INTERNALS__" in window ? await confirm("删除批注会保留在 Revision 历史中。", { title: "删除批注？", kind: "warning" }) : window.confirm("删除批注？"); if (!approved) return; try { await applySnapshot(await kernelClient.deleteAnnotation(annotationId)); notify("批注已删除"); } catch (error) { notify(`删除批注失败：${errorMessage(error)}`); } };
const resolveAnnotation = async (annotationId: string) => { try { await applySnapshot(await kernelClient.resolveAnnotation(annotationId)); notify("批注已标记为解决"); } catch (error) { notify(`解决批注失败：${errorMessage(error)}`); } };
const exportProject = async (format: ExportFormat) => { exportOpen.value = false; const outputPath = await saveDialog({ title: `导出 ${format.toUpperCase()}`, defaultPath: `${projectSummary.value.name}.${format}`, filters: [{ name: format.toUpperCase(), extensions: [format] }] }); if (!outputPath) return; busy.value = true; try { await kernelClient.exportProject(format, outputPath); notify(`已导出 ${outputPath}`); } catch (error) { notify(`导出失败：${errorMessage(error)}`); } finally { busy.value = false; } };
function applyUiSettings() { document.documentElement.dataset.theme = theme.value; document.documentElement.style.fontSize = `${fontScale.value}%`; localStorage.setItem("jueming-theme", theme.value); localStorage.setItem("jueming-font-scale", String(fontScale.value)); }
const modeSelect = (event: Event) => setMode((event.target as HTMLSelectElement).value as WorkspaceMode);
const handleShortcut = (event: KeyboardEvent) => {
  if (!(event.ctrlKey || event.metaKey)) return;
  const key = event.key.toLowerCase();
  if (key === "s") { event.preventDefault(); void saveProject(); return; }
  if (key === "f" && event.shiftKey) { event.preventDefault(); setNav("search"); return; }
  const target = event.target as HTMLElement | null;
  const editingText = target?.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target?.tagName ?? "");
  if (editingText) return;
  if (key === "z" && !event.shiftKey) { event.preventDefault(); void performUndo(); }
  else if (key === "y" || (key === "z" && event.shiftKey)) { event.preventDefault(); void performRedo(); }
};
onMounted(() => window.addEventListener("keydown", handleShortcut));
onBeforeUnmount(() => window.removeEventListener("keydown", handleShortcut));
</script>

<template>
  <div class="app-shell">
    <header class="window-chrome" data-tauri-drag-region><div class="brand-lockup" data-tauri-drag-region><img :src="brandIcon" alt="" /><span class="brand-title" data-tauri-drag-region>决明对齐器 <em data-tauri-drag-region>Jueming Aligner</em></span></div><div class="window-actions"><button type="button" title="最小化" @click="windowAction('minimize')"><Minimize2 :size="16" /></button><button type="button" title="最大化" @click="windowAction('maximize')"><Maximize2 :size="15" /></button><button type="button" title="关闭" @click="windowAction('close')"><X :size="18" /></button></div></header>
    <header class="app-toolbar"><div class="toolbar-left"><button class="toolbar-button" type="button" title="新建工程" @click="openImport('new')"><FilePlus2 :size="19" />新建</button><button class="toolbar-button" type="button" title="打开本地工程" @click="openImport('open')"><FolderOpen :size="19" />打开</button><button class="toolbar-button" type="button" title="保存工程" @click="saveProject"><Save :size="19" />保存</button><div class="toolbar-divider"></div><div class="toolbar-actions"><div class="toolbar-button--export-wrap"><button class="toolbar-button toolbar-button--export" type="button" title="导出工程" @click="exportOpen = !exportOpen"><Download :size="19" />导出<ChevronDown :size="15" /></button><div v-if="exportOpen" class="export-menu"><button type="button" @click="exportProject('txt')">TXT 文本</button><button type="button" @click="exportProject('json')">JSON 工程</button><button type="button" @click="exportProject('xml')">XML 对齐</button></div></div><div class="toolbar-divider"></div><button class="toolbar-button" type="button" :disabled="!canUndo || busy" title="撤销" @click="performUndo"><Undo2 :size="19" />撤销</button><button class="toolbar-button" type="button" :disabled="!canRedo || busy" title="重做" @click="performRedo"><Redo2 :size="19" />重做</button><div class="toolbar-divider"></div><button class="toolbar-button" type="button" @click="setNav('settings')"><Settings2 :size="19" />设置</button></div></div><div class="mode-control" :class="`mode-control--${activeMode}`"><component :is="currentMode.icon" :size="17" /><span>{{ currentMode.label }} <small>{{ currentMode.hint }}</small></span><ChevronDown :size="15" /><select :value="activeMode" aria-label="切换工作模式" @change="modeSelect"><option v-for="item in modeItems" :key="item.id" :value="item.id">{{ item.label }} {{ item.hint }}</option></select></div></header>
    <div class="content-grid" :class="{ 'content-grid--collapsed': sideNavCollapsed, 'content-grid--annotation': annotationOpen }">
      <nav class="side-nav" :class="{ 'side-nav--collapsed': sideNavCollapsed }"><button v-for="item in navItems" :key="item.id" class="nav-item" :class="{ 'nav-item--active': activeNav === item.id }" type="button" :title="item.label" @click="setNav(item.id)"><component :is="item.icon" :size="22" :stroke-width="activeNav === item.id ? 2.2 : 1.8" /><span class="nav-label">{{ item.label }}<small v-if="item.hint">{{ item.hint }}</small></span></button><div class="nav-collapse"><button class="nav-item" type="button" :title="sideNavCollapsed ? '展开侧栏' : '收起侧栏'" @click="toggleSideNav"><ChevronDown :size="21" :style="{ transform: sideNavCollapsed ? 'rotate(-90deg)' : 'rotate(90deg)' }" /><span class="nav-label">{{ sideNavCollapsed ? '展开' : '收起' }}</span></button></div></nav>
      <main class="main-stage">
        <ParallelWorkspace v-if="activeNav === 'parallel' || activeNav === 'annotations'" :mode="activeMode" :source-segments="sourceRows" :target-segments="targetRows" :alignments="alignmentRows" :selected-alignment-id="selectedAlignmentId" :bookmarked-alignment-ids="bookmarkedAlignmentIds" @select="selectedAlignmentId = $event" @save-segment="saveSegment" @cancel-edit="notify('已取消编辑')" @move="moveSegment" @reorder="reorderSegment" @reset-order="resetOrder" @link="linkSegments" @unlink="unlinkAlignment" @merge="mergeAlignments" @split="splitAlignment" @bookmark="toggleBookmark" @status="notify" />
        <SearchReplaceWorkspace v-else-if="activeNav === 'search'" v-model:query="searchQuery" v-model:side="searchSide" v-model:regex="searchRegex" v-model:case-sensitive="searchCaseSensitive" v-model:replacement="replacement" :results="searchResults" :project-label="projectSummary.name" :loading="busy" @search="runSearch" @select-result="selectSearchResult" @replace-preview="previewReplacement" @apply-replace="applyReplacement" @reset="resetSearch" />
        <HistoryWorkspace v-else-if="activeNav === 'history'" :revisions="revisionItems" :diff="historyDiff" :base-revision-id="baseRevisionId" :selected-revision-id="selectedRevisionId" :current-revision-id="projectSummary.revision_id" :loading="busy" @select-revision="selectHistoryRevision" @compare="compareHistory" @restore="restoreHistory" @copy-value="copyHistoryValue" />
        <section v-else-if="activeNav === 'settings'" class="aux-view settings-view"><Settings2 :size="28" /><h2>设置</h2><p>工程内容完全离线保存；外观设置保存在本机。</p><div class="settings-card"><label>显示主题<select v-model="theme" @change="applyUiSettings"><option value="light">明亮</option><option value="eye">护眼</option></select></label><label>界面缩放 <input v-model.number="fontScale" type="range" min="85" max="130" step="5" @change="applyUiSettings" /><output>{{ fontScale }}%</output></label><span><Monitor :size="16" />不连接云端服务，不上传原文或译文。</span></div></section>
        <section v-else-if="activeNav === 'bookmarks'" class="aux-view bookmarks-view"><Star :size="28" /><h2>书签</h2><p>书签锚定稳定的 Alignment ID，不受文本编辑和语句重排影响。</p><div v-if="projectSnapshot?.bookmarks.length" class="bookmark-list"><article v-for="bookmark in projectSnapshot.bookmarks" :key="bookmark.bookmark_id"><button type="button" @click="openBookmark(bookmark.alignment_id)"><Star :size="16" fill="currentColor" /><span><b>{{ bookmark.label }}</b><small>{{ segmentLabel(bookmark.segment_id) }} · {{ alignmentLabel(bookmark.alignment_id) }}</small></span></button><button type="button" title="移除书签" @click="removeBookmark(bookmark.bookmark_id)"><X :size="15" /></button></article></div><p v-else class="empty-copy">当前工程暂无书签。可在平行视图中点击句段右侧的星标。</p></section>
        <section v-else class="aux-view project-view"><Folder :size="28" /><h2>项目</h2><p>{{ projectSummary.name }}</p><div class="project-summary-card"><span>{{ projectSummary.source_count }}</span><small>中文句段</small><span>{{ projectSummary.target_count }}</span><small>English segments</small><span>{{ projectSummary.alignment_count }}</span><small>Alignment</small></div><div class="project-actions"><button type="button" class="primary-button" @click="openImport('open')">打开工程</button><button type="button" class="secondary-button" @click="openImport('new')">新建工程</button></div></section>
      </main>
      <AnnotationPanel v-if="annotationOpen" v-model:active-filter="annotationFilter" :annotations="annotations" :selected-id="selectedAnnotationId" @select="selectedAnnotationId = $event" @create="createAnnotation" @edit="editAnnotation" @delete="deleteAnnotation" @resolve="resolveAnnotation" @close="annotationOpen = false; if (activeNav === 'annotations') activeNav = 'parallel'" />
    </div>
    <footer class="bottom-status"><div class="footer-left"><div class="footer-project"><strong>项目：</strong>{{ projectSummary.name }}</div><div class="footer-separator"></div><div class="footer-project">文件：<span>{{ projectSummary.source_label }}</span><Link2 :size="14" /><span>{{ projectSummary.target_label }}</span></div><div class="footer-separator"></div><div>对齐状态：<span class="footer-status-pill">{{ projectSummary.source_unlinked_count + projectSummary.target_unlinked_count === 0 ? '1:1' : '待校对' }}</span></div></div><div class="footer-right"><span class="save-state" :class="{ 'save-state--dirty': dirty }"><Monitor :size="14" />{{ busy ? '处理中…' : dirty ? '有未保存更改' : statusMessage }}</span><span>已处理：{{ projectSummary.alignment_count }} / {{ Math.max(projectSummary.source_count, projectSummary.target_count) }}</span><span>进度：</span><div class="progress-track"><span :style="{ width: `${Math.round(100 * projectSummary.alignment_count / Math.max(1, projectSummary.source_count, projectSummary.target_count))}%` }"></span></div><span>{{ Math.round(100 * projectSummary.alignment_count / Math.max(1, projectSummary.source_count, projectSummary.target_count)) }}%</span></div></footer>
    <div v-if="importOpen" class="modal-backdrop" @click.self="importOpen = false"><section class="import-modal" role="dialog" aria-modal="true" aria-labelledby="import-title"><header><div><span class="eyebrow">NEW PROJECT</span><h2 id="import-title">新建平行工程</h2></div><button type="button" title="关闭导入向导" @click="importOpen = false"><X :size="19" /></button></header><div class="import-project-grid"><label>项目名称<input v-model="projectName" placeholder="例如：阿古顿巴_中英对齐" /></label><label>保存位置<div class="compact-picker"><input v-model="projectDirectory" readonly placeholder="选择父文件夹" /><button type="button" @click="chooseProjectDirectory">浏览…</button></div></label></div><div class="import-source-grid"><section><div class="import-path"><label for="source-path">中文（原文）</label><div><FolderOpen :size="17" /><input id="source-path" v-model="sourcePath" readonly placeholder="选择 TXT 原文" /><button type="button" @click="chooseTextFile('source')">浏览…</button></div></div><div class="import-settings"><label>分段方式 <select v-model="sourceSegmentation" @change="previewFile('source')"><option value="non_empty_line">非空行</option><option value="sentence_rules">规则分句</option><option value="legacy_tagged_line">SISU 标记行</option></select></label><label>编码 <select v-model="sourceEncoding" @change="previewFile('source')"><option value="utf8">UTF-8</option><option value="utf8-bom">UTF-8 BOM</option><option value="gb18030">GB18030</option></select></label></div></section><section><div class="import-path"><label for="target-path">English（译文）</label><div><FolderOpen :size="17" /><input id="target-path" v-model="targetPath" readonly placeholder="选择 TXT 译文" /><button type="button" @click="chooseTextFile('target')">浏览…</button></div></div><div class="import-settings"><label>分段方式 <select v-model="targetSegmentation" @change="previewFile('target')"><option value="non_empty_line">非空行</option><option value="sentence_rules">规则分句</option><option value="legacy_tagged_line">SISU 标记行</option></select></label><label>编码 <select v-model="targetEncoding" @change="previewFile('target')"><option value="utf8">UTF-8</option><option value="utf8-bom">UTF-8 BOM</option><option value="gb18030">GB18030</option></select></label></div></section></div><div class="preview-tabs"><button :class="{ active: previewTab === 'source' }" type="button" @click="previewTab = 'source'">中文（原文） · {{ sourcePreview?.preview.segments.length ?? 0 }} 段</button><button :class="{ active: previewTab === 'target' }" type="button" @click="previewTab = 'target'">English（译文） · {{ targetPreview?.preview.segments.length ?? 0 }} 段</button></div><div class="segment-preview"><div v-if="!activePreview" class="preview-empty">选择文本后会以指定编码和规则生成真实预览。</div><div v-for="text in activePreview?.preview.segments ?? []" :key="text.ordinal" class="preview-line"><span>{{ String(text.ordinal + 1).padStart(2, '0') }}</span><p>{{ text.content }}</p></div></div><footer><span class="import-contract">源文件只读；工程写入 <b>.jm</b> 文件夹并保留稳定 ID。</span><button class="secondary-button" type="button" @click="importOpen = false">取消</button><button class="primary-button" type="button" :disabled="busy" @click="createProjectFromPreview"><Check :size="15" />创建并打开工程</button></footer></section></div>
  </div>
</template>

<style scoped>
.toolbar-button--export-wrap { position: relative; }.export-menu { position: absolute; z-index: 8; top: 41px; left: 0; width: 142px; padding: 6px; border: 1px solid var(--line); border-radius: 7px; background: #fff; box-shadow: 0 10px 25px rgb(35 55 38 / 14%); }.export-menu button { display: block; width: 100%; padding: 8px 10px; border: 0; border-radius: 4px; background: transparent; text-align: left; cursor: pointer; }.export-menu button:hover { color: var(--green-900); background: var(--green-050); }.mode-control small { margin-left: 2px; color: var(--green-700); font-size: 12px; font-weight: 500; }.mode-control--edit { border-color: #e6d19d; color: #916714; background: #fffaf0; }.mode-control--edit small { color: #af8321; }.mode-control--history { border-color: #c7d4df; color: #536b7a; background: #f8fbfd; }
.search-view { display: flex; flex-direction: column; height: 100%; min-height: 0; padding: 25px 28px; }.search-bar { display: flex; align-items: center; gap: 10px; }.search-bar > label { width: 42px; font-size: 14px; }.search-input-wrap { display: flex; align-items: center; width: min(570px, 48vw); height: 42px; padding: 0 12px; border: 1px solid #c9d1ca; border-radius: 7px 0 0 7px; color: var(--ink-500); }.search-input-wrap input { flex: 1; min-width: 0; padding: 0 9px; border: 0; outline: none; }.search-input-wrap button { padding: 3px; border: 0; background: transparent; color: var(--ink-500); cursor: pointer; }.primary-search, .primary-button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; height: 42px; padding: 0 22px; border: 1px solid var(--green-900); border-radius: 0 6px 6px 0; color: #fff; background: var(--green-900); cursor: pointer; }.filter-button { height: 42px; margin-left: -10px; padding: 0 12px; border: 1px solid #c9d1ca; border-radius: 0 7px 7px 0; background: #fff; cursor: pointer; }.check-label { display: inline-flex; align-items: center; gap: 6px; margin-left: 22px; color: var(--ink-700); font-size: 13px; }.current-project { display: inline-flex; align-items: center; gap: 4px; margin-left: auto; color: var(--green-900); font-size: 13px; }.search-summary { padding: 15px 5px 12px; color: var(--ink-700); font-size: 13px; }.search-summary strong { color: var(--green-900); }.search-summary span { color: var(--ink-500); }.search-table { min-height: 0; overflow: auto; border: 1px solid var(--line); border-radius: 7px; }.search-table-head, .search-row { display: grid; grid-template-columns: 85px 1.2fr 95px 1.45fr 90px; align-items: center; gap: 14px; padding: 0 17px; }.search-table-head { height: 43px; color: var(--ink-700); background: #f8faf8; font-size: 12px; }.search-row { width: 100%; min-height: 57px; border: 0; border-top: 1px solid var(--line); background: #fff; color: var(--ink-900); font-size: 13px; text-align: left; cursor: pointer; }.search-row:hover { background: #f5fbf3; }.search-row > span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }.search-row mark { width: max-content; padding: 2px 7px; color: #72591b; background: #fff1c9; font-weight: 660; }
.aux-view { display: flex; flex-direction: column; align-items: flex-start; justify-content: center; gap: 13px; max-width: 540px; height: 100%; margin: auto; padding: 32px; color: var(--green-900); }.aux-view h2 { margin: 0; color: var(--ink-900); font-size: 22px; }.aux-view p { margin: 0; color: var(--ink-700); line-height: 1.6; }.secondary-button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; height: 38px; padding: 0 17px; border: 1px solid #bdcabf; border-radius: 6px; color: var(--ink-700); background: #fff; cursor: pointer; }.aux-view .primary-button { border-radius: 6px; }.project-summary-card { display: grid; grid-template-columns: auto auto; gap: 3px 14px; margin: 4px 0; padding: 14px 18px; border: 1px solid var(--line); border-radius: 8px; color: var(--ink-900); }.project-summary-card span { font-size: 23px; color: var(--green-900); }.project-summary-card small { color: var(--ink-500); font-size: 11px; }
.settings-card { display: grid; gap: 16px; width: min(470px, 70vw); padding: 20px; border: 1px solid var(--line); border-radius: 8px; color: var(--ink-700); background: var(--paper); }.settings-card label { display: grid; grid-template-columns: 105px 1fr auto; align-items: center; gap: 10px; font-size: 13px; }.settings-card select { height: 34px; padding: 0 9px; border: 1px solid #c8d3c9; border-radius: 5px; color: var(--ink-900); background: var(--paper); }.settings-card output { min-width: 42px; color: var(--green-900); }.settings-card > span { display: flex; align-items: center; gap: 8px; padding-top: 13px; border-top: 1px solid var(--line); color: var(--green-700); font-size: 12px; }.bookmark-list { display: grid; gap: 8px; width: min(560px, 72vw); max-height: 420px; overflow: auto; }.bookmark-list article { display: grid; grid-template-columns: 1fr 36px; align-items: center; border: 1px solid var(--line); border-radius: 7px; background: var(--paper); }.bookmark-list button { display: flex; align-items: center; gap: 11px; min-width: 0; padding: 11px 13px; border: 0; color: var(--green-900); background: transparent; text-align: left; cursor: pointer; }.bookmark-list button:last-child { justify-content: center; padding-inline: 8px; color: var(--ink-500); }.bookmark-list span { display: grid; gap: 3px; }.bookmark-list small { color: var(--ink-500); font-family: ui-monospace, Consolas, monospace; }.empty-copy { padding: 20px; border: 1px dashed #c8d3c9; border-radius: 7px; }.project-actions { display: flex; gap: 9px; }
.modal-backdrop { position: fixed; z-index: 20; inset: 0; display: grid; place-items: center; background: rgb(31 42 34 / 22%); }.import-modal { display: flex; flex-direction: column; width: min(750px, calc(100vw - 80px)); max-height: calc(100vh - 90px); overflow: hidden; border: 1px solid #cbd6cc; border-radius: 10px; background: #fff; box-shadow: 0 22px 70px rgb(29 48 32 / 23%); }.import-modal > header, .import-modal > footer { display: flex; align-items: center; justify-content: space-between; padding: 20px 24px; border-bottom: 1px solid var(--line); }.import-modal > header button { padding: 5px; border: 0; background: transparent; color: var(--ink-500); cursor: pointer; }.import-modal > header h2 { margin: 3px 0 0; color: var(--ink-900); font-size: 21px; }.eyebrow { color: var(--green-700); font-size: 10px; font-weight: 700; letter-spacing: .1em; }.import-path { padding: 18px 24px 13px; }.import-path label { display: block; margin-bottom: 8px; color: var(--ink-700); font-size: 13px; }.import-path > div { display: flex; align-items: center; gap: 8px; height: 40px; padding: 0 11px; border: 1px solid #c8d3c9; border-radius: 6px; color: var(--ink-500); }.import-path input { flex: 1; min-width: 0; border: 0; outline: none; }.import-path button { height: 29px; padding: 0 10px; border: 1px solid #c4d0c5; border-radius: 4px; color: var(--green-900); background: #fff; cursor: pointer; }.import-path small { display: block; margin-top: 7px; color: var(--ink-500); font-size: 11px; }.preview-tabs { display: flex; gap: 4px; padding: 0 24px; border-bottom: 1px solid var(--line); }.preview-tabs button { padding: 11px 13px; border: 0; border-bottom: 2px solid transparent; color: var(--ink-500); background: transparent; font-size: 13px; cursor: pointer; }.preview-tabs button.active { border-bottom-color: var(--green-700); color: var(--green-900); font-weight: 600; }.segment-preview { min-height: 170px; max-height: 255px; overflow: auto; padding: 5px 24px; }.preview-line { display: grid; grid-template-columns: 30px 1fr; gap: 11px; padding: 8px 0; border-bottom: 1px solid #edf1ed; }.preview-line span { color: var(--ink-500); font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: 11px; }.preview-line p { margin: 0; color: var(--ink-900); font-size: 13px; line-height: 1.5; }.import-settings { display: flex; gap: 24px; padding: 12px 24px; border-top: 1px solid var(--line); background: #fbfcfb; }.import-settings label { display: flex; align-items: center; gap: 7px; color: var(--ink-700); font-size: 12px; }.import-settings select { height: 30px; border: 1px solid #cbd6cc; border-radius: 4px; color: var(--ink-700); background: #fff; }.import-modal > footer { justify-content: flex-end; gap: 10px; border-top: 1px solid var(--line); border-bottom: 0; }.import-modal > footer .primary-button { border-radius: 6px; height: 38px; }
.import-modal { width: min(980px, calc(100vw - 80px)); }.import-project-grid, .import-source-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; padding: 16px 24px; }.import-project-grid { border-bottom: 1px solid var(--line); background: #fbfcfb; }.import-project-grid > label { display: grid; gap: 7px; color: var(--ink-700); font-size: 12px; }.import-project-grid > label > input, .compact-picker { height: 38px; border: 1px solid #c8d3c9; border-radius: 6px; background: #fff; }.import-project-grid > label > input { padding: 0 11px; outline: none; }.compact-picker { display: flex; overflow: hidden; }.compact-picker input { flex: 1; min-width: 0; padding: 0 11px; border: 0; outline: none; }.compact-picker button { padding: 0 13px; border: 0; border-left: 1px solid #c8d3c9; color: var(--green-900); background: #fff; cursor: pointer; }.import-source-grid { padding-top: 3px; padding-bottom: 3px; }.import-source-grid > section { min-width: 0; }.import-source-grid .import-path { padding: 12px 0 8px; }.import-source-grid .import-settings { justify-content: space-between; gap: 10px; padding: 8px 0 12px; border-top: 0; background: transparent; }.preview-empty { display: grid; min-height: 160px; place-items: center; color: var(--ink-500); font-size: 13px; }.import-contract { margin-right: auto; color: var(--ink-500); font-size: 11px; }.primary-button:disabled { opacity: .55; cursor: wait; }
.annotation-panel { position: fixed; z-index: 10; top: 116px; right: 0; bottom: 58px; display: flex; flex-direction: column; width: 355px; overflow: auto; border-left: 1px solid #ccd9cd; background: #fbfdfb; box-shadow: -12px 0 30px rgb(32 51 35 / 10%); }.annotation-panel header { display: flex; align-items: flex-start; justify-content: space-between; padding: 19px 18px 12px; }.annotation-panel header button { padding: 5px; border: 0; background: transparent; color: var(--ink-500); cursor: pointer; }.annotation-panel h2 { margin: 4px 0 0; color: var(--ink-900); font-size: 16px; }.annotation-panel h2 small { display: inline-block; margin-left: 3px; padding: 2px 6px; border-radius: 10px; color: var(--green-900); background: var(--green-100); font-size: 11px; }.annotation-filter { display: flex; gap: 4px; padding: 0 13px 13px; border-bottom: 1px solid var(--line); }.annotation-filter button { padding: 7px 8px; border: 1px solid transparent; border-radius: 5px; color: var(--ink-500); background: transparent; font-size: 12px; cursor: pointer; }.annotation-filter button.active { border-color: #b3d5b7; color: var(--green-900); background: #f3faf1; }.annotation-card { display: grid; grid-template-columns: 25px 1fr; gap: 7px; margin: 13px 13px 0; padding: 13px 11px; border: 1px solid #d3dde5; border-radius: 8px; background: #fff; }.annotation-card--draft { border-color: #d3c4ec; }.annotation-number { display: grid; place-items: center; width: 22px; height: 22px; border-radius: 6px; color: #fff; background: #9864d5; font-size: 12px; }.annotation-number--green { background: #3c9a5a; }.annotation-state { display: flex; align-items: center; gap: 7px; color: var(--ink-900); font-size: 12px; font-weight: 650; }.annotation-state span { color: var(--ink-500); font-weight: 500; }.annotation-state time { margin-left: auto; color: var(--ink-500); font-size: 10px; font-weight: 400; }.annotation-card h3 { margin: 12px 0 7px; color: var(--ink-900); font-size: 14px; }.annotation-card p { margin: 0; color: var(--ink-700); font-size: 12px; line-height: 1.55; }.annotation-links { display: flex; flex-direction: column; gap: 5px; margin-top: 10px; padding: 8px; border-radius: 5px; background: #f8faf8; color: var(--ink-700); font-size: 11px; }.annotation-links b { float: right; color: var(--ink-500); font-weight: 500; }.annotation-card footer { display: flex; gap: 15px; margin-top: 10px; padding-top: 9px; border-top: 1px solid var(--line); }.annotation-card footer button, .resolve-button, .new-annotation { display: inline-flex; align-items: center; gap: 5px; border: 0; color: var(--ink-700); background: transparent; font-size: 12px; cursor: pointer; }.resolve-button { margin-top: 11px; padding: 6px 9px; border: 1px solid #b5dab9; border-radius: 5px; color: var(--green-900); background: #f2faf1; }.new-annotation { justify-content: center; margin: 13px; padding: 10px; border: 1px solid #abd1af; border-radius: 6px; color: var(--green-900); background: #f4fbf2; }
</style>
