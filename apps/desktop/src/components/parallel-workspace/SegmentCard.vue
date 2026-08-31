<script setup lang="ts">
import { formatError } from '../../i18n/kernel-messages';
import { t } from '../../i18n';
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { Check, GripVertical, Link2, MessageSquareText, Plus, Star, X } from "@lucide/vue";
import type { EditSession } from "../../composables/useViewModeController";
import type { DropEdge, RegisterOrderSegment } from "../../composables/useOrderDragAndDrop";
import type { AlignmentGapEdge, LanguageSide, SegmentDto, WorkspaceMode } from "../../domain/kernel-client";

const props = withDefaults(defineProps<{
  segment: SegmentDto;
  side: LanguageSide;
  alignmentId: string;
  mode: WorkspaceMode;
  writable: boolean;
  reorderEnabled?: boolean;
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
  fragmented?: boolean;
  bookmarked?: boolean;
  annotated?: boolean;
  jumpHighlighted?: boolean;
}>(), {
  reorderEnabled: false,
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
  fragmented: false,
  bookmarked: false,
  annotated: false,
  jumpHighlighted: false,
});

const emit = defineEmits<{
  select: [event: MouseEvent];
  selectOrder: [];
  requestEdit: [];
  insertGap: [edge: AlignmentGapEdge];
  bookmark: [];
  annotation: [];
  editDraft: [draft: string];
  commitEdit: [exitAfterSave: boolean];
  cancelEdit: [];
  escapeEdit: [];
}>();

const rootRef = ref<HTMLElement | null>(null);
let cleanupOrderDrag: (() => void) | null = null;
const orderDraggable = computed(() => props.reorderEnabled && props.writable);
const orderLabel = computed(() => String(props.segment.order + 1));
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
    class="segment-card"
    :class="[
      `segment-card--${side}`,
      {
        'segment-card--reorder': reorderEnabled,
        'segment-card--selected': selected,
        'segment-card--order-selected': orderSelected,
        'segment-card--editing': editing && editSession,
        'segment-card--dragging': dragged,
        'segment-card--drop-before': dropEdge === 'before',
        'segment-card--drop-after': dropEdge === 'after',
        'segment-card--jump-highlighted': jumpHighlighted,
      },
    ]"
    :data-segment-id="segment.id"
    :data-segment-side="side"
  >
    <button
      v-if="reorderEnabled"
      class="segment-card__index segment-card__drag-handle"
      type="button"
      :disabled="!writable"
      data-order-drag-handle
      :aria-label="t('dragSegmentLabel', { p0: side === 'source' ? t('sourceSide') : t('targetSide'), p1: segment.order + 1 })"
      :title="t('dragSegmentHint', { p0: side === 'source' ? t('sourceSide') : t('targetSide') })"
      @click.stop="emit('selectOrder')"
    ><GripVertical :size="17" /><span>{{ orderLabel }}</span></button>
    <button
      v-else
      class="segment-card__index"
      type="button"
      :title="segment.id"
      :aria-label="t('selectSegmentLabel', { p0: side === 'source' ? t('sourceSide') : t('targetSide'), p1: segment.order + 1 })"
      @click.stop="emit('select', $event)"
    >{{ orderLabel }}</button>

    <div v-if="!editing" class="segment-card__context-actions">
      <button
        class="segment-card__context-action"
        :class="{ 'segment-card__context-action--highlighted': annotated }"
        type="button"
        :aria-label="annotated ? t('openExistingAnnotations') : t('openSegmentAnnotations')"
        :title="annotated ? t('hasAnnotations') : t('annotation')"
        @click.stop="emit('annotation')"
      ><MessageSquareText :size="17" /></button>
      <button
        class="segment-card__context-action"
        :class="{ 'segment-card__context-action--highlighted': bookmarked }"
        type="button"
        :disabled="!writable"
        :aria-label="bookmarked ? t('removeBookmark') : t('addBookmark')"
        :title="bookmarked ? t('removeBookmark') : t('addBookmark')"
        @click.stop="emit('bookmark')"
      ><Star :size="17" :fill="bookmarked ? 'currentColor' : 'none'" /></button>
    </div>

    <div v-if="editing && editSession" class="segment-card__editor" @click.stop>
      <textarea
        :value="editSession.draft"
        autofocus
        :aria-label="side === 'source' ? t('editSource') : t('editTarget')"
        @input="emit('editDraft', ($event.target as HTMLTextAreaElement).value)"
        @keydown.esc.stop.prevent="emit('escapeEdit')"
        @keydown.ctrl.enter.stop.prevent="emit('commitEdit', true)"
        @keydown.meta.enter.stop.prevent="emit('commitEdit', true)"
      ></textarea>
      <div class="segment-card__edit-meta">
        <span>{{ side === 'source' ? t('characterCount', { p0: editSession.draft.length }) : t('wordCount', { p0: wordCount }) }}</span>
        <span class="segment-card__save-state"><Check :size="15" />{{ editSession.status === 'saving' ? t('saving') : editSession.status === 'error' ? t('saveFailed') : editSession.status === 'dirty' ? t('savePending') : t('saved') }}</span>
        <Link2 :size="17" />
      </div>
      <p v-if="editSession.error" class="segment-card__edit-error">{{ formatError(editSession.error) }}</p>
      <div class="segment-card__edit-actions">
        <button type="button" @click="emit('cancelEdit')"><X :size="14" />{{ t('discardExit') }}</button>
        <button class="primary-button" type="button" :disabled="editSession.status === 'saving'" @click="emit('commitEdit', true)"><Check :size="14" />{{ t('saveExit') }}</button>
      </div>
    </div>
    <button
      v-else
      class="segment-card__text"
      type="button"
      @click.stop="emit('select', $event)"
      @dblclick.stop="emit('requestEdit')"
    >{{ segment.text }}</button>

    <span v-if="fragmented" class="segment-card__fragment-warning">{{ t('fragmentWarning') }}</span>

    <button
      v-if="showGapControls"
      class="segment-card__gap segment-card__gap--before"
      type="button"
      :disabled="!canInsertBefore"
      :aria-label="t('gapAboveLabel', { p0: side === 'source' ? t('sourceSide') : t('targetSide') })"
      :title="t('gapAboveHint')"
      @click.stop="emit('insertGap', 'before')"
    ><Plus :size="15" :stroke-width="2.6" /></button>
    <button
      v-if="showGapControls"
      class="segment-card__gap segment-card__gap--after"
      type="button"
      :disabled="!canInsertAfter"
      :aria-label="t('gapBelowLabel', { p0: side === 'source' ? t('sourceSide') : t('targetSide') })"
      :title="t('gapBelowHint')"
      @click.stop="emit('insertGap', 'after')"
    ><Plus :size="15" :stroke-width="2.6" /></button>
  </div>
