<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import { ArrowDown, ArrowUp, Check, GripVertical, Link2, Link2Off, LockKeyhole, Merge, RotateCcw, Scissors, Star, X } from "@lucide/vue";
import type { AlignmentDto, SegmentDto, WorkspaceMode } from "../domain/kernel-client";

const props = withDefaults(defineProps<{
  mode: WorkspaceMode;
  sourceSegments: SegmentDto[];
  targetSegments: SegmentDto[];
  alignments: AlignmentDto[];
  bookmarkedAlignmentIds?: string[];
  selectedAlignmentId: string;
}>(), { bookmarkedAlignmentIds: () => [], selectedAlignmentId: "" });

const emit = defineEmits<{
  select: [alignmentId: string];
  saveSegment: [id: string, text: string];
  cancelEdit: [];
  move: [id: string, direction: "up" | "down"];
  reorder: [segmentId: string, beforeSegmentId: string];
  resetOrder: [];
  link: [sourceIds: string[], targetIds: string[]];
  unlink: [alignmentId: string];
  merge: [alignmentIds: string[]];
  split: [alignmentId: string, sourceGroups: string[][], targetGroups: string[][]];
  bookmark: [segmentId: string, alignmentId: string];
  status: [message: string];
}>();

const editingId = ref<string | null>(null);
const draft = ref("");
const draggedId = ref<string | null>(null);
const bookmarked = computed(() => new Set(props.bookmarkedAlignmentIds));
const selectedSourceIds = ref(new Set<string>());
const selectedTargetIds = ref(new Set<string>());
const selectedAlignmentIds = ref(new Set<string>());
const listRef = ref<HTMLElement | null>(null);

type WorkspaceRow = { sources: SegmentDto[]; targets: SegmentDto[]; alignmentId: string; index: number };
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
const virtualizer = useVirtualizer({ getScrollElement: () => listRef.value, count: rows.value.length, estimateSize: () => 112, overscan: 8 });
const virtualRows = computed(() => virtualizer.value.getVirtualItems());
const totalSize = computed(() => virtualizer.value.getTotalSize());
watch(() => rows.value.length, (count) => virtualizer.value.setOptions({ ...virtualizer.value.options, count }));
const measureRow = (element: unknown) => { if (element instanceof Element) virtualizer.value.measureElement(element); };

const beginEdit = (segment: SegmentDto) => { editingId.value = segment.id; draft.value = segment.text; };
const saveEdit = (segment: SegmentDto) => { emit("saveSegment", segment.id, draft.value.trim() || segment.text); editingId.value = null; };
const toggleBookmark = (row: WorkspaceRow) => { const segmentId = row.sources[0]?.id ?? row.targets[0]?.id; if (segmentId) emit("bookmark", segmentId, row.alignmentId); };
const toggleSegment = (side: "source" | "target", id: string, alignmentId: string, event: MouseEvent) => {
  const selected = side === "source" ? selectedSourceIds : selectedTargetIds;
  const next = new Set(event.metaKey || event.ctrlKey ? selected.value : []);
  if (next.has(id)) next.delete(id); else next.add(id);
  selected.value = next;
  selectedAlignmentIds.value = new Set(event.metaKey || event.ctrlKey ? selectedAlignmentIds.value : []);
  selectedAlignmentIds.value.add(alignmentId);
  emit("select", alignmentId);
};
const selectRow = (row: WorkspaceRow, event: MouseEvent) => {
  const next = new Set(event.metaKey || event.ctrlKey ? selectedAlignmentIds.value : []);
  if (next.has(row.alignmentId)) next.delete(row.alignmentId); else next.add(row.alignmentId);
  selectedAlignmentIds.value = next;
  emit("select", row.alignmentId);
};
const clearSelection = () => { selectedSourceIds.value = new Set(); selectedTargetIds.value = new Set(); selectedAlignmentIds.value = new Set(); };
const selectedMergeable = computed(() => [...selectedAlignmentIds.value].filter((id) => props.alignments.some((alignment) => alignment.id === id)));
const splitPlan = computed(() => {
  const alignmentId = [...selectedAlignmentIds.value][0];
  const alignment = props.alignments.find((candidate) => candidate.id === alignmentId);
  if (!alignment || alignment.sourceIds.length < 2 || alignment.targetIds.length < 2) return null;
  const sourceGroups = alignment.sourceIds.map((sourceId) => [sourceId]);
  const targetGroups = alignment.targetIds.map((targetId) => [targetId]);
  if (targetGroups.length < sourceGroups.length) return null;
  if (targetGroups.length > sourceGroups.length) {
    targetGroups[sourceGroups.length - 1].push(...targetGroups.splice(sourceGroups.length).flat());
  }
  return { alignmentId, sourceGroups, targetGroups };
});
const focusAlignment = async (alignmentId: string) => {
  const rowIndex = rows.value.findIndex((row) => row.alignmentId === alignmentId);
  if (rowIndex < 0) return;
  virtualizer.value.scrollToIndex(rowIndex, { align: "center" });
  await nextTick();
  listRef.value?.querySelector(`[data-alignment-id="${alignmentId}"]`)?.scrollIntoView({ block: "nearest" });
};
watch(() => props.selectedAlignmentId, (alignmentId) => { if (alignmentId) void focusAlignment(alignmentId); });
const dropSegment = (targetId: string) => {
  if (draggedId.value && targetId && draggedId.value !== targetId) emit("reorder", draggedId.value, targetId);
  draggedId.value = null;
};
const alignmentKind = (row: WorkspaceRow) => !row.sources.length || !row.targets.length ? "未对齐" : `${row.sources.length}:${row.targets.length}`;
const idsLabel = (segments: SegmentDto[]) => segments.map((segment) => segment.id.slice(0, 8)).join(" · ") || "—";
</script>

