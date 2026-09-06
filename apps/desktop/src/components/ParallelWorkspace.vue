<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { ArrowDown, ArrowUp, ChevronsDown, ChevronsUp, GripVertical, Link2, Link2Off, LockKeyhole, Merge, RotateCcw, Scissors, Search, X } from "@lucide/vue";
import type { EditSession } from "../composables/useViewModeController";
import type { AlignmentDto, AlignmentGapEdge, LanguageSide, SegmentDto, WorkspaceMode } from "../domain/kernel-client";
import { buildAlignmentBlocks, type AlignmentBlockView, type OrderSelection } from "../domain/workspace-projection";
import SegmentContentDialog, { type ContentOperationRequest } from "./SegmentContentDialog.vue";
import AlignmentUngroupDialog from "./AlignmentUngroupDialog.vue";
import AlignedWorkspaceViewport from "./parallel-workspace/AlignedWorkspaceViewport.vue";
import OrderDragOverlay from "./parallel-workspace/OrderDragOverlay.vue";
import { useOrderDragAndDrop, type ReorderIntent } from "../composables/useOrderDragAndDrop";
import { useTransientSegmentJump } from "../composables/useTransientSegmentJump";

const props = withDefaults(defineProps<{
  mode: WorkspaceMode;
  sourceSegments: SegmentDto[];
  targetSegments: SegmentDto[];
  alignments: AlignmentDto[];
  bookmarkedSegmentIds?: string[];
  annotatedSegmentIds?: string[];
  selectedAlignmentId: string;
  editSession?: EditSession | null;
  trackpadOptimized?: boolean;
  smoothNavigation?: boolean;
  writable?: boolean;
}>(), { bookmarkedSegmentIds: () => [], annotatedSegmentIds: () => [], selectedAlignmentId: "", editSession: null, trackpadOptimized: false, smoothNavigation: true, writable: false });

const emit = defineEmits<{
  select: [alignmentId: string];
  requestEdit: [segmentId: string, alignmentId: string];
  editDraft: [draft: string];
  commitEdit: [exitAfterSave: boolean];
  cancelEdit: [];
  escapeEdit: [];
  move: [side: LanguageSide, segmentId: string, direction: "up" | "down"];
  reorder: [side: LanguageSide, orderedSegmentIds: string[], intent: ReorderIntent];
  insertGap: [segmentId: string, edge: AlignmentGapEdge];
  resetOrder: [];
  link: [sourceIds: string[], targetIds: string[]];
  unlink: [alignmentId: string];
  mergeSegments: [segmentIds: string[], mergedContent: string];
  splitSegment: [segmentId: string, parts: string[]];
  group: [alignmentIds: string[], unlinkedSegmentIds: string[]];
  ungroup: [alignmentId: string, sourceGroups: string[][], targetGroups: string[][]];
  bookmark: [segmentId: string, alignmentId: string | null];
  annotation: [segmentId: string, alignmentId: string | null];
  status: [message: string];
}>();

const bookmarked = computed(() => new Set(props.bookmarkedSegmentIds));
const annotated = computed(() => new Set(props.annotatedSegmentIds));
const selectedSourceIds = ref(new Set<string>());
const selectedTargetIds = ref(new Set<string>());
const selectedAlignmentIds = ref(new Set<string>());
const orderSelection = ref<OrderSelection | null>(null);
const hasSegmentSelection = computed(() => selectedSourceIds.value.size > 0 || selectedTargetIds.value.size > 0);
const hasOperationSelection = computed(() => hasSegmentSelection.value || selectedAlignmentIds.value.size > 0);
type AlignedWorkspaceExposed = {
  focusAlignment: (alignmentId: string) => Promise<boolean>;
  focusSegment: (segmentId: string) => Promise<boolean>;
  focusSegments: (sourceIds: string[], targetIds: string[]) => Promise<boolean>;
  focusViewport: () => void;
  measure: () => Promise<void>;
};
const alignedWorkspaceRef = ref<AlignedWorkspaceExposed | null>(null);
const listScrollbarWidth = ref(0);
const findInputRef = ref<HTMLInputElement | null>(null);
const findOpen = ref(false);
const findQuery = ref("");
const deferredFindQuery = ref("");
const findCursor = ref(-1);
const transientHighlightIds = ref(new Set<string>());
const contentDialog = ref<ContentOperationRequest | null>(null);
const ungroupDialogAlignment = ref<AlignmentDto | null>(null);
const suppressedSelectionFocusId = ref("");
let highlightTimer: ReturnType<typeof setTimeout> | null = null;
let findDebounceTimer: ReturnType<typeof setTimeout> | null = null;

const rows = computed(() => buildAlignmentBlocks(props.sourceSegments, props.targetSegments, props.alignments));

