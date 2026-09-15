<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { Check, GripVertical, Link2, MessageSquareText, Plus, Star, X } from "@lucide/vue";
import type { EditSession } from "../../composables/useViewModeController";
import type { DropEdge, RegisterOrderSegment } from "../../composables/useOrderDragAndDrop";
import type { AlignmentGapEdge, LanguageSide, SegmentDto, WorkspaceMode } from "../../domain/kernel-client";
import { t } from "../../i18n";
import { formatError } from "../../i18n/kernel-messages";

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
const sideLabel = computed(() => t(props.side === "source" ? "puiSideSource" : "puiSideTarget"));
const editStatusLabel = computed(() => t(props.editSession?.status === "saving"
  ? "puiSaving"
  : props.editSession?.status === "error"
    ? "puiSaveFailed"
    : props.editSession?.status === "dirty"
      ? "puiSavePending"
      : "puiSaved"));

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
      :aria-label="t('puiSelectDragSegment', { p0: sideLabel, p1: segment.order + 1 })"
      :title="t('puiDragSegmentTitle', { p0: sideLabel })"
      @click.stop="emit('selectOrder')"
    ><GripVertical :size="17" /><span>{{ orderLabel }}</span></button>
    <button
      v-else
      class="segment-card__index"
      type="button"
      :title="segment.id"
      :aria-label="t('puiSelectSegment', { p0: sideLabel, p1: segment.order + 1 })"
      @click.stop="emit('select', $event)"
    >{{ orderLabel }}</button>

    <div v-if="!editing" class="segment-card__context-actions">
      <button
        class="segment-card__context-action"
        :class="{ 'segment-card__context-action--highlighted': annotated }"
        type="button"
        :aria-label="t(annotated ? 'puiOpenAnnotated' : 'puiOpenAnnotation')"
        :title="t(annotated ? 'puiHasAnnotation' : 'puiAnnotation')"
        @click.stop="emit('annotation')"
      ><MessageSquareText :size="17" /></button>
      <button
        class="segment-card__context-action"
        :class="{ 'segment-card__context-action--highlighted': bookmarked }"
        type="button"
        :disabled="!writable"
        :aria-label="t(bookmarked ? 'puiRemoveBookmark' : 'puiAddBookmark')"
        :title="t(bookmarked ? 'puiRemoveBookmark' : 'puiAddBookmark')"
        @click.stop="emit('bookmark')"
      ><Star :size="17" :fill="bookmarked ? 'currentColor' : 'none'" /></button>
    </div>

    <div v-if="editing && editSession" class="segment-card__editor" @click.stop>
      <textarea
        data-segment-content
        :value="editSession.draft"
        autofocus
        :aria-label="t(side === 'source' ? 'puiEditSource' : 'puiEditTarget')"
        @input="emit('editDraft', ($event.target as HTMLTextAreaElement).value)"
        @keydown.esc.stop.prevent="emit('escapeEdit')"
        @keydown.ctrl.enter.stop.prevent="emit('commitEdit', true)"
        @keydown.meta.enter.stop.prevent="emit('commitEdit', true)"
      ></textarea>
      <div class="segment-card__edit-meta">
        <span>{{ t(side === 'source' ? 'puiCharacterCount' : 'puiWordCount', { p0: side === 'source' ? editSession.draft.length : wordCount }) }}</span>
        <span class="segment-card__save-state"><Check :size="15" />{{ editStatusLabel }}</span>
        <Link2 :size="17" />
      </div>
      <p v-if="editSession.error" class="segment-card__edit-error">{{ formatError(editSession.error) }}</p>
      <div class="segment-card__edit-actions">
        <button type="button" @click="emit('cancelEdit')"><X :size="14" />{{ t("puiDiscardExit") }}</button>
        <button class="primary-button" type="button" :disabled="editSession.status === 'saving'" @click="emit('commitEdit', true)"><Check :size="14" />{{ t("puiSaveExit") }}</button>
      </div>
    </div>
    <button
      v-else
      class="segment-card__text"
      data-segment-content
      type="button"
      @click.stop="emit('select', $event)"
      @dblclick.stop="segment.loaded !== false && emit('requestEdit')"
     :aria-busy="segment.loaded === false">{{ segment.loaded === false ? t("puiLoading") : segment.text }}</button>

    <span v-if="fragmented" class="segment-card__fragment-warning">{{ t("puiFragmented") }}</span>

    <button
      v-if="showGapControls"
      class="segment-card__gap segment-card__gap--before"
      type="button"
      :disabled="!canInsertBefore"
      :aria-label="t('puiInsertGapAria', { p0: sideLabel, p1: t('puiAbove') })"
      :title="t('puiInsertGapTitle', { p0: t('puiAbove') })"
      @click.stop="emit('insertGap', 'before')"
    ><Plus :size="15" :stroke-width="2.6" /></button>
    <button
      v-if="showGapControls"
      class="segment-card__gap segment-card__gap--after"
      type="button"
      :disabled="!canInsertAfter"
      :aria-label="t('puiInsertGapAria', { p0: sideLabel, p1: t('puiBelow') })"
      :title="t('puiInsertGapTitle', { p0: t('puiBelow') })"
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
.segment-card--jump-highlighted { z-index: 26; @apply bg-green-selected; }
.segment-card--jump-highlighted::after { position: absolute; z-index: 35; inset: 3px; border: 2px solid var(--green-700); border-radius: 5px; box-shadow: 0 0 0 4px rgb(47 129 67 / 12%); content: ""; pointer-events: none; animation: bookmark-jump-highlight 2.4s ease-out; }
@keyframes bookmark-jump-highlight { 0%, 58% { opacity: 1; } 100% { opacity: 0; } }
.segment-card--editing { z-index: 20; }
.segment-card--dragging { z-index: 34; opacity: .4; }
.segment-card--drop-before, .segment-card--drop-after { z-index: 28; }
.segment-card--drop-before::before, .segment-card--drop-after::after { position: absolute; z-index: 16; right: 0; left: 0; height: 3px; @apply bg-accent; box-shadow: 0 0 0 3px rgb(55 127 66 / 14%); content: ""; }
.segment-card--drop-before::before { top: -2px; }
.segment-card--drop-after::after { bottom: -2px; }
.segment-card__index { display: flex; min-width: 0; grid-column: 1; grid-row: 1; align-items: center; justify-content: center; gap: 5px; padding: 28px 8px 0; border: 0; border-right: 1px solid var(--line); @apply text-ink-500 bg-muted; font-family: var(--jm-font-mono); font-size: var(--jm-font-size-callout); cursor: pointer; line-height: var(--jm-line-height-callout); }
.segment-card--source .segment-card__index { grid-column: 2; border-right: 0; border-left: 1px solid var(--line); }
.segment-card__drag-handle { position: relative; z-index: 2; color: var(--ink-700, #68756c); cursor: grab; touch-action: none; }
.segment-card__drag-handle:active { cursor: grabbing; }
.segment-card__drag-handle:disabled { cursor: not-allowed; opacity: .5; }
.segment-card__order-number { font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }
.segment-card__text { display: block; width: 100%; min-width: 0; grid-column: 2; grid-row: 1; padding: 18px 26px; border: 0; @apply text-ink-900; background: transparent; font-size: calc(var(--jm-reading-source-size) * var(--reading-font-scale)); line-height: var(--jm-reading-line-height); text-align: left; cursor: pointer; font-weight: var(--jm-font-weight-regular); }
.segment-card--source .segment-card__text { grid-column: 1; }
.segment-card--target .segment-card__text { font-size: calc(var(--jm-reading-target-size) * var(--reading-font-scale)); }
.segment-card__context-actions { position: absolute; z-index: 30; top: 7px; left: 34px; display: flex; align-items: center; gap: 1px; }
.segment-card--source .segment-card__context-actions { right: 34px; left: auto; }
.segment-card__context-action { display: grid; width: 25px; height: 25px; place-items: center; padding: 0; border: 0; border-radius: 6px; color: var(--ink-700, #aeb8b0); background: transparent; cursor: pointer; opacity: 0; transition: color 140ms ease, background-color 140ms ease, opacity 140ms ease; }
.segment-card:hover .segment-card__context-action, .segment-card--selected .segment-card__context-action, .segment-card--order-selected .segment-card__context-action, .segment-card__context-action:focus-visible { opacity: 1; }
.segment-card__context-action:hover:not(:disabled) { @apply text-accent-strong bg-hover; }
.segment-card__context-action--highlighted { color: var(--text-warning, #e5a300); opacity: 1; }
.segment-card__context-action--highlighted:hover:not(:disabled) { color: var(--text-warning, #d38e00); @apply bg-warm-soft; }
.segment-card__fragment-warning { position: absolute; z-index: 4; right: calc(var(--segment-control-width) + 10px); bottom: 8px; padding: 2px 6px; border: 1px solid #9cc9a2; border-radius: 999px; @apply text-accent-strong bg-green-soft; font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-semibold); pointer-events: none; line-height: var(--jm-line-height-subheadline); }
.segment-card--target .segment-card__fragment-warning { right: auto; left: calc(var(--segment-control-width) + 10px); }
.segment-card__editor { min-width: 0; grid-column: 2; grid-row: 1; margin: 10px 16px; overflow: hidden; border: 1px solid #b9c6bc; border-radius: 4px; @apply bg-raised; box-shadow: var(--shadow-soft); }
.segment-card--source .segment-card__editor { grid-column: 1; }
.segment-card__editor textarea { display: block; width: 100%; min-height: 100px; resize: vertical; padding: 14px 15px; border: 0; border-bottom: 1px solid var(--line); @apply text-ink-900; background: transparent; font-size: calc(var(--jm-reading-source-size) * var(--reading-font-scale)); line-height: var(--jm-editing-line-height); font-weight: var(--jm-font-weight-regular); }
.segment-card__edit-meta { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 9px 12px; @apply text-ink-500; font-size: var(--jm-font-size-callout); line-height: var(--jm-line-height-callout); }
.segment-card__save-state { display: inline-flex; align-items: center; gap: 5px; @apply text-accent; }
.segment-card__edit-error { margin: 0; padding: 6px 12px; color: var(--text-danger, #a24c4c); @apply bg-danger-soft; font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }
.segment-card__edit-actions { display: flex; justify-content: flex-end; gap: 8px; padding: 0 12px 11px; }
.segment-card__edit-actions button { display: inline-flex; align-items: center; gap: 5px; padding: 7px 13px; border: 1px solid var(--line); border-radius: 5px; @apply bg-raised; cursor: pointer; }
.segment-card__edit-actions .primary-button { @apply border-accent; color: #fff; @apply bg-accent-solid; }
.segment-card__gap { position: absolute; z-index: 40; left: 50%; display: grid; width: 34px; height: 24px; padding: 0; transform: translateX(-50%); place-items: center; border: 1px solid #8cbe92; border-radius: 999px; @apply text-accent-strong bg-raised; box-shadow: 0 2px 7px rgb(31 53 34 / 14%); cursor: pointer; }
.segment-card__gap--before { top: -12px; }
.segment-card__gap--after { bottom: -12px; }
.segment-card__gap:hover:not(:disabled) { @apply border-accent bg-green-soft; transform: translateX(-50%) scale(1.06); }
.segment-card__gap:disabled { cursor: not-allowed; opacity: .35; }
@media (max-width: 1280px) { .segment-card { --segment-control-width: 86px; grid-template-columns: var(--segment-control-width) minmax(0, 1fr); } .segment-card--source { grid-template-columns: minmax(0, 1fr) var(--segment-control-width); } .segment-card__context-actions { left: 30px; } .segment-card--source .segment-card__context-actions { right: 30px; left: auto; } .segment-card__text { padding-right: 18px; padding-left: 18px; } }
</style>
