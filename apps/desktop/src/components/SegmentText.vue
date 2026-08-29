<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { Check, GripVertical, Link2, Plus, X } from "@lucide/vue";
import type { EditSession } from "../composables/useViewModeController";
import type { DropEdge, RegisterOrderSegment } from "../composables/useOrderDragAndDrop";
import type { AlignmentGapEdge, LanguageSide, SegmentDto, WorkspaceMode } from "../domain/kernel-client";

const props = withDefaults(defineProps<{
  segment: SegmentDto;
  side: LanguageSide;
  alignmentId: string;
  mode: WorkspaceMode;
  writable: boolean;
  selected?: boolean;
  orderSelected?: boolean;
  editing?: boolean;
  editSession?: EditSession | null;
  showGapControls?: boolean;
  canInsertBefore?: boolean;
  canInsertAfter?: boolean;
  dragged?: boolean;
  dropEdge?: DropEdge | null;
  registerOrderSegment?: RegisterOrderSegment;
}>(), {
  selected: false,
  orderSelected: false,
  editing: false,
  editSession: null,
  showGapControls: false,
  canInsertBefore: false,
  canInsertAfter: false,
  dragged: false,
  dropEdge: null,
  registerOrderSegment: undefined,
});

const emit = defineEmits<{
  select: [event: MouseEvent];
  selectOrder: [];
  requestEdit: [];
  insertGap: [edge: AlignmentGapEdge];
  editDraft: [draft: string];
  commitEdit: [exitAfterSave: boolean];
  cancelEdit: [];
  escapeEdit: [];
}>();

const rootRef = ref<HTMLElement | null>(null);
let cleanupOrderDrag: (() => void) | null = null;
const orderDraggable = computed(() => props.side === "source" && props.mode === "order" && props.writable);
const orderLabel = computed(() => String(props.segment.order + 1).padStart(6, "0"));
const wordCount = computed(() => props.editSession?.draft.trim().split(/\s+/).filter(Boolean).length ?? 0);

watch(
  [rootRef, orderDraggable, () => props.segment.id, () => props.registerOrderSegment],
  ([element, enabled]) => {
    cleanupOrderDrag?.();
    cleanupOrderDrag = null;
    if (element instanceof HTMLElement && enabled && props.registerOrderSegment) {
      cleanupOrderDrag = props.registerOrderSegment(element, props.segment.id, props.side);
    }
  },
  { immediate: true, flush: "post" },
);
onBeforeUnmount(() => cleanupOrderDrag?.());
</script>

<template>
  <div
    ref="rootRef"
    class="segment-item"
    :class="[
      `segment-item--${side}`,
      {
        'segment-item--order': mode === 'order',
        'segment-item--selected': selected,
        'segment-item--order-selected': orderSelected,
        'segment-item--dragging': dragged,
        'segment-item--drop-before': dropEdge === 'before',
        'segment-item--drop-after': dropEdge === 'after',
      },
    ]"
    :data-segment-id="segment.id"
    :data-segment-side="side"
  >
    <button
      v-if="side === 'source' && mode === 'order'"
      class="segment-item__drag-handle"
      type="button"
      :disabled="!writable"
      data-order-drag-handle
      :aria-label="`选择并拖动中文 Segment ${segment.order + 1}`"
      title="拖动此中文 Segment；单击后也可用上移和下移按钮"
      @click.stop="emit('selectOrder')"
    ><GripVertical :size="18" /><span>{{ segment.order + 1 }}</span></button>
    <button
      v-else-if="mode === 'order'"
      class="segment-item__order-number"
      type="button"
      :aria-label="`选择英文 Segment ${segment.order + 1}`"
      @click.stop="emit('selectOrder')"
    >{{ segment.order + 1 }}</button>
    <button
      v-else
      class="segment-item__id"
      type="button"
      :title="segment.id"
      :aria-label="`选择 ${side === 'source' ? '中文' : '英文'} Segment ${segment.order + 1}`"
      @click.stop="emit('select', $event)"
    >{{ orderLabel }}</button>

    <div v-if="editing && editSession" class="segment-item__editor" @click.stop>
      <textarea
        :value="editSession.draft"
        autofocus
        :aria-label="side === 'source' ? '编辑中文原文' : '编辑英文译文'"
        @input="emit('editDraft', ($event.target as HTMLTextAreaElement).value)"
        @keydown.esc.stop.prevent="emit('escapeEdit')"
        @keydown.ctrl.enter.stop.prevent="emit('commitEdit', true)"
        @keydown.meta.enter.stop.prevent="emit('commitEdit', true)"
      ></textarea>
      <div class="segment-item__edit-meta">
        <span>{{ side === 'source' ? `字数: ${editSession.draft.length}` : `Words: ${wordCount}` }}</span>
        <span class="segment-item__save-state"><Check :size="15" />{{ editSession.status === 'saving' ? '保存中' : editSession.status === 'error' ? '保存失败' : editSession.status === 'dirty' ? '待保存' : '已保存' }}</span>
        <Link2 :size="17" />
      </div>
      <p v-if="editSession.error" class="segment-item__edit-error">{{ editSession.error }}</p>
      <div class="segment-item__edit-actions">
        <button type="button" @click="emit('cancelEdit')"><X :size="14" />放弃并退出</button>
        <button class="primary-button" type="button" :disabled="editSession.status === 'saving'" @click="emit('commitEdit', true)"><Check :size="14" />保存并退出</button>
      </div>
    </div>
    <button
      v-else
      class="segment-item__text"
      type="button"
      @click.stop="emit('select', $event)"
      @dblclick.stop="emit('requestEdit')"
    >{{ segment.text }}</button>

    <button
      v-if="showGapControls"
      class="segment-item__gap segment-item__gap--before"
      type="button"
      :disabled="!canInsertBefore"
      :aria-label="`在所选${side === 'source' ? '中文' : '英文'}句段上方插入空位`"
      title="在上方插入空位，并自动重建后续 1:1 对齐"
      @click.stop="emit('insertGap', 'before')"
    ><Plus :size="15" :stroke-width="2.6" /></button>
    <button
      v-if="showGapControls"
      class="segment-item__gap segment-item__gap--after"
      type="button"
      :disabled="!canInsertAfter"
      :aria-label="`在所选${side === 'source' ? '中文' : '英文'}句段下方插入空位`"
      title="在下方插入空位，并自动重建后续 1:1 对齐"
      @click.stop="emit('insertGap', 'after')"
    ><Plus :size="15" :stroke-width="2.6" /></button>
  </div>
