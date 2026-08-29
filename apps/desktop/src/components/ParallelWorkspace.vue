<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import { ArrowDown, ArrowUp, GripVertical, Link2, Link2Off, LockKeyhole, Merge, RotateCcw, Scissors, Search, X } from "@lucide/vue";
import type { EditSession } from "../composables/useViewModeController";
import type { AlignmentDto, AlignmentGapEdge, LanguageSide, SegmentDto, WorkspaceMode } from "../domain/kernel-client";
import { buildAlignmentBlocks, type AlignmentBlockView, type OrderSelection } from "../domain/workspace-projection";
import SegmentContentDialog, { type ContentOperationRequest } from "./SegmentContentDialog.vue";
import AlignmentUngroupDialog from "./AlignmentUngroupDialog.vue";
import AlignmentBlock from "./AlignmentBlock.vue";
import OrderDragOverlay from "./OrderDragOverlay.vue";
import { useOrderDragAndDrop } from "../composables/useOrderDragAndDrop";

const props = withDefaults(defineProps<{
  mode: WorkspaceMode;
  sourceSegments: SegmentDto[];
  targetSegments: SegmentDto[];
  alignments: AlignmentDto[];
  bookmarkedAlignmentIds?: string[];
  selectedAlignmentId: string;
  editSession?: EditSession | null;
  trackpadOptimized?: boolean;
  writable?: boolean;
}>(), { bookmarkedAlignmentIds: () => [], selectedAlignmentId: "", editSession: null, trackpadOptimized: false, writable: false });

const emit = defineEmits<{
  select: [alignmentId: string];
  requestEdit: [segmentId: string, alignmentId: string];
  editDraft: [draft: string];
  commitEdit: [exitAfterSave: boolean];
  cancelEdit: [];
  escapeEdit: [];
  move: [side: LanguageSide, segmentId: string, direction: "up" | "down"];
  reorder: [side: LanguageSide, orderedSegmentIds: string[]];
  insertGap: [segmentId: string, edge: AlignmentGapEdge];
  resetOrder: [];
  link: [sourceIds: string[], targetIds: string[]];
  unlink: [alignmentId: string];
  mergeSegments: [segmentIds: string[], mergedContent: string];
  splitSegment: [segmentId: string, parts: string[]];
  group: [alignmentIds: string[], unlinkedSegmentIds: string[]];
  ungroup: [alignmentId: string, sourceGroups: string[][], targetGroups: string[][]];
  bookmark: [segmentId: string, alignmentId: string];
  status: [message: string];
}>();

const bookmarked = computed(() => new Set(props.bookmarkedAlignmentIds));
const selectedSourceIds = ref(new Set<string>());
const selectedTargetIds = ref(new Set<string>());
const selectedAlignmentIds = ref(new Set<string>());
const orderSelection = ref<OrderSelection | null>(null);
const hasSegmentSelection = computed(() => selectedSourceIds.value.size > 0 || selectedTargetIds.value.size > 0);
const hasOperationSelection = computed(() => hasSegmentSelection.value || selectedAlignmentIds.value.size > 0);
const listRef = ref<HTMLElement | null>(null);
const listScrollbarWidth = ref(0);
const findInputRef = ref<HTMLInputElement | null>(null);
const findOpen = ref(false);
const findQuery = ref("");
const deferredFindQuery = ref("");
const findCursor = ref(-1);
const transientHighlightId = ref("");
const contentDialog = ref<ContentOperationRequest | null>(null);
const ungroupDialogAlignment = ref<AlignmentDto | null>(null);
let highlightTimer: ReturnType<typeof setTimeout> | null = null;
let scrollAnimationFrame: number | null = null;
let resolveScrollAnimation: (() => void) | null = null;
let findDebounceTimer: ReturnType<typeof setTimeout> | null = null;
let listResizeObserver: ResizeObserver | null = null;

const rows = computed(() => buildAlignmentBlocks(props.sourceSegments, props.targetSegments, props.alignments));

