<script setup lang="ts">
import { computed, ref } from "vue";
import {
  ArrowDown,
  ArrowUp,
  Check,
  GripVertical,
  Link2,
  LockKeyhole,
  RotateCcw,
  Star,
  X,
} from "@lucide/vue";
import type { AlignmentDto, SegmentDto, WorkspaceMode } from "../domain/kernel-client";

const props = withDefaults(
  defineProps<{
    mode: WorkspaceMode;
    sourceSegments: SegmentDto[];
    targetSegments: SegmentDto[];
    alignments: AlignmentDto[];
    selectedAlignmentId: string;
  }>(),
  { selectedAlignmentId: "alignment-000004" },
);

const emit = defineEmits<{
  select: [alignmentId: string];
  saveSegment: [id: string, text: string];
  cancelEdit: [];
  move: [id: string, direction: "up" | "down"];
  resetOrder: [];
  status: [message: string];
}>();

const editingId = ref<string | null>(null);
const draft = ref("");
const draggedId = ref<string | null>(null);
const bookmarked = ref(new Set(["alignment-000003"]));

const rows = computed(() => {
  const sourceByOrder = [...props.sourceSegments].sort((a, b) => a.order - b.order);
  return sourceByOrder.map((source, index) => {
    const alignment = props.alignments.find((candidate) => candidate.sourceIds.includes(source.id));
    const targetIds = alignment?.targetIds ?? [];
    return {
      source,
      target: targetIds.map((id) => props.targetSegments.find((segment) => segment.id === id)).filter(Boolean) as SegmentDto[],
      alignmentId: alignment?.id ?? `unlinked-${source.id}`,
      index,
    };
  });
});

const beginEdit = (segment: SegmentDto) => {
  editingId.value = segment.id;
  draft.value = segment.text;
};

const saveEdit = (segment: SegmentDto) => {
  emit("saveSegment", segment.id, draft.value.trim() || segment.text);
  editingId.value = null;
};

const toggleBookmark = (alignmentId: string) => {
  const next = new Set(bookmarked.value);
  if (next.has(alignmentId)) next.delete(alignmentId);
  else next.add(alignmentId);
  bookmarked.value = next;
  emit("status", next.has(alignmentId) ? "已添加书签" : "已移除书签");
};

const dropSegment = (targetId: string) => {
  if (!draggedId.value || draggedId.value === targetId) return;
  emit("status", `已准备将 ${draggedId.value} 放置到 ${targetId} 之前`);
  draggedId.value = null;
};

const alignmentKind = (row: (typeof rows.value)[number]) => {
  if (!row.target.length) return "未对齐";
  return `${row.target.length === 1 ? "1" : row.target.length}:1`;
};
</script>