const rowBySegmentId = computed(() => {
  const result = new Map<string, AlignmentBlockView>();
  for (const row of rows.value) [...row.sourceSegments, ...row.targetSegments].forEach((segment) => result.set(segment.id, row));
  return result;
});
const alignmentBySegmentId = computed(() => {
  const result = new Map<string, string>();
  for (const alignment of props.alignments) {
    for (const segmentId of [...alignment.sourceIds, ...alignment.targetIds]) result.set(segmentId, alignment.id);
  }
  return result;
});

const searchableSegments = computed(() => [...props.sourceSegments, ...props.targetSegments].map((segment) => ({
  segmentId: segment.id,
  normalizedText: segment.text.toLocaleLowerCase(),
})));
const findPending = computed(() => findQuery.value.trim().toLocaleLowerCase() !== deferredFindQuery.value);
const findMatches = computed(() => {
  const query = deferredFindQuery.value;
  if (!query) return [];
  return searchableSegments.value
    .filter((segment) => segment.normalizedText.includes(query))
    .map((segment) => ({ segmentId: segment.segmentId, alignmentId: rowBySegmentId.value.get(segment.segmentId)?.alignmentId ?? "" }))
    .filter((match) => match.alignmentId);
});

const pulseHighlight = async (alignmentIds: string[]) => {
  if (highlightTimer !== null) clearTimeout(highlightTimer);
  transientHighlightIds.value = new Set();
  await nextTick();
  transientHighlightIds.value = new Set(alignmentIds);
  highlightTimer = setTimeout(() => { transientHighlightIds.value = new Set(); }, 850);
};

const focusAlignment = async (alignmentId: string) => {
  const focused = await alignedWorkspaceRef.value?.focusAlignment(alignmentId);
  if (!focused) return;
  await pulseHighlight([alignmentId]);
};
const { highlightedSegmentId: jumpHighlightSegmentId, jumpToSegment } = useTransientSegmentJump({
  focus: async (segmentId) => alignedWorkspaceRef.value?.focusSegment(segmentId),
  onMissing: () => emit("status", "未找到跳转锚定的句段"),
});
const focusSegment = jumpToSegment;

const unlinkedRuns = computed(() => {
  const runs: AlignmentBlockView[][] = [];
  for (const row of rows.value) {
    if (row.linked) continue;
    const previousRow = rows.value[row.index - 1];
    if (!previousRow || previousRow.linked) runs.push([]);
    runs[runs.length - 1].push(row);
  }
  return runs;
});
const navigateUnlinked = async (direction: -1 | 1, byRun: boolean) => {
  const candidates = byRun ? unlinkedRuns.value : rows.value.filter((row) => !row.linked).map((row) => [row]);
  if (!candidates.length) { emit("status", "当前视图没有未匹配 Segment"); return; }
  const currentRowIndex = rows.value.findIndex((row) => row.alignmentId === props.selectedAlignmentId);
  const currentCandidateIndex = candidates.findIndex((candidate) => candidate.some((row) => row.index === currentRowIndex));
  let targetIndex: number;
  if (currentCandidateIndex >= 0) targetIndex = (currentCandidateIndex + direction + candidates.length) % candidates.length;
  else if (direction > 0) targetIndex = candidates.findIndex((candidate) => candidate[0].index > currentRowIndex);
  else {
    targetIndex = -1;
    for (let index = candidates.length - 1; index >= 0; index -= 1) {
      if (candidates[index][candidates[index].length - 1].index < currentRowIndex) { targetIndex = index; break; }
    }
  }
  if (targetIndex < 0) targetIndex = direction > 0 ? 0 : candidates.length - 1;
  const target = candidates[targetIndex];
  const targetSegmentId = target[0].sourceSegments[0]?.id ?? target[0].targetSegments[0]?.id;
  if (!targetSegmentId) { emit("status", "未找到跳转锚定的句段"); return; }
  clearSelection();
  suppressedSelectionFocusId.value = target[0].alignmentId;
  emit("select", target[0].alignmentId);
  await jumpToSegment(targetSegmentId);
};

watch(() => props.selectedAlignmentId, (alignmentId) => {
  if (alignmentId === suppressedSelectionFocusId.value) {
    suppressedSelectionFocusId.value = "";
    return;
  }
  suppressedSelectionFocusId.value = "";
  if (alignmentId && !hasOperationSelection.value) void focusAlignment(alignmentId);
});

const activateFindMatch = (index: number) => {
  const matches = findMatches.value;
  if (!matches.length) { findCursor.value = -1; return; }
  findCursor.value = (index + matches.length) % matches.length;
  const match = matches[findCursor.value];
  const selectionChanged = props.selectedAlignmentId !== match.alignmentId;
  emit("select", match.alignmentId);
  if (!selectionChanged) void focusAlignment(match.alignmentId);
};
watch(findQuery, (query) => {
  if (findDebounceTimer !== null) clearTimeout(findDebounceTimer);
  const normalizedQuery = query.trim().toLocaleLowerCase();
  if (!normalizedQuery) { deferredFindQuery.value = ""; return; }
  findDebounceTimer = setTimeout(() => {
    findDebounceTimer = null;
    deferredFindQuery.value = normalizedQuery;
  }, 120);
});
watch(deferredFindQuery, () => activateFindMatch(0));

