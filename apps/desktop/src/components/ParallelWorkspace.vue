<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch, type DirectiveBinding, type ObjectDirective } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import { combine } from "@atlaskit/pragmatic-drag-and-drop/combine";
import { draggable, dropTargetForElements } from "@atlaskit/pragmatic-drag-and-drop/adapter/element-adapter";
import { ArrowDown, ArrowUp, Check, GripVertical, Link2, Link2Off, LockKeyhole, Merge, RotateCcw, Scissors, Search, Star, X } from "@lucide/vue";
import type { EditSession } from "../composables/useViewModeController";
import type { AlignmentDto, SegmentDto, WorkspaceMode } from "../domain/kernel-client";

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
  move: [id: string, direction: "up" | "down"];
  reorder: [orderedSegmentIds: string[]];
  resetOrder: [];
  link: [sourceIds: string[], targetIds: string[]];
  unlink: [alignmentId: string];
  merge: [alignmentIds: string[]];
  split: [alignmentId: string, sourceGroups: string[][], targetGroups: string[][]];
  bookmark: [segmentId: string, alignmentId: string];
  status: [message: string];
}>();

type WorkspaceRow = { sources: SegmentDto[]; targets: SegmentDto[]; alignmentId: string; index: number };
type DropEdge = "before" | "after";
type OrderDirectiveValue = { mode: WorkspaceMode; row: WorkspaceRow; writable: boolean };
type DndRegistration = { key: string; cleanup: () => void };

const bookmarked = computed(() => new Set(props.bookmarkedAlignmentIds));
const selectedSourceIds = ref(new Set<string>());
const selectedTargetIds = ref(new Set<string>());
const selectedAlignmentIds = ref(new Set<string>());
const hasSegmentSelection = computed(() => selectedSourceIds.value.size > 0 || selectedTargetIds.value.size > 0);
const hasOperationSelection = computed(() => hasSegmentSelection.value || selectedAlignmentIds.value.size > 0);
const listRef = ref<HTMLElement | null>(null);
const findInputRef = ref<HTMLInputElement | null>(null);
const findOpen = ref(false);
const findQuery = ref("");
const deferredFindQuery = ref("");
const findCursor = ref(-1);
const transientHighlightId = ref("");
const draggedId = ref<string | null>(null);
const dropTargetId = ref<string | null>(null);
const dropEdge = ref<DropEdge | null>(null);
const dndRegistrations = new WeakMap<HTMLElement, DndRegistration>();
let highlightTimer: ReturnType<typeof setTimeout> | null = null;
let scrollAnimationFrame: number | null = null;
let resolveScrollAnimation: (() => void) | null = null;
let findDebounceTimer: ReturnType<typeof setTimeout> | null = null;

const rows = computed<WorkspaceRow[]>(() => {
  const sourceById = new Map(props.sourceSegments.map((segment) => [segment.id, segment]));
  const targetById = new Map(props.targetSegments.map((segment) => [segment.id, segment]));
  const usedSources = new Set<string>();
  const usedTargets = new Set<string>();
  const aligned = [...props.alignments].sort((a, b) => Math.min(...a.sourceIds.map((id) => sourceById.get(id)?.order ?? 999999)) - Math.min(...b.sourceIds.map((id) => sourceById.get(id)?.order ?? 999999)));
  const result: WorkspaceRow[] = [];
  for (const alignment of aligned) {
    const sources = alignment.sourceIds.map((id) => sourceById.get(id)).filter(Boolean) as SegmentDto[];
    const targets = alignment.targetIds.map((id) => targetById.get(id)).filter(Boolean) as SegmentDto[];
    sources.forEach((segment) => usedSources.add(segment.id));
    targets.forEach((segment) => usedTargets.add(segment.id));
    result.push({ sources, targets, alignmentId: alignment.id, index: result.length });
  }
  props.sourceSegments.filter((segment) => !usedSources.has(segment.id)).sort((a, b) => a.order - b.order).forEach((source) => result.push({ sources: [source], targets: [], alignmentId: `unlinked-${source.id}`, index: result.length }));
  props.targetSegments.filter((segment) => !usedTargets.has(segment.id)).sort((a, b) => a.order - b.order).forEach((target) => result.push({ sources: [], targets: [target], alignmentId: `unlinked-${target.id}`, index: result.length }));
  return result;
});