const rowBySegmentId = computed(() => {
  const result = new Map<string, AlignmentBlockView>();
  for (const row of rows.value) [...row.sourceSegments, ...row.targetSegments].forEach((segment) => result.set(segment.id, row));
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

const virtualizer = useVirtualizer({ getScrollElement: () => listRef.value, count: rows.value.length, estimateSize: () => 112, overscan: 10 });
const virtualRows = computed(() => virtualizer.value.getVirtualItems());
const totalSize = computed(() => virtualizer.value.getTotalSize());
watch(() => rows.value.length, (count) => virtualizer.value.setOptions({ ...virtualizer.value.options, count }));
const measureRow = (element: unknown) => { if (element instanceof Element) virtualizer.value.measureElement(element); };

const pulseHighlight = async (alignmentId: string) => {
  if (highlightTimer !== null) clearTimeout(highlightTimer);
  transientHighlightId.value = "";
  await nextTick();
  transientHighlightId.value = alignmentId;
  highlightTimer = setTimeout(() => { transientHighlightId.value = ""; }, 850);
};

const cancelScrollAnimation = () => {
  if (scrollAnimationFrame !== null) cancelAnimationFrame(scrollAnimationFrame);
  scrollAnimationFrame = null;
  resolveScrollAnimation?.();
  resolveScrollAnimation = null;
};

const animateScrollToIndex = (rowIndex: number, reducedMotion: boolean) => new Promise<void>((resolve) => {
  cancelScrollAnimation();
  const scroller = listRef.value;
  if (!scroller || reducedMotion) {
    virtualizer.value.scrollToIndex(rowIndex, { align: "center" });
    resolve();
    return;
  }
  const firstTarget = virtualizer.value.getOffsetForIndex(rowIndex, "center")?.[0];
  if (firstTarget === undefined) { resolve(); return; }
  const startOffset = scroller.scrollTop;
  const distance = Math.abs(firstTarget - startOffset);
  if (distance < 2) { resolve(); return; }
  const maximumDuration = props.trackpadOptimized ? 420 : 650;
  const duration = Math.min(maximumDuration, Math.max(220, Math.sqrt(distance) * 3));
  const startedAt = performance.now();
  resolveScrollAnimation = resolve;
  const step = (now: number) => {
    const progress = Math.min(1, (now - startedAt) / duration);
    const eased = 1 - (1 - progress) ** 3;
    const currentTarget = virtualizer.value.getOffsetForIndex(rowIndex, "center")?.[0] ?? firstTarget;
    scroller.scrollTo({ top: startOffset + (currentTarget - startOffset) * eased, behavior: "auto" });
    if (progress < 1) { scrollAnimationFrame = requestAnimationFrame(step); return; }
    scrollAnimationFrame = null;
    resolveScrollAnimation = null;
    virtualizer.value.scrollToIndex(rowIndex, { align: "center" });
    resolve();
  };
  scrollAnimationFrame = requestAnimationFrame(step);
});

const focusAlignment = async (alignmentId: string) => {
  const rowIndex = rows.value.findIndex((row) => row.alignmentId === alignmentId);
  if (rowIndex < 0) return;
  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  await animateScrollToIndex(rowIndex, reducedMotion);
  await pulseHighlight(alignmentId);
};

watch(() => props.selectedAlignmentId, (alignmentId) => { if (alignmentId && !hasOperationSelection.value) void focusAlignment(alignmentId); });

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
const closeFind = () => { findOpen.value = false; findInputRef.value?.blur(); listRef.value?.focus(); };
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
const focusSegment = (segmentId: string) => {
  const row = rowBySegmentId.value.get(segmentId);
  if (row) void focusAlignment(row.alignmentId);
};
defineExpose({ openFind, closeFind, navigateFind, clearSelection, focusSegment });

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
const toggleBookmark = (row: AlignmentBlockView) => {
  if (!props.writable) { emit("status", "当前为演示预览，请先新建或打开工程"); return; }
  const segmentId = row.sourceSegments[0]?.id ?? row.targetSegments[0]?.id;
  if (segmentId) emit("bookmark", segmentId, row.alignmentId);
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
  if (props.mode === "order") {
    selectOrderSegment(side, segmentId, alignmentId);
    return;
  }
  toggleSegment(side, segmentId, alignmentId, event);
};
const canInsertGap = (side: LanguageSide, segmentId: string, rowOrEdge: AlignmentBlockView | AlignmentGapEdge, possibleEdge?: AlignmentGapEdge) => {
  const row = typeof rowOrEdge === "string" ? rowBySegmentId.value.get(segmentId) : rowOrEdge;
  const edge = typeof rowOrEdge === "string" ? rowOrEdge : possibleEdge;
  if (!row || !edge) return false;
  if (!props.writable || !row.linked) return false;
  const alignment = props.alignments.find((candidate) => candidate.id === row.alignmentId);
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
  if (props.mode === "order") orderSelection.value = null;
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
watch(() => props.mode, (mode, previousMode) => {
  if (mode === "order") clearOperationSelection();
  if (previousMode === "order" && mode !== "order") orderSelection.value = null;
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
const alignmentBySegmentId = computed(() => {
  const result = new Map<string, string>();
  for (const alignment of props.alignments) {
    for (const segmentId of [...alignment.sourceIds, ...alignment.targetIds]) result.set(segmentId, alignment.id);
  }
  return result;
});
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

const { draggedId, dropTargetId, dropEdge, dragPointer, draggedSegment, registerOrderSegment } = useOrderDragAndDrop({
  getSegments: (side) => side === "source" ? props.sourceSegments : props.targetSegments,
  onReorder: (side, orderedSegmentIds) => emit("reorder", side, orderedSegmentIds),
  afterReorderApplied: () => { virtualizer.value.measure(); },
});
const measureListScrollbar = () => {
  const list = listRef.value;
  listScrollbarWidth.value = list ? Math.max(0, list.offsetWidth - list.clientWidth) : 0;
};
onMounted(async () => {
  await nextTick();
  measureListScrollbar();
  if (listRef.value) {
    listResizeObserver = new ResizeObserver(measureListScrollbar);
    listResizeObserver.observe(listRef.value);
  }
});
onBeforeUnmount(() => {
  listResizeObserver?.disconnect();
  if (highlightTimer !== null) clearTimeout(highlightTimer);
  if (findDebounceTimer !== null) clearTimeout(findDebounceTimer);
  cancelScrollAnimation();
});
</script>

<template>
  <section class="workspace" :class="[`workspace--${mode}`, { 'workspace--trackpad': trackpadOptimized }]" aria-label="双语平行工作区">
    <div v-if="mode === 'order'" class="order-toolbar">
      <div class="order-label"><GripVertical :size="16" /><span>排序工具：</span></div>
      <button class="tool-button tool-button--active" type="button"><GripVertical :size="15" />拖动卡片</button>
      <button class="tool-button" type="button" :disabled="!writable || !orderSelection" @click="orderSelection && emit('move', orderSelection.side, orderSelection.segmentId, 'up')"><ArrowUp :size="15" />上移</button>
      <button class="tool-button" type="button" :disabled="!writable || !orderSelection" @click="orderSelection && emit('move', orderSelection.side, orderSelection.segmentId, 'down')"><ArrowDown :size="15" />下移</button>
      <button class="tool-button" type="button" :disabled="!writable" @click="emit('resetOrder')"><RotateCcw :size="15" />恢复顺序</button>
      <span class="order-hint">{{ writable ? `共 ${rows.length} 个 Alignment Block · 从中文编号手柄开始拖拽；选中两侧句段均可插入空位` : '演示预览 · 打开工程后可排序' }}</span>
    </div>
    <div v-if="findOpen && mode === 'review'" class="view-find" role="search" aria-label="审阅模式快速查找">
      <Search :size="16" /><input ref="findInputRef" v-model="findQuery" aria-label="查找当前平行视图" placeholder="查找中文或英文…" @keydown.enter.prevent="activateFindMatch(findCursor + ($event.shiftKey ? -1 : 1))" @keydown.esc.prevent="closeFind" />
      <span>{{ findPending ? '查找中…' : findMatches.length ? `${findCursor + 1} / ${findMatches.length}` : findQuery ? '无结果' : '输入关键词' }}</span>
      <button type="button" title="上一处" :disabled="!findMatches.length" @click="activateFindMatch(findCursor - 1)"><ArrowUp :size="15" /></button><button type="button" title="下一处" :disabled="!findMatches.length" @click="activateFindMatch(findCursor + 1)"><ArrowDown :size="15" /></button><button type="button" title="关闭查找" @click="closeFind"><X :size="16" /></button>
    </div>
    <div class="operation-toolbar" aria-label="Segment 与 Alignment 操作工具">
      <section class="operation-group operation-group--segments" aria-label="Segment 内容操作">
        <div class="operation-group__heading"><span>SEGMENT</span><b>内容操作</b><small>{{ selectedSegments.length }} 项</small></div>
        <button class="tool-button" type="button" :disabled="!writable || !segmentMergePlan" :title="segmentMergeIssue ?? '合并内容会保留首个 Segment ID'" @click="openMergeContent"><Merge :size="15" />Merge 内容</button>
        <button class="tool-button" type="button" :disabled="!writable || !segmentSplitPlan" :title="segmentSplitIssue ?? '以无损 parts 拆分当前内容'" @click="openSplitContent"><Scissors :size="15" />Split 内容</button>
        <span v-if="segmentMergeIssue && selectedSegments.length" class="operation-note operation-note--warning">{{ segmentMergeIssue }}</span>
      </section>
      <section class="operation-group operation-group--alignments" aria-label="Alignment 关系操作">
        <div class="operation-group__heading"><span>ALIGNMENT</span><b>关系操作</b><small>{{ selectedGroupable.length }} 项</small></div>
        <button class="tool-button tool-button--link" type="button" :disabled="!writable || selectedAlignmentIds.size > 0 || hasAlignedSegmentSelection || !selectedSourceIds.size || !selectedTargetIds.size" :title="hasAlignedSegmentSelection ? '已对齐句段须先 Unlink；Link 不会静默抢占关系' : '分别选择至少一条未对齐中文和英文 Segment'" @click="emit('link', [...selectedSourceIds], [...selectedTargetIds])"><Link2 :size="15" />Link</button>
        <button class="tool-button" type="button" :disabled="!writable || !selectedUnlinkableId" title="在中间关系轨选择一个 Alignment" @click="selectedUnlinkableId && emit('unlink', selectedUnlinkableId)"><Link2Off :size="15" />Unlink</button>
        <button class="tool-button" type="button" :disabled="!writable || !groupPlan" :title="groupHint" @click="groupPlan && emit('group', groupPlan.alignmentIds, groupPlan.unlinkedSegmentIds)"><Merge :size="15" />Group</button>
        <button class="tool-button" type="button" :disabled="!writable || !ungroupableAlignment" title="编辑两侧的明确分组边界" @click="openUngroupDialog"><Scissors :size="15" />Ungroup</button>
        <span v-if="groupWarning" class="operation-note operation-note--warning">{{ groupWarning }}</span>
      </section>
      <button v-if="hasOperationSelection" class="selection-clear" type="button" @click="clearSelection">清除选择</button>
    </div>
    <div class="column-headings" :style="{ paddingRight: `${listScrollbarWidth}px` }"><h2>中文 <span>（原文）</span></h2><div class="heading-divider" aria-hidden="true"></div><h2>English <span>（译文）</span></h2></div>
    <div ref="listRef" class="parallel-list" :class="{ 'parallel-list--order': mode === 'order' }" tabindex="-1" @wheel.passive="cancelScrollAnimation" @touchstart.passive="cancelScrollAnimation"><div class="parallel-list-inner" :style="{ height: `${totalSize}px` }">
      <div
        v-for="virtualRow in virtualRows"
        :key="rows[virtualRow.index].alignmentId"
        :ref="measureRow"
        :data-index="virtualRow.index"
        class="virtual-alignment-block"
        :style="{ transform: `translateY(${virtualRow.start}px)` }"
      >
        <AlignmentBlock
          :block="rows[virtualRow.index]"
          :mode="mode"
          :writable="writable"
          :alignment-active="mode !== 'order' && (selectedAlignmentIds.has(rows[virtualRow.index].alignmentId) || (!hasOperationSelection && rows[virtualRow.index].alignmentId === selectedAlignmentId))"
          :alignment-operation-selected="selectedAlignmentIds.has(rows[virtualRow.index].alignmentId)"
          :order-selection="orderSelection"
          :selected-source-ids="selectedSourceIds"
          :selected-target-ids="selectedTargetIds"
          :edit-session="editSession"
          :bookmarked="bookmarked.has(rows[virtualRow.index].alignmentId)"
          :highlighted="rows[virtualRow.index].alignmentId === transientHighlightId"
          :dragged-id="draggedId"
          :drop-target-id="dropTargetId"
          :drop-edge="dropEdge"
          :register-order-segment="registerOrderSegment"
          :can-insert-gap="canInsertGap"
          @select-alignment="selectRow(rows[virtualRow.index], $event)"
          @select-segment="(side, segmentId, event) => selectSegment(side, segmentId, rows[virtualRow.index].alignmentId, event)"
          @select-order-segment="(side, segmentId) => selectOrderSegment(side, segmentId, rows[virtualRow.index].alignmentId)"
          @request-edit="requestEdit($event, rows[virtualRow.index].alignmentId)"
          @insert-gap="(segmentId, edge) => emit('insertGap', segmentId, edge)"
          @bookmark="toggleBookmark(rows[virtualRow.index])"
          @edit-draft="emit('editDraft', $event)"
          @commit-edit="emit('commitEdit', $event)"
          @cancel-edit="emit('cancelEdit')"
          @escape-edit="emit('escapeEdit')"
        />
      </div>
    </div></div>
    <div v-if="mode === 'edit' && !editSession" class="edit-tip"><LockKeyhole :size="15" />双击任意句子进入编辑；Esc 保存并退出，Ctrl/⌘+Enter 保存并退出。</div>
    <OrderDragOverlay :dragged-segment="draggedSegment" :drag-pointer="dragPointer" />
    <SegmentContentDialog v-if="contentDialog" :operation="contentDialog" @close="closeContentDialog" @merge="confirmMergeContent" @split="confirmSplitContent" />
    <AlignmentUngroupDialog v-if="ungroupDialogAlignment" :alignment="ungroupDialogAlignment" :source-segments="sourceSegments" :target-segments="targetSegments" @close="ungroupDialogAlignment = null" @confirm="confirmUngroup" />
  </section>
</template>

<style scoped>
.virtual-alignment-block { position: absolute; z-index: 10; right: 0; left: 0; width: 100%; }
.operation-toolbar { display: flex; align-items: stretch; min-height: 58px; padding: 0 28px; border-bottom: 1px solid var(--line); background: var(--surface-muted); }.operation-group { display: flex; align-items: center; gap: 7px; min-width: 0; padding: 7px 16px 7px 0; }.operation-group + .operation-group { padding-left: 16px; border-left: 1px solid var(--line); }.operation-group__heading { display: grid; min-width: 73px; gap: 1px; line-height: 1.05; }.operation-group__heading span { color: var(--green-700); font-size: 9px; font-weight: 750; letter-spacing: .08em; }.operation-group__heading b { color: var(--ink-900); font-size: 12px; }.operation-group__heading small { color: var(--ink-500); font-size: 10px; }.operation-toolbar .tool-button { height: 32px; white-space: nowrap; }.operation-note { max-width: 220px; color: var(--ink-500); font-size: 10px; line-height: 1.35; }.operation-note--warning { color: #916714; }.tool-button--link:not(:disabled) { border-color: #a8d0ad; color: var(--green-900); background: var(--surface-green-soft); }.selection-clear { margin: auto 0 auto auto; padding: 5px 9px; border: 0; color: var(--ink-500); background: transparent; font-size: 12px; cursor: pointer; }
.view-find { display: flex; align-items: center; gap: 7px; min-height: 45px; padding: 0 18px; border-bottom: 1px solid #b8d5bb; color: var(--green-900); background: #f4faf2; }.view-find input { flex: 1; min-width: 140px; height: 31px; padding: 0 10px; border: 1px solid #b8cdb9; border-radius: 5px; outline: none; background: #fff; }.view-find input:focus { border-color: var(--green-700); box-shadow: 0 0 0 2px rgb(55 127 66 / 14%); }.view-find span { min-width: 70px; color: var(--ink-500); font-size: 11px; text-align: right; }.view-find button { display: grid; width: 30px; height: 30px; place-items: center; border: 1px solid transparent; border-radius: 5px; color: var(--ink-700); background: transparent; cursor: pointer; }.view-find button:hover:not(:disabled) { border-color: #bad2bd; background: #fff; }
.parallel-list { position: relative; min-height: 0; overflow: auto; overscroll-behavior: contain; scrollbar-gutter: stable; }
.workspace--trackpad .parallel-list { overscroll-behavior-x: none; overscroll-behavior-y: contain; scroll-behavior: auto; }
.parallel-list-inner { position: relative; width: 100%; }
</style>