</template>

<style scoped>
.segment-item { position: relative; display: grid; grid-template-columns: 72px minmax(0, 1fr); min-width: 0; min-height: 76px; align-items: stretch; isolation: isolate; }
.segment-item + .segment-item { border-top: 1px dashed var(--line); }
.segment-item--selected { background: var(--surface-green-selected); box-shadow: inset 3px 0 var(--green-700); }
.segment-item--target.segment-item--selected { box-shadow: inset -3px 0 #eebf4f; }
.segment-item--order-selected { z-index: 12; background: var(--surface-green-selected); box-shadow: inset 0 0 0 2px var(--green-700); }
.segment-item--dragging { z-index: 34; opacity: .38; }
.segment-item--drop-before, .segment-item--drop-after { z-index: 28; }
.segment-item--drop-before::before, .segment-item--drop-after::after { position: absolute; z-index: 4; right: 8px; left: 8px; height: 3px; border-radius: 3px; background: var(--green-700); box-shadow: 0 0 0 3px rgb(55 127 66 / 14%); content: ""; }
.segment-item--drop-before::before { top: -2px; }
.segment-item--drop-after::after { bottom: -2px; }
.segment-item__id, .segment-item__order-number, .segment-item__drag-handle { min-width: 0; padding: 0 8px; border: 0; border-right: 1px solid var(--line); color: #849087; background: transparent; font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: 12px; cursor: pointer; }
.segment-item--target :is(.segment-item__id, .segment-item__order-number) { border-right: 1px solid var(--line); }
.segment-item__drag-handle { position: relative; z-index: 2; display: flex; align-items: center; justify-content: center; gap: 5px; color: #68756c; background: var(--surface-muted); cursor: grab; touch-action: none; }
.segment-item__drag-handle:active { cursor: grabbing; }
.segment-item__drag-handle:disabled { cursor: not-allowed; opacity: .5; }
.segment-item__order-number { color: var(--ink-500); background: var(--surface-muted); font-size: 14px; }
.segment-item__text { display: block; width: 100%; min-width: 0; padding: 18px 40px 18px 26px; border: 0; color: var(--ink-900); background: transparent; font-size: 16px; line-height: 1.62; text-align: left; cursor: pointer; }
.segment-item--target .segment-item__text { padding-right: 54px; font-size: 15.5px; }
.segment-item__editor { min-width: 0; margin: 10px 16px; overflow: hidden; border: 1px solid #b9c6bc; border-radius: 4px; background: var(--surface-raised); box-shadow: var(--shadow-soft); }
.segment-item__editor textarea { display: block; width: 100%; min-height: 100px; resize: vertical; padding: 14px 15px; border: 0; border-bottom: 1px solid var(--line); color: var(--ink-900); background: transparent; font-size: 16px; line-height: 1.52; }
.segment-item__edit-meta { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 9px 12px; color: var(--ink-500); font-size: 12px; }
.segment-item__save-state { display: inline-flex; align-items: center; gap: 5px; color: var(--green-700); }
.segment-item__edit-error { margin: 0; padding: 6px 12px; color: #a24c4c; background: var(--surface-danger-soft); font-size: 11px; }
.segment-item__edit-actions { display: flex; justify-content: flex-end; gap: 8px; padding: 0 12px 11px; }
.segment-item__edit-actions button { display: inline-flex; align-items: center; gap: 5px; padding: 7px 13px; border: 1px solid var(--line); border-radius: 5px; background: var(--surface-raised); cursor: pointer; }
.segment-item__edit-actions .primary-button { border-color: var(--green-700); color: #fff; background: var(--green-700); }
.segment-item__gap { position: absolute; z-index: 40; left: 50%; display: grid; width: 34px; height: 24px; padding: 0; transform: translateX(-50%); place-items: center; border: 1px solid #8cbe92; border-radius: 999px; color: var(--green-900); background: var(--surface-raised); box-shadow: 0 2px 7px rgb(31 53 34 / 14%); cursor: pointer; }
.segment-item__gap--before { top: -12px; }
.segment-item__gap--after { bottom: -12px; }
.segment-item__gap:hover:not(:disabled) { border-color: var(--green-700); background: var(--surface-green-soft); transform: translateX(-50%) scale(1.06); }
.segment-item__gap:disabled { cursor: not-allowed; opacity: .35; }
@media (max-width: 1280px) { .segment-item { grid-template-columns: 62px minmax(0, 1fr); } .segment-item__text { padding-right: 28px; padding-left: 18px; } }
</style>