</template>

<style scoped>
.segment-card { --segment-control-width: 96px; position: relative; display: grid; grid-template-columns: var(--segment-control-width) minmax(0, 1fr); min-width: 0; min-height: 76px; align-items: stretch; isolation: isolate; }
.segment-card--source { grid-template-columns: minmax(0, 1fr) var(--segment-control-width); }
.segment-card + .segment-card { border-top: 1px solid var(--line); }
.segment-card--selected, .segment-card--order-selected { z-index: 12; }
.segment-card--selected::after, .segment-card--order-selected::after { position: absolute; z-index: 24; inset: 0; border: 2px solid var(--green-700); content: ""; pointer-events: none; }
.segment-card--jump-highlighted { z-index: 26; background: var(--surface-green-selected); }
.segment-card--jump-highlighted::after { position: absolute; z-index: 35; inset: 3px; border: 2px solid var(--green-700); border-radius: 5px; box-shadow: 0 0 0 4px rgb(47 129 67 / 12%); content: ""; pointer-events: none; animation: bookmark-jump-highlight 2.4s ease-out; }
@keyframes bookmark-jump-highlight { 0%, 58% { opacity: 1; } 100% { opacity: 0; } }
.segment-card--editing { z-index: 20; }
.segment-card--dragging { z-index: 34; opacity: .4; }
.segment-card--drop-before, .segment-card--drop-after { z-index: 28; }
.segment-card--drop-before::before, .segment-card--drop-after::after { position: absolute; z-index: 16; right: 0; left: 0; height: 3px; background: var(--green-700); box-shadow: 0 0 0 3px rgb(55 127 66 / 14%); content: ""; }
.segment-card--drop-before::before { top: -2px; }
.segment-card--drop-after::after { bottom: -2px; }
.segment-card__index { display: flex; min-width: 0; grid-column: 1; grid-row: 1; align-items: center; justify-content: center; gap: 5px; padding: 28px 8px 0; border: 0; border-right: 1px solid var(--line); color: var(--ink-500); background: var(--surface-muted); font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: 13px; cursor: pointer; }
.segment-card--source .segment-card__index { grid-column: 2; border-right: 0; border-left: 1px solid var(--line); }
.segment-card__drag-handle { position: relative; z-index: 2; color: #68756c; cursor: grab; touch-action: none; }
.segment-card__drag-handle:active { cursor: grabbing; }
.segment-card__drag-handle:disabled { cursor: not-allowed; opacity: .5; }
.segment-card__order-number { font-size: 14px; }
.segment-card__text { display: block; width: 100%; min-width: 0; grid-column: 2; grid-row: 1; padding: 18px 26px; border: 0; color: var(--ink-900); background: transparent; font-size: calc(16px * var(--reading-font-scale)); line-height: 1.62; text-align: left; cursor: pointer; }
.segment-card--source .segment-card__text { grid-column: 1; }
.segment-card--target .segment-card__text { font-size: calc(15.5px * var(--reading-font-scale)); }
.segment-card__context-actions { position: absolute; z-index: 30; top: 7px; left: 34px; display: flex; align-items: center; gap: 1px; }
.segment-card--source .segment-card__context-actions { right: 34px; left: auto; }
.segment-card__context-action { display: grid; width: 25px; height: 25px; place-items: center; padding: 0; border: 0; border-radius: 6px; color: #aeb8b0; background: transparent; cursor: pointer; opacity: 0; transition: color 140ms ease, background-color 140ms ease, opacity 140ms ease; }
.segment-card:hover .segment-card__context-action, .segment-card--selected .segment-card__context-action, .segment-card--order-selected .segment-card__context-action, .segment-card__context-action:focus-visible { opacity: 1; }
.segment-card__context-action:hover:not(:disabled) { color: var(--green-900); background: var(--surface-hover); }
.segment-card__context-action--highlighted { color: #e5a300; opacity: 1; }
.segment-card__context-action--highlighted:hover:not(:disabled) { color: #d38e00; background: var(--surface-warm-soft); }
.segment-card__fragment-warning { position: absolute; z-index: 4; right: calc(var(--segment-control-width) + 10px); bottom: 8px; padding: 2px 6px; border: 1px solid #9cc9a2; border-radius: 999px; color: var(--green-900); background: var(--surface-green-soft); font-size: 9px; font-weight: 700; pointer-events: none; }
.segment-card--target .segment-card__fragment-warning { right: auto; left: calc(var(--segment-control-width) + 10px); }
.segment-card__editor { min-width: 0; grid-column: 2; grid-row: 1; margin: 10px 16px; overflow: hidden; border: 1px solid #b9c6bc; border-radius: 4px; background: var(--surface-raised); box-shadow: var(--shadow-soft); }
.segment-card--source .segment-card__editor { grid-column: 1; }
.segment-card__editor textarea { display: block; width: 100%; min-height: 100px; resize: vertical; padding: 14px 15px; border: 0; border-bottom: 1px solid var(--line); color: var(--ink-900); background: transparent; font-size: calc(16px * var(--reading-font-scale)); line-height: 1.52; }
.segment-card__edit-meta { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 9px 12px; color: var(--ink-500); font-size: 12px; }
.segment-card__save-state { display: inline-flex; align-items: center; gap: 5px; color: var(--green-700); }
.segment-card__edit-error { margin: 0; padding: 6px 12px; color: #a24c4c; background: var(--surface-danger-soft); font-size: 11px; }
.segment-card__edit-actions { display: flex; justify-content: flex-end; gap: 8px; padding: 0 12px 11px; }
.segment-card__edit-actions button { display: inline-flex; align-items: center; gap: 5px; padding: 7px 13px; border: 1px solid var(--line); border-radius: 5px; background: var(--surface-raised); cursor: pointer; }
.segment-card__edit-actions .primary-button { border-color: var(--green-700); color: #fff; background: var(--green-700); }
.segment-card__gap { position: absolute; z-index: 40; left: 50%; display: grid; width: 34px; height: 24px; padding: 0; transform: translateX(-50%); place-items: center; border: 1px solid #8cbe92; border-radius: 999px; color: var(--green-900); background: var(--surface-raised); box-shadow: 0 2px 7px rgb(31 53 34 / 14%); cursor: pointer; }
.segment-card__gap--before { top: -12px; }
.segment-card__gap--after { bottom: -12px; }
.segment-card__gap:hover:not(:disabled) { border-color: var(--green-700); background: var(--surface-green-soft); transform: translateX(-50%) scale(1.06); }
.segment-card__gap:disabled { cursor: not-allowed; opacity: .35; }
@media (max-width: 1280px) { .segment-card { --segment-control-width: 86px; grid-template-columns: var(--segment-control-width) minmax(0, 1fr); } .segment-card--source { grid-template-columns: minmax(0, 1fr) var(--segment-control-width); } .segment-card__context-actions { left: 30px; } .segment-card--source .segment-card__context-actions { right: 30px; left: auto; } .segment-card__text { padding-right: 18px; padding-left: 18px; } }
</style>