<template>
  <section class="workspace" :class="`workspace--${mode}`" aria-label="双语平行工作区">
    <div v-if="mode === 'order'" class="order-toolbar"><div class="order-label"><GripVertical :size="16" /><span>排序工具：</span></div><button class="tool-button tool-button--active" type="button"><GripVertical :size="15" />拖动排序</button><button class="tool-button" type="button" :disabled="!selectedAlignmentId" @click="emit('move', selectedAlignmentId, 'up')"><ArrowUp :size="15" />上移</button><button class="tool-button" type="button" :disabled="!selectedAlignmentId" @click="emit('move', selectedAlignmentId, 'down')"><ArrowDown :size="15" />下移</button><button class="tool-button" type="button" @click="emit('resetOrder')"><RotateCcw :size="15" />恢复顺序</button><span class="order-hint">共 {{ rows.length }} 句 <i></i> 拖动句子调整顺序</span></div>
    <template v-if="mode !== 'history'">
      <div class="selection-toolbar" aria-label="Alignment 操作工具"><span class="selection-count">已选 <b>{{ selectedSourceIds.size }}</b> 条中文 · <b>{{ selectedTargetIds.size }}</b> 条英文</span><button class="tool-button tool-button--link" type="button" :disabled="!selectedSourceIds.size || !selectedTargetIds.size" title="选中至少一条中文和一条英文" @click="emit('link', [...selectedSourceIds], [...selectedTargetIds])"><Link2 :size="15" />Link</button><button class="tool-button" type="button" :disabled="selectedAlignmentIds.size !== 1" title="解除当前 Alignment" @click="emit('unlink', [...selectedAlignmentIds][0])"><Link2Off :size="15" />Unlink</button><button class="tool-button" type="button" :disabled="selectedMergeable.length < 2" title="按 Ctrl/⌘ 多选 Alignment 后合并" @click="emit('merge', selectedMergeable)"><Merge :size="15" />Merge</button><button class="tool-button" type="button" :disabled="!splitPlan" title="需要至少两个中文和两个英文句段的明确分组" @click="splitPlan && emit('split', splitPlan.alignmentId, splitPlan.sourceGroups, splitPlan.targetGroups)"><Scissors :size="15" />Split</button><button v-if="selectedSourceIds.size || selectedTargetIds.size || selectedAlignmentIds.size" class="selection-clear" type="button" @click="clearSelection">清除选择</button></div>
      <div class="column-headings"><h2>中文 <span>（原文）</span></h2><div class="heading-divider" aria-hidden="true"></div><h2>English <span>（译文）</span></h2></div>
      <div ref="listRef" class="parallel-list" :class="{ 'parallel-list--order': mode === 'order' }"><div class="parallel-list-inner" :style="{ height: `${totalSize}px` }"><article v-for="virtualRow in virtualRows" :key="rows[virtualRow.index].alignmentId" :ref="measureRow" :data-alignment-id="rows[virtualRow.index].alignmentId" class="alignment-row" :class="{ 'alignment-row--selected': rows[virtualRow.index].alignmentId === selectedAlignmentId, 'alignment-row--empty': !rows[virtualRow.index].sources.length || !rows[virtualRow.index].targets.length, 'alignment-row--dragging': draggedId === rows[virtualRow.index].sources[0]?.id }" :style="{ transform: `translateY(${virtualRow.start}px)` }" @click="selectRow(rows[virtualRow.index], $event)" @dragover.prevent @drop.prevent="dropSegment(rows[virtualRow.index].sources[0]?.id ?? rows[virtualRow.index].targets[0]?.id ?? '')">
          <div v-if="mode === 'order' && rows[virtualRow.index].sources[0]" class="drag-handle" draggable="true" @dragstart="draggedId = rows[virtualRow.index].sources[0].id" @dragend="draggedId = null"><GripVertical :size="18" /><span>{{ String(rows[virtualRow.index].index + 1).padStart(2, '0') }}</span></div>
          <div class="segment-cell segment-cell--source" :class="{ 'segment-cell--selected': rows[virtualRow.index].alignmentId === selectedAlignmentId, 'segment-cell--multi-selected': rows[virtualRow.index].sources.some(source => selectedSourceIds.has(source.id)) }" @click.stop="rows[virtualRow.index].sources[0] && toggleSegment('source', rows[virtualRow.index].sources[0].id, rows[virtualRow.index].alignmentId, $event)">
            <span class="segment-id">{{ idsLabel(rows[virtualRow.index].sources) }}</span>
            <div v-for="source in rows[virtualRow.index].sources" :key="source.id" class="segment-stack">
              <template v-if="mode === 'edit' && editingId === source.id"><div class="edit-card"><textarea v-model="draft" aria-label="编辑中文原文" @click.stop></textarea><div class="edit-meta"><span>字数: {{ draft.length }}</span><span class="spell-status"><Check :size="15" />拼写检查</span><Link2 :size="17" /></div><div class="edit-actions"><button type="button" @click.stop="emit('cancelEdit'); editingId = null"><X :size="14" />取消</button><button class="primary-button" type="button" @click.stop="saveEdit(source)"><Check :size="14" />保存</button></div></div></template>
              <button v-else class="segment-text" type="button" @click.stop="toggleSegment('source', source.id, rows[virtualRow.index].alignmentId, $event)" @dblclick.stop="beginEdit(source)">{{ source.text }}</button>
            </div>
            <button v-if="props.alignments.some(alignment => alignment.id === rows[virtualRow.index].alignmentId)" class="bookmark-button" :class="{ 'bookmark-button--on': bookmarked.has(rows[virtualRow.index].alignmentId) }" type="button" :aria-label="bookmarked.has(rows[virtualRow.index].alignmentId) ? '移除书签' : '添加书签'" @click.stop="toggleBookmark(rows[virtualRow.index])"><Star :size="18" :fill="bookmarked.has(rows[virtualRow.index].alignmentId) ? 'currentColor' : 'none'" /></button>
          </div>
          <div class="link-cell" :class="{ 'link-cell--active': rows[virtualRow.index].alignmentId === selectedAlignmentId }" @click.stop="focusAlignment(rows[virtualRow.index].alignmentId); emit('select', rows[virtualRow.index].alignmentId)"><Link2 :size="23" /><span v-if="mode === 'order'" class="order-index">{{ String(rows[virtualRow.index].index + 1).padStart(2, '0') }}</span></div>
          <div class="segment-cell segment-cell--target" :class="{ 'segment-cell--selected': rows[virtualRow.index].alignmentId === selectedAlignmentId, 'segment-cell--multi-selected': rows[virtualRow.index].targets.some(target => selectedTargetIds.has(target.id)) }" @click.stop="rows[virtualRow.index].targets[0] && toggleSegment('target', rows[virtualRow.index].targets[0].id, rows[virtualRow.index].alignmentId, $event)">
            <span class="segment-id">{{ idsLabel(rows[virtualRow.index].targets) }}</span>
            <div v-for="target in rows[virtualRow.index].targets" :key="target.id" class="segment-stack">
              <template v-if="mode === 'edit' && editingId === target.id"><div class="edit-card"><textarea v-model="draft" aria-label="编辑英文译文" @click.stop></textarea><div class="edit-meta"><span>Words: {{ draft.trim().split(/\\s+/).filter(Boolean).length }}</span><span class="spell-status"><Check :size="15" />Spell Check</span><Link2 :size="17" /></div><div class="edit-actions"><button type="button" @click.stop="emit('cancelEdit'); editingId = null"><X :size="14" />取消</button><button class="primary-button" type="button" @click.stop="saveEdit(target)"><Check :size="14" />保存</button></div></div></template>
              <button v-else class="segment-text segment-text--target" type="button" @click.stop="toggleSegment('target', target.id, rows[virtualRow.index].alignmentId, $event)" @dblclick.stop="beginEdit(target)">{{ target.text }}</button>
            </div>
            <span class="alignment-kind" :class="{ 'alignment-kind--empty': !rows[virtualRow.index].sources.length || !rows[virtualRow.index].targets.length }">{{ alignmentKind(rows[virtualRow.index]) }}</span>
          </div>
          <div v-if="mode === 'order' && draggedId === rows[virtualRow.index].sources[0]?.id" class="drop-slot"><ArrowDown :size="15" />将句子拖放到此处</div>
        </article></div></div>
      <div v-if="mode === 'edit' && editingId === null" class="edit-tip"><LockKeyhole :size="15" />双击任意句子进入编辑；Ctrl/⌘ 点击可多选，先选两侧句段再 Link。</div>
    </template>
    <div v-else class="history-view"><header class="history-header"><div><span class="eyebrow">REVISION HISTORY</span><h2>历史记录</h2><p>比较工程版本，恢复操作会创建新的 Revision。</p></div><div class="history-actions"><button class="tool-button" type="button" @click="emit('status', '已选择两个版本进行比较')"><Link2 :size="15" />比较</button><button class="tool-button" type="button" @click="emit('status', '恢复版本需要接入 KernelClient')"><RotateCcw :size="15" />恢复此版本</button></div></header><div class="history-layout"><aside class="revision-list"><button class="revision-card" type="button"><strong>R128 <small>当前版本</small></strong><time>今天 15:42</time><b>Edit Segment</b><span>编辑了第 000104 段的英文译文，优化表达和结构。</span></button><button class="revision-card" type="button"><strong>R121</strong><time>今天 14:37</time><b>Merge Alignment</b><span>合并了相邻的 2 个对齐段落。</span></button><button class="revision-card" type="button"><strong>R117</strong><time>今天 11:09</time><b>Edit Segment</b><span>编辑了第 000103 段的中文原文。</span></button></aside><section class="revision-diff"><div class="diff-caption"><span>比较：<b>R117 → R128</b></span><span>段落：000104 <Link2 :size="15" /></span></div><div class="diff-grid"><div><small>R117（旧版本）</small><p class="diff-old">We steadily promote high-quality development, speed up the building of a new development pattern, and the quality and benefit of economic growth keep improving.</p></div><div><small>R128（当前版本）</small><p class="diff-new">We advanced high-quality development in depth, accelerated the building of a new development paradigm, and continuously improved the quality and efficiency of economic growth.</p></div></div><div class="change-summary"><b>变更摘要</b><p><span class="diff-delete">− 删除 1 行</span><span class="diff-add">＋ 添加 1 行</span></p><p>优化英文表达，使用更准确的词汇。</p></div></section></div></div>
  </section>