<template>
  <section class="workspace" :class="`workspace--${mode}`" aria-label="双语平行工作区">
    <div v-if="mode === 'order'" class="order-toolbar">
      <div class="order-label"><GripVertical :size="16" /> <span>排序工具：</span></div>
      <button class="tool-button tool-button--active" type="button" @click="emit('status', '拖动排序已启用')"><GripVertical :size="15" />拖动排序</button>
      <button class="tool-button" type="button" :disabled="!selectedAlignmentId" @click="emit('move', selectedAlignmentId, 'up')"><ArrowUp :size="15" />上移</button>
      <button class="tool-button" type="button" :disabled="!selectedAlignmentId" @click="emit('move', selectedAlignmentId, 'down')"><ArrowDown :size="15" />下移</button>
      <button class="tool-button" type="button" @click="emit('resetOrder')"><RotateCcw :size="15" />恢复顺序</button>
      <span class="order-hint">共 {{ rows.length }} 句 <i></i> 拖动句子调整顺序</span>
    </div>

    <template v-if="mode !== 'history'">
    <div class="column-headings">
      <h2>中文 <span>（原文）</span></h2>
      <div class="heading-divider" aria-hidden="true"></div>
      <h2>English <span>（译文）</span></h2>
    </div>

    <div class="parallel-list" :class="{ 'parallel-list--order': mode === 'order' }">
      <article
        v-for="row in rows"
        :key="row.alignmentId"
        class="alignment-row"
        :class="{
          'alignment-row--selected': row.alignmentId === selectedAlignmentId,
          'alignment-row--empty': row.target.length === 0,
          'alignment-row--dragging': draggedId === row.source.id,
        }"
        @click="emit('select', row.alignmentId)"
        @dragover.prevent
        @drop.prevent="dropSegment(row.source.id)"
      >
        <div v-if="mode === 'order'" class="drag-handle" draggable="true" @dragstart="draggedId = row.source.id" @dragend="draggedId = null">
          <GripVertical :size="18" />
          <span>{{ String(row.index + 1).padStart(2, '0') }}</span>
        </div>
        <div class="segment-cell segment-cell--source" :class="{ 'segment-cell--selected': row.alignmentId === selectedAlignmentId }">
          <span class="segment-id">{{ row.source.id.replace('src-', '') }}</span>
          <template v-if="mode === 'edit' && editingId === row.source.id">
            <div class="edit-card">
              <textarea v-model="draft" aria-label="编辑中文原文" @click.stop></textarea>
              <div class="edit-meta"><span>字数: {{ draft.length }}</span><span class="spell-status"><Check :size="15" />拼写检查</span><Link2 :size="17" /></div>
              <div class="edit-actions"><button type="button" @click.stop="emit('cancelEdit'); editingId = null"><X :size="14" />取消</button><button class="primary-button" type="button" @click.stop="saveEdit(row.source)"><Check :size="14" />保存</button></div>
            </div>
          </template>
          <button v-else class="segment-text" type="button" @dblclick.stop="beginEdit(row.source)">{{ row.source.text }}</button>
          <button class="bookmark-button" :class="{ 'bookmark-button--on': bookmarked.has(row.alignmentId) }" type="button" :aria-label="bookmarked.has(row.alignmentId) ? '移除书签' : '添加书签'" @click.stop="toggleBookmark(row.alignmentId)"><Star :size="18" :fill="bookmarked.has(row.alignmentId) ? 'currentColor' : 'none'" /></button>
        </div>

        <div class="link-cell" :class="{ 'link-cell--active': row.alignmentId === selectedAlignmentId }">
          <Link2 :size="23" />
          <span v-if="mode === 'order'" class="order-index">{{ String(row.index + 1).padStart(2, '0') }}</span>
        </div>

        <div class="segment-cell segment-cell--target" :class="{ 'segment-cell--selected': row.alignmentId === selectedAlignmentId }">
          <span class="segment-id">{{ row.target[0]?.id.replace('tgt-', '') ?? '—' }}</span>
          <template v-if="mode === 'edit' && editingId === row.target[0]?.id && row.target[0]">
            <div class="edit-card">
              <textarea v-model="draft" aria-label="编辑英文译文" @click.stop></textarea>
              <div class="edit-meta"><span>Words: {{ draft.trim().split(/\s+/).filter(Boolean).length }}</span><span class="spell-status"><Check :size="15" />Spell Check</span><Link2 :size="17" /></div>
              <div class="edit-actions edit-actions--right"><button type="button" @click.stop="emit('cancelEdit'); editingId = null"><X :size="14" />取消</button><button class="primary-button" type="button" @click.stop="saveEdit(row.target[0])"><Check :size="14" />保存</button></div>
            </div>
          </template>
          <button v-else class="segment-text segment-text--target" type="button" @dblclick.stop="row.target[0] && beginEdit(row.target[0])">{{ row.target[0]?.text ?? '此句尚未建立译文对齐' }}</button>
          <span class="alignment-kind" :class="{ 'alignment-kind--empty': !row.target.length }">{{ alignmentKind(row) }}</span>
        </div>

        <div v-if="mode === 'order' && draggedId === row.source.id" class="drop-slot"><ArrowDown :size="15" />将句子拖放到此处</div>
      </article>
    </div>

    <div v-if="mode === 'edit' && editingId === null" class="edit-tip"><LockKeyhole :size="15" />双击任意句子进入编辑；保存会形成一个可撤销的 Revision。</div>
    </template>
    <div v-else class="history-view">
      <header class="history-header"><div><span class="eyebrow">REVISION HISTORY</span><h2>历史记录</h2><p>比较工程版本，恢复操作会创建新的 Revision。</p></div><div class="history-actions"><button class="tool-button" type="button" @click="emit('status', '已选择两个版本进行比较')"><Link2 :size="15" />比较</button><button class="tool-button" type="button" @click="emit('status', '恢复版本需要接入 KernelClient')"><RotateCcw :size="15" />恢复此版本</button></div></header>
      <div class="history-layout"><aside class="revision-list"><button class="revision-card" type="button" :class="{ active: selectedAlignmentId === 'revision-r128' }" @click="selectedAlignmentId = 'revision-r128'"><strong>R128 <small>当前版本</small></strong><time>今天 15:42</time><b>Edit Segment</b><span>编辑了第 000104 段的英文译文，优化表达和结构。</span></button><button class="revision-card" type="button" @click="selectedAlignmentId = 'revision-r121'"><strong>R121</strong><time>今天 14:37</time><b>Merge Alignment</b><span>合并了相邻的 2 个对齐段落。</span></button><button class="revision-card" type="button" @click="selectedAlignmentId = 'revision-r117'"><strong>R117</strong><time>今天 11:09</time><b>Edit Segment</b><span>编辑了第 000103 段的中文原文。</span></button><button class="revision-card" type="button" @click="selectedAlignmentId = 'revision-r103'"><strong>R103</strong><time>昨天 18:22</time><b>Move Segment</b><span>将第 000095 段移动到第 000094 段之后。</span></button></aside><section class="revision-diff"><div class="diff-caption"><span>比较：<b>R117 → R128</b></span><span>段落：000104 <Link2 :size="15" /></span></div><div class="diff-grid"><div><small>R117（旧版本）</small><p class="diff-old">We steadily promote high-quality development, speed up the building of a new development pattern, and the quality and benefit of economic growth keep improving.</p></div><div><small>R128（当前版本）</small><p class="diff-new">We advanced high-quality development in depth, accelerated the building of a new development paradigm, and continuously improved the quality and efficiency of economic growth.</p></div></div><div class="change-summary"><b>变更摘要</b><p><span class="diff-delete">− 删除 1 行</span><span class="diff-add">＋ 添加 1 行</span></p><p>优化英文表达，使用更准确的词汇。</p></div></section></div>
    </div>
  </section>