const openFind = async () => { findOpen.value = true; await nextTick(); findInputRef.value?.focus(); findInputRef.value?.select(); };
const closeFind = () => { findOpen.value = false; findInputRef.value?.blur(); alignedWorkspaceRef.value?.focusViewport(); };
const navigateFind = async (direction: -1 | 1) => {
  if (!findOpen.value) await openFind();
  if (findMatches.value.length) activateFindMatch(findCursor.value + direction);
};
function clearOperationSelection() {
  selectedSourceIds.value = new Set();
  selectedTargetIds.value = new Set();
  selectedAlignmentIds.value = new Set();
}
function clearSelection() {
  clearOperationSelection();
  orderSelection.value = null;
}
const revealSegments = async (sourceIds: string[], targetIds: string[]) => {
  clearSelection();
  selectedSourceIds.value = new Set(sourceIds);
  selectedTargetIds.value = new Set(targetIds);
  await nextTick();
  await alignedWorkspaceRef.value?.focusSegments(sourceIds, targetIds);
};
defineExpose({ openFind, closeFind, navigateFind, clearSelection, focusSegment, revealSegments });

const requestEdit = (segmentId: string, alignmentId: string) => {
  const segment = [...props.sourceSegments, ...props.targetSegments].find((candidate) => candidate.id === segmentId);
  if (!segment) return;
  if (!props.writable) { emit("status", "当前为演示预览，请先新建或打开工程"); return; }
  // A browser double-click dispatches two click events first. Those clicks are
  // valid for alignment operations in Review, but must not leak into Edit.
  clearSelection();
  emit("select", alignmentId);
  emit("requestEdit", segmentId, alignmentId);
};
const toggleBookmark = (segmentId: string, alignmentId: string | null) => {
  if (!props.writable) { emit("status", "当前为演示预览，请先新建或打开工程"); return; }
  emit("bookmark", segmentId, alignmentId);
};
const toggleSegment = (side: "source" | "target", id: string, alignmentId: string, event: MouseEvent) => {
  const selected = side === "source" ? selectedSourceIds : selectedTargetIds;
  const additive = event.metaKey || event.ctrlKey;
  const next = new Set(additive ? selected.value : selected.value.size === 1 && selected.value.has(id) ? selected.value : []);
  if (next.has(id)) next.delete(id); else next.add(id);
  selected.value = next;
  if (!additive) selectedAlignmentIds.value = new Set();
  emit("select", alignmentId);
};
const selectOrderSegment = (side: LanguageSide, segmentId: string, alignmentId: string) => {
  clearOperationSelection();
  orderSelection.value = { side, segmentId };
  emit("select", alignmentId);
};
const selectSegment = (side: LanguageSide, segmentId: string, alignmentId: string, event: MouseEvent) => {
  orderSelection.value = null;
  toggleSegment(side, segmentId, alignmentId, event);
};
const canInsertGap = (side: LanguageSide, segmentId: string, rowOrEdge: AlignmentBlockView | AlignmentGapEdge, possibleEdge?: AlignmentGapEdge) => {
  const edge = typeof rowOrEdge === "string" ? rowOrEdge : possibleEdge;
  const alignmentId = typeof rowOrEdge === "string"
    ? alignmentBySegmentId.value.get(segmentId)
    : rowOrEdge.linked ? rowOrEdge.alignmentId : undefined;
  if (!props.writable || !edge || !alignmentId) return false;
  const alignment = props.alignments.find((candidate) => candidate.id === alignmentId);
  if (!alignment) return false;
  const ownSegments = [...(side === "source" ? props.sourceSegments : props.targetSegments)].sort((a, b) => a.order - b.order);
  const oppositeSegments = [...(side === "source" ? props.targetSegments : props.sourceSegments)].sort((a, b) => a.order - b.order);
  const ownRank = ownSegments.findIndex((segment) => segment.id === segmentId);
  const oppositeIds = side === "source" ? alignment.targetIds : alignment.sourceIds;
  const oppositeRanks = oppositeIds
    .map((id) => oppositeSegments.findIndex((segment) => segment.id === id))
    .filter((rank) => rank >= 0);
  if (ownRank < 0 || !oppositeRanks.length) return false;
  const ownPairStart = ownRank + Number(edge === "after");
  const gapRank = edge === "before" ? Math.min(...oppositeRanks) : Math.max(...oppositeRanks) + 1;
  return ownPairStart < ownSegments.length && gapRank + 1 < oppositeSegments.length;
};
const selectRow = (row: AlignmentBlockView, event: MouseEvent) => {
  orderSelection.value = null;
  const additive = event.metaKey || event.ctrlKey;
  if (!row.linked) {
    if (!additive) clearSelection();
    const sourceIds = row.sourceSegments.map((segment) => segment.id);
    const targetIds = row.targetSegments.map((segment) => segment.id);
    const allSelected = sourceIds.every((id) => selectedSourceIds.value.has(id))
      && targetIds.every((id) => selectedTargetIds.value.has(id));
    const nextSources = new Set(selectedSourceIds.value);
    const nextTargets = new Set(selectedTargetIds.value);
    for (const id of sourceIds) allSelected && additive ? nextSources.delete(id) : nextSources.add(id);
    for (const id of targetIds) allSelected && additive ? nextTargets.delete(id) : nextTargets.add(id);
    selectedSourceIds.value = nextSources;
    selectedTargetIds.value = nextTargets;
    emit("select", row.alignmentId);
    return;
  }
  if (!additive) {
    selectedSourceIds.value = new Set();
    selectedTargetIds.value = new Set();
  }
  // selectedAlignmentId is also the navigation focus. When no explicit
  // operation selection exists it is rendered as the single selected row.
  // Seed the first Ctrl/Command click with that visible row; otherwise Windows
  // users see the focus disappear and still end up with only one selected item.
  const focusedAlignmentIsVisibleSelection = additive
    && !hasOperationSelection.value
    && props.alignments.some((alignment) => alignment.id === props.selectedAlignmentId);
  const next = new Set(additive
    ? [
        ...selectedAlignmentIds.value,
        ...(focusedAlignmentIsVisibleSelection ? [props.selectedAlignmentId] : []),
      ]
    : []);
  if (additive && next.has(row.alignmentId)) next.delete(row.alignmentId); else next.add(row.alignmentId);
  selectedAlignmentIds.value = next;
  emit("select", row.alignmentId);
};
const quickRelationAction = (row: AlignmentBlockView) => {
  if (import.meta.env.DEV) console.debug("[jueming:alignment-action]", {
    action: row.linked ? "unlink" : "link",
    alignmentId: row.linked ? row.alignmentId : null,
    sourceIds: row.sourceSegments.map((segment) => segment.id),
    targetIds: row.targetSegments.map((segment) => segment.id),
  });
  if (!props.writable) {
    emit("status", "当前为演示预览，请先新建或打开工程");
    return;
  }
  if (row.linked) {
    clearSelection();
    emit("unlink", row.alignmentId);
    return;
  }
  const sourceIds = row.sourceSegments.map((segment) => segment.id);
  const targetIds = row.targetSegments.map((segment) => segment.id);
  if (sourceIds.length && targetIds.length) {
    clearSelection();
    emit("link", sourceIds, targetIds);
    return;
  }
  clearSelection();
  selectedSourceIds.value = new Set(sourceIds);
  selectedTargetIds.value = new Set(targetIds);
  emit("status", "已选中待匹配 Segment；请在另一侧选择句段后使用 Link");
};
watch(() => props.mode, (mode, previousMode) => {
  if (mode === "edit" || previousMode === "edit") clearSelection();
});
const selectedGroupable = computed(() => [...selectedAlignmentIds.value].filter((id) => props.alignments.some((alignment) => alignment.id === id)));
const alignedSegmentIds = computed(() => new Set(props.alignments.flatMap((alignment) => [...alignment.sourceIds, ...alignment.targetIds])));
const selectedUnlinkedSourceIds = computed(() => [...selectedSourceIds.value].filter((id) => !alignedSegmentIds.value.has(id)));
const selectedUnlinkedTargetIds = computed(() => [...selectedTargetIds.value].filter((id) => !alignedSegmentIds.value.has(id)));
const hasAlignedSegmentSelection = computed(() => selectedSourceIds.value.size + selectedTargetIds.value.size
  > selectedUnlinkedSourceIds.value.length + selectedUnlinkedTargetIds.value.length);