</template>

<style scoped>
.selection-toolbar { display: flex; align-items: center; gap: 8px; min-height: 49px; padding: 0 28px; border-bottom: 1px solid var(--line); background: #fbfcfb; }.selection-count { margin-right: 10px; color: var(--ink-500); font-size: 12px; }.selection-count b { color: var(--green-900); }.selection-toolbar .tool-button { height: 32px; }.tool-button--link:not(:disabled) { border-color: #a8d0ad; color: var(--green-900); background: #eef8ec; }.selection-clear { margin-left: auto; padding: 5px 9px; border: 0; color: var(--ink-500); background: transparent; font-size: 12px; cursor: pointer; }.parallel-list { position: relative; min-height: 0; overflow: auto; }.parallel-list-inner { position: relative; width: 100%; }.alignment-row { position: absolute; left: 0; display: grid; grid-template-columns: minmax(0, 1fr) 66px minmax(0, 1fr); width: 100%; min-height: 106px; border-bottom: 1px solid var(--line); cursor: pointer; }.segment-cell--multi-selected { outline: 2px solid #80b686; outline-offset: -3px; background: #eff9ed; }.unlinked-strip { display: flex; align-items: center; gap: 8px; position: absolute; right: 18px; bottom: 16px; left: 18px; padding: 9px 12px; border: 1px dashed #d7b85f; border-radius: 6px; color: #8d6d20; background: #fffaf0; font-size: 12px; }.unlinked-strip button { padding: 3px 7px; border: 1px solid #dfc983; border-radius: 4px; color: #84651b; background: #fff; font-family: ui-monospace, monospace; font-size: 11px; cursor: pointer; }.parallel-list:has(.unlinked-strip) { padding-bottom: 55px; }
.history-view { display: flex; flex-direction: column; height: 100%; min-height: 0; background: #fbfcfb; }.history-header { display: flex; align-items: flex-start; justify-content: space-between; padding: 24px 30px 20px; border-bottom: 1px solid var(--line); }.history-header h2 { margin: 4px 0; color: var(--ink-900); font-size: 21px; }.history-header p { margin: 0; color: var(--ink-500); font-size: 12px; }.history-actions { display: flex; gap: 9px; }.history-layout { display: grid; grid-template-columns: 290px minmax(0, 1fr); min-height: 0; flex: 1; }.revision-list { overflow: auto; padding: 18px 14px 18px 24px; border-right: 1px solid var(--line); }.revision-card { display: grid; grid-template-columns: 1fr auto; gap: 7px; width: 100%; margin-bottom: 11px; padding: 14px; border: 1px solid #e1e7e1; border-radius: 8px; background: #fff; text-align: left; cursor: pointer; }.revision-card:hover { border-color: #a7cfa9; background: #f5fbf3; }.revision-card strong { color: var(--green-900); font-size: 14px; }.revision-card strong small { margin-left: 4px; padding: 2px 5px; border-radius: 4px; color: var(--green-700); background: #e4f2e2; font-size: 10px; }.revision-card time { color: var(--ink-500); font-size: 10px; }.revision-card b { grid-column: 1 / -1; color: var(--ink-900); font-size: 13px; }.revision-card span { grid-column: 1 / -1; color: var(--ink-700); font-size: 11px; line-height: 1.5; }.revision-diff { overflow: auto; padding: 24px 28px; }.diff-caption { display: flex; justify-content: space-between; margin-bottom: 19px; color: var(--ink-700); font-size: 13px; }.diff-grid { display: grid; grid-template-columns: 1fr 1fr; border: 1px solid var(--line); border-radius: 7px; overflow: hidden; }.diff-grid > div { min-height: 210px; padding: 15px 18px; }.diff-grid > div + div { border-left: 1px solid var(--line); }.diff-grid small { color: var(--ink-500); font-size: 11px; }.diff-grid p { margin: 18px 0 0; padding: 12px; color: var(--ink-900); font-size: 14px; line-height: 1.65; }.diff-old { background: #fff0f0; text-decoration: line-through; text-decoration-color: #bc7272; }.diff-new { background: #eff9ec; }.change-summary { margin-top: 22px; padding: 15px 17px; border-top: 1px solid var(--line); color: var(--ink-700); font-size: 12px; }.change-summary p { margin: 12px 0 0; }.diff-delete, .diff-add { display: inline-block; margin-right: 11px; padding: 4px 7px; border-radius: 4px; }.diff-delete { color: #a24c4c; background: #fde7e7; }.diff-add { color: #387946; background: #e6f5e2; }
</style>