</template>

<style scoped>
.history-view { display: flex; flex-direction: column; height: 100%; min-height: 0; background: #fbfcfb; }
.history-header { display: flex; align-items: flex-start; justify-content: space-between; padding: 24px 30px 20px; border-bottom: 1px solid var(--line); }
.history-header h2 { margin: 4px 0 4px; color: var(--ink-900); font-size: 21px; }.history-header p { margin: 0; color: var(--ink-500); font-size: 12px; }.history-actions { gap: 9px; }.history-actions .tool-button { height: 35px; }
.history-layout { display: grid; grid-template-columns: 290px minmax(0, 1fr); min-height: 0; flex: 1; }.revision-list { overflow: auto; padding: 18px 14px 18px 24px; border-right: 1px solid var(--line); }.revision-card { display: grid; grid-template-columns: 1fr auto; gap: 7px; width: 100%; margin-bottom: 11px; padding: 14px; border: 1px solid #e1e7e1; border-radius: 8px; background: #fff; text-align: left; cursor: pointer; }.revision-card:hover, .revision-card.active { border-color: #a7cfa9; background: #f5fbf3; }.revision-card strong { color: var(--green-900); font-size: 14px; }.revision-card strong small { margin-left: 4px; padding: 2px 5px; border-radius: 4px; color: var(--green-700); background: #e4f2e2; font-size: 10px; }.revision-card time { color: var(--ink-500); font-size: 10px; }.revision-card b { grid-column: 1 / -1; color: var(--ink-900); font-size: 13px; }.revision-card span { grid-column: 1 / -1; color: var(--ink-700); font-size: 11px; line-height: 1.5; }.revision-diff { overflow: auto; padding: 24px 28px; }.diff-caption { display: flex; justify-content: space-between; margin-bottom: 19px; color: var(--ink-700); font-size: 13px; }.diff-caption b { color: var(--ink-900); }.diff-caption span:last-child { display: flex; align-items: center; gap: 7px; }.diff-grid { display: grid; grid-template-columns: 1fr 1fr; border: 1px solid var(--line); border-radius: 7px; overflow: hidden; }.diff-grid > div { min-height: 210px; padding: 15px 18px; }.diff-grid > div + div { border-left: 1px solid var(--line); }.diff-grid small { color: var(--ink-500); font-size: 11px; }.diff-grid p { margin: 18px 0 0; padding: 12px; color: var(--ink-900); font-size: 14px; line-height: 1.65; }.diff-old { background: #fff0f0; text-decoration: line-through; text-decoration-color: #bc7272; }.diff-new { background: #eff9ec; }.change-summary { margin-top: 22px; padding: 15px 17px; border-top: 1px solid var(--line); color: var(--ink-700); font-size: 12px; }.change-summary b { color: var(--ink-900); }.change-summary p { margin: 12px 0 0; }.diff-delete, .diff-add { display: inline-block; margin-right: 11px; padding: 4px 7px; border-radius: 4px; }.diff-delete { color: #a24c4c; background: #fde7e7; }.diff-add { color: #387946; background: #e6f5e2; }
</style>