const selectedUnlinkableId = computed(() => !hasSegmentSelection.value && selectedAlignmentIds.value.size === 1 && selectedGroupable.value.length === 1 ? selectedGroupable.value[0] : null);
const selectedGroupRowIndexes = computed(() => rows.value
  .filter((row) => row.linked
    ? selectedAlignmentIds.value.has(row.alignmentId)
    : [...row.sourceSegments, ...row.targetSegments].some((segment) => selectedSourceIds.value.has(segment.id) || selectedTargetIds.value.has(segment.id)))
  .map((row) => row.index));
const groupSourceIds = computed(() => new Set([
  ...selectedGroupable.value.flatMap((id) => props.alignments.find((alignment) => alignment.id === id)?.sourceIds ?? []),
  ...selectedUnlinkedSourceIds.value,
]));
const groupTargetIds = computed(() => new Set([
  ...selectedGroupable.value.flatMap((id) => props.alignments.find((alignment) => alignment.id === id)?.targetIds ?? []),
  ...selectedUnlinkedTargetIds.value,
]));
const groupSelectionSize = computed(() => selectedGroupable.value.length + selectedUnlinkedSourceIds.value.length + selectedUnlinkedTargetIds.value.length);
const groupPlan = computed(() => {
  if (hasAlignedSegmentSelection.value
    || !selectedGroupable.value.length
    || groupSelectionSize.value < 2
    || !groupSourceIds.value.size
    || !groupTargetIds.value.size) return null;
  return {
    alignmentIds: selectedGroupable.value,
    unlinkedSegmentIds: [...selectedUnlinkedSourceIds.value, ...selectedUnlinkedTargetIds.value],
  };
});
const groupHint = computed(() => {
  if (hasAlignedSegmentSelection.value) return "已对齐句段请通过中间关系图标选择整个 Alignment";
  if (!selectedGroupable.value.length && groupSelectionSize.value) return "仅选择未对齐句段时请使用 Link";
  if (groupSelectionSize.value < 2) return "按 Ctrl/⌘ 多选至少两个完整 Alignment，或一个 Alignment 加未对齐句段";
  if (!groupSourceIds.value.size || !groupTargetIds.value.size) return "分组结果必须同时包含左右语言句段";
  return `分组 ${selectedGroupable.value.length} 个 Alignment 与 ${selectedUnlinkedSourceIds.value.length + selectedUnlinkedTargetIds.value.length} 条未对齐句段`;
});
const groupWarning = computed(() => {
  if (!groupPlan.value) return null;
  const indexes = selectedGroupRowIndexes.value;
  if (indexes.length > 1 && indexes[indexes.length - 1] - indexes[0] + 1 !== indexes.length) return "所选块在当前视图投影中不连续；仍会按稳定 Segment ID 分组。";
  const positions = selectedGroupable.value.map((id) => {
    const alignment = props.alignments.find((candidate) => candidate.id === id);
    const source = Math.min(...(alignment?.sourceIds.map((segmentId) => props.sourceSegments.find((segment) => segment.id === segmentId)?.order ?? Number.MAX_SAFE_INTEGER) ?? []));
    const target = Math.min(...(alignment?.targetIds.map((segmentId) => props.targetSegments.find((segment) => segment.id === segmentId)?.order ?? Number.MAX_SAFE_INTEGER) ?? []));
    return { source, target };
  }).sort((a, b) => a.source - b.source);
  return positions.some((position, index) => index > 0 && position.target < positions[index - 1].target)
    ? "所选 Alignment 的两侧顺序可能交叉；请确认 Group 后的阅读顺序。"
    : null;
});
const ungroupableAlignment = computed(() => {
  if (hasSegmentSelection.value || selectedAlignmentIds.value.size !== 1) return null;
  const alignmentId = [...selectedAlignmentIds.value][0];
  const alignment = props.alignments.find((candidate) => candidate.id === alignmentId);
  if (!alignment || alignment.sourceIds.length < 2 || alignment.targetIds.length < 2) return null;
  return alignment;
});