const rowBySegmentId = computed(() => {
  const result = new Map<string, WorkspaceRow>();
  for (const row of rows.value) [...row.sources, ...row.targets].forEach((segment) => result.set(segment.id, row));
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
function clearSelection() {
  selectedSourceIds.value = new Set();
  selectedTargetIds.value = new Set();
  selectedAlignmentIds.value = new Set();
}
defineExpose({ openFind, closeFind, navigateFind, clearSelection });

const requestEdit = (segment: SegmentDto, alignmentId: string) => {
  if (!props.writable) { emit("status", "当前为演示预览，请先新建或打开工程"); return; }
  // A browser double-click dispatches two click events first. Those clicks are
  // valid for alignment operations in Review, but must not leak into Edit.
  clearSelection();
  emit("select", alignmentId);
  emit("requestEdit", segment.id, alignmentId);
};
const toggleBookmark = (row: WorkspaceRow) => {
  if (!props.writable) { emit("status", "当前为演示预览，请先新建或打开工程"); return; }
  const segmentId = row.sources[0]?.id ?? row.targets[0]?.id;
  if (segmentId) emit("bookmark", segmentId, row.alignmentId);
};
const toggleSegment = (side: "source" | "target", id: string, alignmentId: string, event: MouseEvent) => {
  const selected = side === "source" ? selectedSourceIds : selectedTargetIds;
  const additive = event.metaKey || event.ctrlKey;
  const next = new Set(additive ? selected.value : selected.value.size === 1 && selected.value.has(id) ? selected.value : []);
  if (next.has(id)) next.delete(id); else next.add(id);
  selected.value = next;
  selectedAlignmentIds.value = new Set();
  emit("select", alignmentId);
};
const selectRow = (row: WorkspaceRow, event: MouseEvent) => {
  selectedSourceIds.value = new Set();
  selectedTargetIds.value = new Set();
  const additive = event.metaKey || event.ctrlKey;
  const next = new Set(additive ? selectedAlignmentIds.value : []);
  if (additive && next.has(row.alignmentId)) next.delete(row.alignmentId); else next.add(row.alignmentId);
  selectedAlignmentIds.value = next;
  emit("select", row.alignmentId);
};
watch(() => props.mode, (mode, previousMode) => {
  if (mode === "edit" || previousMode === "edit") clearSelection();
});
const selectedMergeable = computed(() => [...selectedAlignmentIds.value].filter((id) => props.alignments.some((alignment) => alignment.id === id)));
const selectedUnlinkableId = computed(() => selectedAlignmentIds.value.size === 1 && selectedMergeable.value.length === 1 ? selectedMergeable.value[0] : null);
const splitPlan = computed(() => {
  const alignmentId = [...selectedAlignmentIds.value][0];
  const alignment = props.alignments.find((candidate) => candidate.id === alignmentId);
  if (!alignment || alignment.sourceIds.length < 2 || alignment.targetIds.length < 2) return null;
  const sourceGroups = alignment.sourceIds.map((sourceId) => [sourceId]);
  const targetGroups = alignment.targetIds.map((targetId) => [targetId]);
  if (targetGroups.length < sourceGroups.length) return null;
  if (targetGroups.length > sourceGroups.length) targetGroups[sourceGroups.length - 1].push(...targetGroups.splice(sourceGroups.length).flat());
  return { alignmentId, sourceGroups, targetGroups };
});

const commitDrop = (segmentId: string, targetId: string, edge: DropEdge) => {
  const order = [...props.sourceSegments].sort((a, b) => a.order - b.order).map((segment) => segment.id);
  const fromIndex = order.indexOf(segmentId);
  if (fromIndex < 0) return;
  order.splice(fromIndex, 1);
  const targetIndex = order.indexOf(targetId);
  if (targetIndex < 0) return;
  order.splice(edge === "after" ? targetIndex + 1 : targetIndex, 0, segmentId);
  emit("reorder", order);
};

const clearDragState = () => { draggedId.value = null; dropTargetId.value = null; dropEdge.value = null; };
const bindOrderDnd = (element: HTMLElement, value: OrderDirectiveValue) => {
  const segmentId = value.row.sources[0]?.id;
  const key = `${value.mode}:${value.writable}:${segmentId ?? "none"}`;
  const existing = dndRegistrations.get(element);
  if (existing?.key === key) return;
  existing?.cleanup();
  if (value.mode !== "order" || !value.writable || !segmentId) { dndRegistrations.delete(element); return; }
  const cleanup = combine(
    draggable({
      element,
      dragHandle: element.querySelector(".drag-handle") ?? undefined,
      getInitialData: () => ({ kind: "jueming-source-segment", segmentId }),
      onDragStart: () => { draggedId.value = segmentId; },
      onDrop: clearDragState,
    }),
    dropTargetForElements({
      element,
      canDrop: ({ source }) => source.data.kind === "jueming-source-segment" && source.data.segmentId !== segmentId,
      getDropEffect: () => "move",
      getData: ({ input }) => {
        const bounds = element.getBoundingClientRect();
        const edge: DropEdge = input.clientY < bounds.top + bounds.height / 2 ? "before" : "after";
        return { kind: "jueming-source-drop-target", targetId: segmentId, edge };
      },
      onDragEnter: ({ self }) => { dropTargetId.value = String(self.data.targetId); dropEdge.value = self.data.edge === "after" ? "after" : "before"; },
      onDrag: ({ self }) => { dropTargetId.value = String(self.data.targetId); dropEdge.value = self.data.edge === "after" ? "after" : "before"; },
      onDragLeave: () => { if (dropTargetId.value === segmentId) { dropTargetId.value = null; dropEdge.value = null; } },
      onDrop: ({ source, self }) => {
        const sourceId = source.data.segmentId;
        const targetId = self.data.targetId;
        if (typeof sourceId === "string" && typeof targetId === "string" && sourceId !== targetId) commitDrop(sourceId, targetId, self.data.edge === "after" ? "after" : "before");
        clearDragState();
      },
    }),
  );
  dndRegistrations.set(element, { key, cleanup });
};

const vOrderDnd: ObjectDirective<HTMLElement, OrderDirectiveValue> = {
  mounted: (element, binding: DirectiveBinding<OrderDirectiveValue>) => bindOrderDnd(element, binding.value),
  updated: (element, binding: DirectiveBinding<OrderDirectiveValue>) => bindOrderDnd(element, binding.value),
  unmounted: (element) => { dndRegistrations.get(element)?.cleanup(); dndRegistrations.delete(element); },
};

const alignmentKind = (row: WorkspaceRow) => !row.sources.length || !row.targets.length ? "未对齐" : `${row.sources.length}:${row.targets.length}`;
const idsLabel = (segments: SegmentDto[]) => segments.map((segment) => String(segment.order + 1).padStart(6, "0")).join(" · ") || "—";
const stableIdsLabel = (segments: SegmentDto[]) => segments.map((segment) => segment.id).join("\n");
onBeforeUnmount(() => {
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
      <button class="tool-button" type="button" :disabled="!writable || !selectedAlignmentId" @click="emit('move', selectedAlignmentId, 'up')"><ArrowUp :size="15" />上移</button>
      <button class="tool-button" type="button" :disabled="!writable || !selectedAlignmentId" @click="emit('move', selectedAlignmentId, 'down')"><ArrowDown :size="15" />下移</button>
      <button class="tool-button" type="button" :disabled="!writable" @click="emit('resetOrder')"><RotateCcw :size="15" />恢复顺序</button>
      <span class="order-hint">{{ writable ? `共 ${rows.length} 句 · 拖动中文卡片手柄调整顺序` : '演示预览 · 打开工程后可排序' }}</span>
    </div>
    <div v-if="findOpen && mode === 'review'" class="view-find" role="search" aria-label="审阅模式快速查找">
      <Search :size="16" /><input ref="findInputRef" v-model="findQuery" aria-label="查找当前平行视图" placeholder="查找中文或英文…" @keydown.enter.prevent="activateFindMatch(findCursor + ($event.shiftKey ? -1 : 1))" @keydown.esc.prevent="closeFind" />
      <span>{{ findPending ? '查找中…' : findMatches.length ? `${findCursor + 1} / ${findMatches.length}` : findQuery ? '无结果' : '输入关键词' }}</span>
      <button type="button" title="上一处" :disabled="!findMatches.length" @click="activateFindMatch(findCursor - 1)"><ArrowUp :size="15" /></button><button type="button" title="下一处" :disabled="!findMatches.length" @click="activateFindMatch(findCursor + 1)"><ArrowDown :size="15" /></button><button type="button" title="关闭查找" @click="closeFind"><X :size="16" /></button>
    </div>
    <div class="selection-toolbar" aria-label="Alignment 操作工具"><span class="selection-count">{{ writable ? '已选' : '演示预览' }} <template v-if="writable"><b>{{ selectedSourceIds.size }}</b> 条中文 · <b>{{ selectedTargetIds.size }}</b> 条英文</template><template v-else>· 请先新建或打开工程</template></span><button class="tool-button tool-button--link" type="button" :disabled="!writable || !selectedSourceIds.size || !selectedTargetIds.size" title="分别选择至少一条中文和一条英文" @click="emit('link', [...selectedSourceIds], [...selectedTargetIds])"><Link2 :size="15" />Link</button><button class="tool-button" type="button" :disabled="!writable || !selectedUnlinkableId" title="点击中间关系图标选择一个 Alignment" @click="selectedUnlinkableId && emit('unlink', selectedUnlinkableId)"><Link2Off :size="15" />Unlink</button><button class="tool-button" type="button" :disabled="!writable || selectedMergeable.length < 2" title="按 Ctrl/⌘ 点击中间关系图标多选 Alignment" @click="emit('merge', selectedMergeable)"><Merge :size="15" />Merge</button><button class="tool-button" type="button" :disabled="!writable || !splitPlan" title="需要至少两个中文和两个英文句段的明确分组" @click="splitPlan && emit('split', splitPlan.alignmentId, splitPlan.sourceGroups, splitPlan.targetGroups)"><Scissors :size="15" />Split</button><button v-if="hasOperationSelection" class="selection-clear" type="button" @click="clearSelection">清除选择</button></div>
    <div class="column-headings"><h2>中文 <span>（原文）</span></h2><div class="heading-divider" aria-hidden="true"></div><h2>English <span>（译文）</span></h2></div>
    <div ref="listRef" class="parallel-list" :class="{ 'parallel-list--order': mode === 'order' }" tabindex="-1" @wheel.passive="cancelScrollAnimation" @touchstart.passive="cancelScrollAnimation"><div class="parallel-list-inner" :style="{ height: `${totalSize}px` }">
      <article v-for="virtualRow in virtualRows" :key="rows[virtualRow.index].alignmentId" v-order-dnd="{ mode, row: rows[virtualRow.index], writable }" :ref="measureRow" :data-index="virtualRow.index" :data-alignment-id="rows[virtualRow.index].alignmentId" class="alignment-row" :class="{ 'alignment-row--selected': selectedAlignmentIds.has(rows[virtualRow.index].alignmentId) || (!hasOperationSelection && rows[virtualRow.index].alignmentId === selectedAlignmentId), 'alignment-row--operation-selected': selectedAlignmentIds.has(rows[virtualRow.index].alignmentId), 'alignment-row--highlight': rows[virtualRow.index].alignmentId === transientHighlightId, 'alignment-row--empty': !rows[virtualRow.index].sources.length || !rows[virtualRow.index].targets.length, 'alignment-row--dragging': draggedId === rows[virtualRow.index].sources[0]?.id, 'alignment-row--drop-before': dropTargetId === rows[virtualRow.index].sources[0]?.id && dropEdge === 'before', 'alignment-row--drop-after': dropTargetId === rows[virtualRow.index].sources[0]?.id && dropEdge === 'after' }" :style="{ transform: `translateY(${virtualRow.start}px)` }" @click="selectRow(rows[virtualRow.index], $event)">
        <div class="segment-cell segment-cell--source" :class="{ 'segment-cell--selected': !hasOperationSelection && rows[virtualRow.index].alignmentId === selectedAlignmentId, 'segment-cell--multi-selected': rows[virtualRow.index].sources.some(source => selectedSourceIds.has(source.id)) }" @click.stop="rows[virtualRow.index].sources[0] && toggleSegment('source', rows[virtualRow.index].sources[0].id, rows[virtualRow.index].alignmentId, $event)"><button v-if="mode === 'order' && rows[virtualRow.index].sources[0]" class="drag-handle" type="button" :disabled="!writable" title="拖动卡片；也可用上移和下移按钮" @click.stop="emit('select', rows[virtualRow.index].alignmentId)"><GripVertical :size="18" /><span>{{ String(rows[virtualRow.index].index + 1).padStart(2, '0') }}</span></button><span v-else class="segment-id" :title="stableIdsLabel(rows[virtualRow.index].sources)">{{ idsLabel(rows[virtualRow.index].sources) }}</span><div v-for="source in rows[virtualRow.index].sources" :key="source.id" class="segment-stack">
          <div v-if="mode === 'edit' && editSession?.segmentId === source.id" class="edit-card"><textarea :value="editSession.draft" autofocus aria-label="编辑中文原文" @click.stop @input="emit('editDraft', ($event.target as HTMLTextAreaElement).value)" @keydown.esc.stop.prevent="emit('escapeEdit')" @keydown.ctrl.enter.stop.prevent="emit('commitEdit', true)" @keydown.meta.enter.stop.prevent="emit('commitEdit', true)"></textarea><div class="edit-meta"><span>字数: {{ editSession.draft.length }}</span><span class="spell-status"><Check :size="15" />{{ editSession.status === 'saving' ? '保存中' : editSession.status === 'error' ? '保存失败' : editSession.status === 'dirty' ? '待保存' : '已保存' }}</span><Link2 :size="17" /></div><p v-if="editSession.error" class="edit-error">{{ editSession.error }}</p><div class="edit-actions"><button type="button" @click.stop="emit('cancelEdit')"><X :size="14" />放弃并退出</button><button class="primary-button" type="button" :disabled="editSession.status === 'saving'" @click.stop="emit('commitEdit', true)"><Check :size="14" />保存并退出</button></div></div>
          <button v-else class="segment-text" type="button" @click.stop="toggleSegment('source', source.id, rows[virtualRow.index].alignmentId, $event)" @dblclick.stop="requestEdit(source, rows[virtualRow.index].alignmentId)">{{ source.text }}</button>
        </div><button v-if="props.alignments.some(alignment => alignment.id === rows[virtualRow.index].alignmentId)" class="bookmark-button" :class="{ 'bookmark-button--on': bookmarked.has(rows[virtualRow.index].alignmentId) }" type="button" :disabled="!writable" :aria-label="bookmarked.has(rows[virtualRow.index].alignmentId) ? '移除书签' : '添加书签'" @click.stop="toggleBookmark(rows[virtualRow.index])"><Star :size="18" :fill="bookmarked.has(rows[virtualRow.index].alignmentId) ? 'currentColor' : 'none'" /></button></div>
        <div class="link-cell" :class="{ 'link-cell--active': selectedAlignmentIds.has(rows[virtualRow.index].alignmentId) || (!hasOperationSelection && rows[virtualRow.index].alignmentId === selectedAlignmentId) }" @click.stop="selectRow(rows[virtualRow.index], $event)"><Link2 :size="23" /><span v-if="mode === 'order'" class="order-index">{{ String(rows[virtualRow.index].index + 1).padStart(2, '0') }}</span></div>
        <div class="segment-cell segment-cell--target" :class="{ 'segment-cell--selected': !hasOperationSelection && rows[virtualRow.index].alignmentId === selectedAlignmentId, 'segment-cell--multi-selected': rows[virtualRow.index].targets.some(target => selectedTargetIds.has(target.id)) }" @click.stop="rows[virtualRow.index].targets[0] && toggleSegment('target', rows[virtualRow.index].targets[0].id, rows[virtualRow.index].alignmentId, $event)"><span class="segment-id" :title="stableIdsLabel(rows[virtualRow.index].targets)">{{ idsLabel(rows[virtualRow.index].targets) }}</span><div v-for="target in rows[virtualRow.index].targets" :key="target.id" class="segment-stack">
          <div v-if="mode === 'edit' && editSession?.segmentId === target.id" class="edit-card"><textarea :value="editSession.draft" autofocus aria-label="编辑英文译文" @click.stop @input="emit('editDraft', ($event.target as HTMLTextAreaElement).value)" @keydown.esc.stop.prevent="emit('escapeEdit')" @keydown.ctrl.enter.stop.prevent="emit('commitEdit', true)" @keydown.meta.enter.stop.prevent="emit('commitEdit', true)"></textarea><div class="edit-meta"><span>Words: {{ editSession.draft.trim().split(/\s+/).filter(Boolean).length }}</span><span class="spell-status"><Check :size="15" />{{ editSession.status === 'saving' ? 'Saving' : editSession.status === 'error' ? 'Failed' : editSession.status === 'dirty' ? 'Pending' : 'Saved' }}</span><Link2 :size="17" /></div><p v-if="editSession.error" class="edit-error">{{ editSession.error }}</p><div class="edit-actions"><button type="button" @click.stop="emit('cancelEdit')"><X :size="14" />放弃并退出</button><button class="primary-button" type="button" :disabled="editSession.status === 'saving'" @click.stop="emit('commitEdit', true)"><Check :size="14" />保存并退出</button></div></div>
          <button v-else class="segment-text segment-text--target" type="button" @click.stop="toggleSegment('target', target.id, rows[virtualRow.index].alignmentId, $event)" @dblclick.stop="requestEdit(target, rows[virtualRow.index].alignmentId)">{{ target.text }}</button>
        </div><span class="alignment-kind" :class="{ 'alignment-kind--empty': !rows[virtualRow.index].sources.length || !rows[virtualRow.index].targets.length }">{{ alignmentKind(rows[virtualRow.index]) }}</span></div>
      </article>
    </div></div>
    <div v-if="mode === 'edit' && !editSession" class="edit-tip"><LockKeyhole :size="15" />双击任意句子进入编辑；Esc 保存并退出，Ctrl/⌘+Enter 保存并退出。</div>
  </section>
</template>

<style scoped>
.selection-toolbar { display: flex; align-items: center; gap: 8px; min-height: 49px; padding: 0 28px; border-bottom: 1px solid var(--line); background: #fbfcfb; }.selection-count { margin-right: 10px; color: var(--ink-500); font-size: 12px; }.selection-count b { color: var(--green-900); }.selection-toolbar .tool-button { height: 32px; }.tool-button--link:not(:disabled) { border-color: #a8d0ad; color: var(--green-900); background: #eef8ec; }.selection-clear { margin-left: auto; padding: 5px 9px; border: 0; color: var(--ink-500); background: transparent; font-size: 12px; cursor: pointer; }
.view-find { display: flex; align-items: center; gap: 7px; min-height: 45px; padding: 0 18px; border-bottom: 1px solid #b8d5bb; color: var(--green-900); background: #f4faf2; }.view-find input { flex: 1; min-width: 140px; height: 31px; padding: 0 10px; border: 1px solid #b8cdb9; border-radius: 5px; outline: none; background: #fff; }.view-find input:focus { border-color: var(--green-700); box-shadow: 0 0 0 2px rgb(55 127 66 / 14%); }.view-find span { min-width: 70px; color: var(--ink-500); font-size: 11px; text-align: right; }.view-find button { display: grid; width: 30px; height: 30px; place-items: center; border: 1px solid transparent; border-radius: 5px; color: var(--ink-700); background: transparent; cursor: pointer; }.view-find button:hover:not(:disabled) { border-color: #bad2bd; background: #fff; }
.parallel-list { position: relative; min-height: 0; overflow: auto; overscroll-behavior: contain; scrollbar-gutter: stable; }.workspace--trackpad .parallel-list { overscroll-behavior-x: none; overscroll-behavior-y: contain; scroll-behavior: auto; }.parallel-list-inner { position: relative; width: 100%; }.alignment-row { position: absolute; left: 0; display: grid; grid-template-columns: minmax(0, 1fr) 66px minmax(0, 1fr); width: 100%; min-height: 106px; border-bottom: 1px solid var(--line); cursor: pointer; transition: background-color 180ms ease, box-shadow 180ms ease, opacity 160ms ease; }.alignment-row--operation-selected { box-shadow: inset 0 0 0 2px #80b686; }.alignment-row--highlight { animation: alignment-highlight 820ms ease-out; }.alignment-row--dragging { opacity: .42; }.alignment-row--drop-before::before, .alignment-row--drop-after::after { position: absolute; z-index: 4; right: 10px; left: 10px; height: 3px; border-radius: 3px; background: var(--green-700); box-shadow: 0 0 0 3px rgb(55 127 66 / 14%); content: ""; }.alignment-row--drop-before::before { top: -1px; }.alignment-row--drop-after::after { bottom: -2px; }.workspace--order .alignment-row { cursor: default; user-select: none; }.workspace--order .segment-cell--source { padding-left: 104px; }.workspace--order .drag-handle { cursor: grab; }.workspace--order .drag-handle:active { cursor: grabbing; }.drag-handle { position: absolute; top: 0; bottom: 0; left: 0; display: flex; width: 78px; align-items: center; justify-content: center; gap: 5px; border: 0; border-right: 1px solid var(--line); color: var(--ink-500); background: #f8faf8; font: inherit; }.drag-handle:disabled { cursor: not-allowed; opacity: .5; }.segment-cell--multi-selected { outline: 2px solid #80b686; outline-offset: -3px; background: #eff9ed; }.edit-error { margin: 0; padding: 6px 12px; color: #a24c4c; background: #fff0f0; font-size: 11px; }
@keyframes alignment-highlight { 0% { box-shadow: inset 0 0 0 3px rgb(77 155 83 / 45%), 0 0 0 0 rgb(77 155 83 / 24%); } 45% { box-shadow: inset 0 0 0 2px rgb(77 155 83 / 26%), 0 0 0 8px rgb(77 155 83 / 0%); } 100% { box-shadow: inset 0 0 0 0 rgb(77 155 83 / 0%); } }
@media (prefers-reduced-motion: reduce) { .alignment-row { transition: none; } .alignment-row--highlight { animation: none; } }
</style>