const selectedSegments = computed(() => ([
  ...props.sourceSegments.filter((segment) => selectedSourceIds.value.has(segment.id)),
  ...props.targetSegments.filter((segment) => selectedTargetIds.value.has(segment.id)),
]));
const segmentMergeIssue = computed(() => {
  const segments = selectedSegments.value;
  if (segments.length < 2) return "选择同一侧至少两个连续 Segment";
  if (new Set(segments.map((segment) => segment.side)).size !== 1) return "Merge 内容只能合并同一语言侧的 Segment";
  const ordered = [...segments].sort((a, b) => a.order - b.order);
  if (ordered.some((segment, index) => index > 0 && segment.order !== ordered[index - 1].order + 1)) return "Merge 内容要求 Segment 在当前顺序中连续";
  const alignmentIds = new Set(segments.map((segment) => alignmentBySegmentId.value.get(segment.id) ?? null));
  if (alignmentIds.size > 1) return "选中的 Segment 跨越多个 Alignment Block；请先 Group，再合并内容";
  return null;
});
const segmentMergePlan = computed(() => segmentMergeIssue.value ? null : [...selectedSegments.value].sort((a, b) => a.order - b.order));
const segmentSplitPlan = computed(() => selectedSegments.value.length === 1 ? selectedSegments.value[0] : null);
const segmentSplitIssue = computed(() => {
  if (!selectedSegments.value.length) return "选择一个 Segment 后拆分内容";
  if (selectedSegments.value.length > 1) return "Split 内容一次只能拆分一个 Segment";
  return null;
});
const openMergeContent = () => {
  const segments = segmentMergePlan.value;
  if (!segments) return;
  contentDialog.value = { kind: "merge", segmentIds: segments.map((segment) => segment.id), initialValue: segments.map((segment) => segment.text).join("\n") };
};
const openSplitContent = () => {
  const segment = segmentSplitPlan.value;
  if (!segment) return;
  contentDialog.value = { kind: "split", segmentId: segment.id, original: segment.text };
};
const closeContentDialog = () => { contentDialog.value = null; };
const confirmMergeContent = (segmentIds: string[], mergedContent: string) => {
  emit("mergeSegments", segmentIds, mergedContent);
  contentDialog.value = null;
  clearSelection();
};
const confirmSplitContent = (segmentId: string, parts: string[]) => {
  emit("splitSegment", segmentId, parts);
  contentDialog.value = null;
  clearSelection();
};
const openUngroupDialog = () => { ungroupDialogAlignment.value = ungroupableAlignment.value; };
const confirmUngroup = (alignmentId: string, sourceGroups: string[][], targetGroups: string[][]) => {
  emit("ungroup", alignmentId, sourceGroups, targetGroups);
  ungroupDialogAlignment.value = null;
  clearSelection();
};

const {
  draggedId,
  dropTargetId,
  dropEdge,
  dragPointer,
  draggedSegment,
  registerOrderSegment,
  registerOrderDropTarget,
} = useOrderDragAndDrop({
  getSegments: (side) => side === "source" ? props.sourceSegments : props.targetSegments,
  onReorder: (side, orderedSegmentIds, intent) => emit("reorder", side, orderedSegmentIds, intent),
  afterReorderApplied: () => alignedWorkspaceRef.value?.measure(),
});
onBeforeUnmount(() => {
  if (highlightTimer !== null) clearTimeout(highlightTimer);
  if (findDebounceTimer !== null) clearTimeout(findDebounceTimer);
});
</script>

<template>
  <section class="workspace" :class="[`workspace--${mode}`, { 'workspace--trackpad': trackpadOptimized }]" aria-label="双语平行工作区">
    <div v-if="mode === 'review' || mode === 'order'" class="unified-toolbar" aria-label="审阅与排序工具">
      <section class="unified-toolbar__group" aria-label="顺序操作">
        <button class="tool-button tool-button--active" type="button" title="从中文或英文编号手柄拖动 Segment"><GripVertical :size="15" /><span class="tool-button__label">拖动排序</span></button>
        <button class="tool-button" type="button" :disabled="!writable || !orderSelection" title="上移所选 Segment" @click="orderSelection && emit('move', orderSelection.side, orderSelection.segmentId, 'up')"><ArrowUp :size="15" /><span class="tool-button__label">上移</span></button>
        <button class="tool-button" type="button" :disabled="!writable || !orderSelection" title="下移所选 Segment" @click="orderSelection && emit('move', orderSelection.side, orderSelection.segmentId, 'down')"><ArrowDown :size="15" /><span class="tool-button__label">下移</span></button>
        <button class="tool-button" type="button" :disabled="!writable" title="恢复 Segment 顺序" @click="emit('resetOrder')"><RotateCcw :size="15" /><span class="tool-button__label">恢复顺序</span></button>
      </section>
      <section class="unified-toolbar__group unified-toolbar__group--relations" aria-label="Alignment 关系操作">
        <button class="tool-button tool-button--link" type="button" :disabled="!writable || selectedAlignmentIds.size > 0 || hasAlignedSegmentSelection || !selectedSourceIds.size || !selectedTargetIds.size" :title="hasAlignedSegmentSelection ? '已对齐句段须先 Unlink；Link 不会静默抢占关系' : '分别选择至少一条未对齐中文和英文 Segment'" @click="emit('link', [...selectedSourceIds], [...selectedTargetIds])"><Link2 :size="15" /><span class="tool-button__label">Link</span></button>
        <button class="tool-button" type="button" :disabled="!writable || !selectedUnlinkableId" title="在中间关系轨选择一个 Alignment" @click="selectedUnlinkableId && emit('unlink', selectedUnlinkableId)"><Link2Off :size="15" /><span class="tool-button__label">Unlink</span></button>
        <button class="tool-button" type="button" :disabled="!writable || !groupPlan" :title="groupHint" @click="groupPlan && emit('group', groupPlan.alignmentIds, groupPlan.unlinkedSegmentIds)"><Merge :size="15" /><span class="tool-button__label">Group</span></button>
        <button class="tool-button" type="button" :disabled="!writable || !ungroupableAlignment" title="编辑两侧的明确分组边界" @click="openUngroupDialog"><Scissors :size="15" /><span class="tool-button__label">Ungroup</span></button>
      </section>
      <section class="unified-toolbar__group unified-toolbar__group--unlinked-nav" aria-label="上一未匹配">
        <span class="unlinked-nav__label">上一未匹配</span>
        <button class="tool-button tool-button--icon" type="button" :disabled="!unlinkedRuns.length" title="上一未匹配 Segment" aria-label="上一未匹配 Segment" @click="navigateUnlinked(-1, false)"><ArrowUp :size="16" /></button>
        <button class="tool-button tool-button--icon" type="button" :disabled="!unlinkedRuns.length" title="上一连续未匹配段" aria-label="上一连续未匹配段" @click="navigateUnlinked(-1, true)"><ChevronsUp :size="16" /></button>
      </section>
      <section class="unified-toolbar__group unified-toolbar__group--unlinked-nav" aria-label="下一未匹配">
        <span class="unlinked-nav__label">下一未匹配</span>
        <button class="tool-button tool-button--icon" type="button" :disabled="!unlinkedRuns.length" title="下一未匹配 Segment" aria-label="下一未匹配 Segment" @click="navigateUnlinked(1, false)"><ArrowDown :size="16" /></button>
        <button class="tool-button tool-button--icon" type="button" :disabled="!unlinkedRuns.length" title="下一连续未匹配段" aria-label="下一连续未匹配段" @click="navigateUnlinked(1, true)"><ChevronsDown :size="16" /></button>
      </section>
      <section v-if="selectedSegments.length" class="unified-toolbar__group unified-toolbar__group--content" aria-label="Segment 内容操作">
        <button class="tool-button" type="button" :disabled="!writable || !segmentMergePlan" :title="segmentMergeIssue ?? '合并内容会保留首个 Segment ID'" @click="openMergeContent"><Merge :size="15" /><span class="tool-button__label">Merge 内容</span></button>
        <button class="tool-button" type="button" :disabled="!writable || !segmentSplitPlan" :title="segmentSplitIssue ?? '以无损 parts 拆分当前内容'" @click="openSplitContent"><Scissors :size="15" /><span class="tool-button__label">Split 内容</span></button>
      </section>
      <span v-if="groupWarning || (segmentMergeIssue && selectedSegments.length)" class="unified-toolbar__warning">{{ groupWarning ?? segmentMergeIssue }}</span>
      <button v-if="hasOperationSelection || orderSelection" class="selection-clear" type="button" @click="clearSelection">清除选择</button>
      <span v-else class="unified-toolbar__summary">{{ writable ? `中文 ${sourceSegments.length} 段 · 英文 ${targetSegments.length} 段` : '演示预览 · 打开工程后可操作' }}</span>
    </div>
    <div v-if="findOpen && mode === 'review'" class="view-find" role="search" aria-label="审阅模式快速查找">
      <Search :size="16" /><input ref="findInputRef" v-model="findQuery" aria-label="查找当前平行视图" placeholder="查找中文或英文…" @keydown.enter.prevent="activateFindMatch(findCursor + ($event.shiftKey ? -1 : 1))" @keydown.esc.prevent="closeFind" />
      <span>{{ findPending ? '查找中…' : findMatches.length ? `${findCursor + 1} / ${findMatches.length}` : findQuery ? '无结果' : '输入关键词' }}</span>
      <button type="button" title="上一处" :disabled="!findMatches.length" @click="activateFindMatch(findCursor - 1)"><ArrowUp :size="15" /></button><button type="button" title="下一处" :disabled="!findMatches.length" @click="activateFindMatch(findCursor + 1)"><ArrowDown :size="15" /></button><button type="button" title="关闭查找" @click="closeFind"><X :size="16" /></button>
    </div>
    <div class="column-headings" :style="{ paddingRight: `${listScrollbarWidth}px` }"><h2>中文 <span>（原文）</span></h2><div class="heading-divider" aria-hidden="true"></div><h2>English <span>（译文）</span></h2></div>
    <AlignedWorkspaceViewport
      ref="alignedWorkspaceRef"
      :rows="rows"
      :mode="mode"
      :writable="writable"
      :smooth-navigation="smoothNavigation"
      :reorder-enabled="mode === 'review' || mode === 'order'"
      :selected-alignment-id="selectedAlignmentId"
      :selected-alignment-ids="selectedAlignmentIds"
      :has-operation-selection="hasOperationSelection"
      :selected-source-ids="selectedSourceIds"
      :selected-target-ids="selectedTargetIds"
      :order-selection="orderSelection"
      :edit-session="editSession"
      :bookmarked-segment-ids="bookmarked"
      :annotated-segment-ids="annotated"
      :highlighted-alignment-ids="transientHighlightIds"
      :jump-highlight-segment-id="jumpHighlightSegmentId"
      :dragged-id="draggedId"
      :drop-target-id="dropTargetId"
      :drop-edge="dropEdge"
      :register-order-segment="registerOrderSegment"
      :register-order-drop-target="registerOrderDropTarget"
      :can-insert-gap="canInsertGap"
      @select-alignment="selectRow"
      @quick-relation-action="quickRelationAction"
      @select-segment="selectSegment"
      @select-order-segment="selectOrderSegment"
      @request-edit="requestEdit"
      @insert-gap="(segmentId, edge) => emit('insertGap', segmentId, edge)"
      @bookmark="toggleBookmark"
      @annotation="(segmentId, alignmentId) => emit('annotation', segmentId, alignmentId)"
      @edit-draft="emit('editDraft', $event)"
      @commit-edit="emit('commitEdit', $event)"
      @cancel-edit="emit('cancelEdit')"
      @escape-edit="emit('escapeEdit')"
      @scrollbar-width="listScrollbarWidth = $event"
    />
    <div v-if="mode === 'edit' && !editSession" class="edit-tip"><LockKeyhole :size="15" />双击任意句子进入编辑；Esc 保存并退出，Ctrl/⌘+Enter 保存并退出。</div>
    <OrderDragOverlay :dragged-segment="draggedSegment" :drag-pointer="dragPointer" />
    <SegmentContentDialog v-if="contentDialog" :operation="contentDialog" @close="closeContentDialog" @merge="confirmMergeContent" @split="confirmSplitContent" />
    <AlignmentUngroupDialog v-if="ungroupDialogAlignment" :alignment="ungroupDialogAlignment" :source-segments="sourceSegments" :target-segments="targetSegments" @close="ungroupDialogAlignment = null" @confirm="confirmUngroup" />
  </section>
</template>

<style scoped>
.workspace { --alignment-gutter: 176px; }
.unified-toolbar { display: flex; min-width: 0; min-height: 55px; align-items: center; gap: 0; overflow: hidden; padding: 0 28px; border-bottom: 1px solid var(--line); background: var(--surface-subtle); }
.unified-toolbar__group { display: flex; flex: 0 0 auto; align-items: center; gap: 9px; padding-right: 16px; }
.unified-toolbar__group + .unified-toolbar__group { padding-left: 16px; border-left: 1px solid var(--line); }
.unified-toolbar .tool-button { min-width: 34px; height: 34px; justify-content: center; white-space: nowrap; transition: gap 180ms ease, padding 180ms ease; }
.unified-toolbar .tool-button svg { flex: 0 0 auto; }
.tool-button__label { display: inline-block; max-width: 9rem; overflow: hidden; opacity: 1; transform: translateX(0); transition: max-width 180ms ease, opacity 120ms ease, transform 180ms ease; }
.tool-button--link:not(:disabled) { border-color: #a8d0ad; color: var(--green-900); background: var(--surface-green-soft); }
.unified-toolbar__group--unlinked-nav { gap: 5px; padding-right: 10px; }
.unified-toolbar__group + .unified-toolbar__group--unlinked-nav { padding-left: 10px; }
.unlinked-nav__label { margin-right: 2px; color: var(--ink-600); font-size: var(--jm-font-size-subheadline); white-space: nowrap; line-height: var(--jm-line-height-subheadline); }
.unified-toolbar .tool-button--icon { width: 34px; padding-inline: 0; justify-content: center; }
.unified-toolbar__warning { min-width: 0; flex: 0 1 230px; overflow: hidden; margin-left: 12px; color: #916714; font-size: var(--jm-font-size-body); line-height: 1.3; }
.unified-toolbar__summary { min-width: 0; overflow: hidden; margin-left: auto; color: var(--ink-500); font-size: var(--jm-font-size-subheadline); text-overflow: ellipsis; white-space: nowrap; line-height: var(--jm-line-height-subheadline); }
.selection-clear { margin-left: auto; padding: 5px 9px; border: 0; color: var(--ink-500); background: transparent; font-size: var(--jm-font-size-callout); white-space: nowrap; cursor: pointer; line-height: var(--jm-line-height-callout); }
.view-find { display: flex; align-items: center; gap: 7px; min-height: 45px; padding: 0 18px; border-bottom: 1px solid #b8d5bb; color: var(--green-900); background: #f4faf2; }.view-find input { flex: 1; min-width: 140px; height: 31px; padding: 0 10px; border: 1px solid #b8cdb9; border-radius: 5px; outline: none; background: #fff; }.view-find input:focus { border-color: var(--green-700); box-shadow: 0 0 0 2px rgb(55 127 66 / 14%); }.view-find span { min-width: 70px; color: var(--ink-500); font-size: var(--jm-font-size-subheadline); text-align: right; line-height: var(--jm-line-height-subheadline); }.view-find button { display: grid; width: 30px; height: 30px; place-items: center; border: 1px solid transparent; border-radius: 5px; color: var(--ink-700); background: transparent; cursor: pointer; }.view-find button:hover:not(:disabled) { border-color: #bad2bd; background: #fff; }
.column-headings { height: 44px; flex: 0 0 44px; grid-template-columns: minmax(0, var(--source-column-fr)) var(--alignment-gutter) minmax(0, var(--target-column-fr)); }
@media (max-width: 1360px) {
  .workspace { --alignment-gutter: 138px; }
  .unified-toolbar { padding-inline: 16px; }
  .unified-toolbar .tool-button:not(.tool-button--icon) { gap: 0; padding-inline: 0; }
  .tool-button__label { max-width: 0; opacity: 0; transform: translateX(-4px); }
}
</style>
