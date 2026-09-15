<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import type { EditSession } from "../../composables/useViewModeController";
import type { DropEdge, RegisterOrderSegment } from "../../composables/useOrderDragAndDrop";
import type { AlignmentGapEdge, LanguageSide, WorkspaceMode } from "../../domain/kernel-client";
import type { AlignmentBlockView, OrderSelection } from "../../domain/workspace-projection";
import SegmentCard from "./SegmentCard.vue";
import { t } from "../../i18n";

const props = withDefaults(defineProps<{
  block: AlignmentBlockView;
  side: LanguageSide;
  mode: WorkspaceMode;
  writable: boolean;
  reorderEnabled?: boolean;
  alignmentActive?: boolean;
  alignmentOperationSelected?: boolean;
  orderSelection?: OrderSelection | null;
  selectedSegmentIds: Set<string>;
  editSession?: EditSession | null;
  bookmarkedSegmentIds: Set<string>;
  annotatedSegmentIds: Set<string>;
  highlighted?: boolean;
  jumpHighlightSegmentId?: string;
  draggedId?: string | null;
  dropTargetId?: string | null;
  dropEdge?: DropEdge | null;
  registerOrderSegment: RegisterOrderSegment;
  canInsertGap: (side: LanguageSide, segmentId: string, edge: AlignmentGapEdge) => boolean;
}>(), {
  reorderEnabled: false,
  alignmentActive: false,
  alignmentOperationSelected: false,
  orderSelection: null,
  editSession: null,
  highlighted: false,
  jumpHighlightSegmentId: "",
  draggedId: null,
  dropTargetId: null,
  dropEdge: null,
});

const emit = defineEmits<{
  resize: [height: number];
  selectAlignment: [event: MouseEvent];
  selectSegment: [segmentId: string, event: MouseEvent];
  selectOrderSegment: [segmentId: string];
  requestEdit: [segmentId: string];
  insertGap: [segmentId: string, edge: AlignmentGapEdge];
  bookmark: [segmentId: string];
  annotation: [segmentId: string];
  editDraft: [draft: string];
  commitEdit: [exitAfterSave: boolean];
  cancelEdit: [];
  escapeEdit: [];
}>();

const rootRef = ref<HTMLElement | null>(null);
const segments = computed(() => props.side === "source" ? props.block.sourceSegments : props.block.targetSegments);
const fragmented = computed(() => props.side === "source" ? props.block.sourceFragmented : props.block.targetFragmented);
const isOrderSelected = (segmentId: string) => props.orderSelection?.side === props.side
  && props.orderSelection.segmentId === segmentId;
let resizeObserver: ResizeObserver | null = null;

const reportSize = () => {
  const element = rootRef.value;
  const rect = element?.getBoundingClientRect();
  const scale = element && rect && element.offsetWidth ? rect.width / element.offsetWidth : 1;
  const height = rect ? rect.height / scale : 0;
  if (height > 0) emit("resize", height);
};
watch(rootRef, async (element) => {
  resizeObserver?.disconnect();
  resizeObserver = null;
  if (!element) return;
  await nextTick();
  reportSize();
  resizeObserver = new ResizeObserver(entries => {
    const height = entries[0]?.borderBoxSize[0]?.blockSize;
    if (height !== undefined && height > 0) emit("resize", height);
    else reportSize();
  });
  resizeObserver.observe(element);
}, { immediate: true, flush: "post" });
onBeforeUnmount(() => resizeObserver?.disconnect());
</script>

<template>
  <article
    ref="rootRef"
    class="alignment-side-block"
    :class="[
      `alignment-side-block--${side}`,
      {
        'alignment-side-block--active': alignmentActive,
        'alignment-side-block--operation-selected': alignmentOperationSelected,
        'alignment-side-block--unlinked': !block.linked,
        'alignment-side-block--highlighted': highlighted,
      },
    ]"
    :data-alignment-id="block.alignmentId"
    @click="emit('selectAlignment', $event)"
  >
    <div class="alignment-side-block__bracket" aria-hidden="true"></div>
    <SegmentCard
      v-for="(segment, segmentIndex) in segments"
      :key="segment.id"
      :segment="segment"
      :side="side"
      :alignment-id="block.alignmentId"
      :mode="mode"
      :writable="writable"
      :reorder-enabled="reorderEnabled"
      :selected="selectedSegmentIds.has(segment.id)"
      :order-selected="isOrderSelected(segment.id)"
      :editing="mode === 'edit' && editSession?.segmentId === segment.id"
      :edit-session="editSession"
      :show-gap-controls="isOrderSelected(segment.id)"
      :can-insert-before="isOrderSelected(segment.id) && canInsertGap(side, segment.id, 'before')"
      :can-insert-after="isOrderSelected(segment.id) && canInsertGap(side, segment.id, 'after')"
      :dragged="draggedId === segment.id"
      :drop-edge="dropTargetId === segment.id ? dropEdge : null"
      :register-order-segment="registerOrderSegment"
      :fragmented="fragmented && segmentIndex === segments.length - 1"
      :bookmarked="bookmarkedSegmentIds.has(segment.id)"
      :annotated="annotatedSegmentIds.has(segment.id)"
      :jump-highlighted="jumpHighlightSegmentId === segment.id"
      @select="emit('selectSegment', segment.id, $event)"
      @select-order="emit('selectOrderSegment', segment.id)"
      @request-edit="emit('requestEdit', segment.id)"
      @insert-gap="emit('insertGap', segment.id, $event)"
      @bookmark="emit('bookmark', segment.id)"
      @annotation="emit('annotation', segment.id)"
      @edit-draft="emit('editDraft', $event)"
      @commit-edit="emit('commitEdit', $event)"
      @cancel-edit="emit('cancelEdit')"
      @escape-edit="emit('escapeEdit')"
    />
    <span v-if="!block.linked" class="alignment-side-block__pending">{{ t("puiPendingMatch") }}</span>
  </article>
</template>

<style scoped>
.alignment-side-block { position: relative; display: flex; min-width: 0; flex-direction: column; gap: 0; overflow: visible; @apply bg-green-soft; transition: background-color 160ms ease, box-shadow 160ms ease; }
.alignment-side-block--active { @apply bg-green-selected; }
.alignment-side-block--operation-selected { box-shadow: inset 0 0 0 2px #80b686; }
.alignment-side-block--unlinked { @apply bg-warm-soft; }
.alignment-side-block--highlighted { animation: alignment-side-highlight 820ms ease-out; }
.alignment-side-block__bracket { position: absolute; z-index: 18; top: 11px; bottom: 11px; width: 10px; @apply text-accent; pointer-events: none; }
.alignment-side-block--source .alignment-side-block__bracket { right: 14px; border-right: 2px solid currentColor; }
.alignment-side-block--target .alignment-side-block__bracket { left: 14px; border-left: 2px solid currentColor; }
.alignment-side-block__bracket::before, .alignment-side-block__bracket::after { position: absolute; width: 9px; height: 2px; background: currentColor; content: ""; }
.alignment-side-block--source .alignment-side-block__bracket::before, .alignment-side-block--source .alignment-side-block__bracket::after { right: 0; }
.alignment-side-block--target .alignment-side-block__bracket::before, .alignment-side-block--target .alignment-side-block__bracket::after { left: 0; }
.alignment-side-block__bracket::before { top: 0; }
.alignment-side-block__bracket::after { bottom: 0; }
.alignment-side-block--unlinked .alignment-side-block__bracket { color: var(--text-warning, #e5a300); }
.alignment-side-block__pending { position: absolute; z-index: 19; right: 30px; bottom: 3px; color: var(--text-warning, #d38e00); font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-semibold); pointer-events: none; line-height: var(--jm-line-height-subheadline); }
.alignment-side-block--target .alignment-side-block__pending { right: auto; left: 30px; }
@keyframes alignment-side-highlight { 0% { box-shadow: inset 0 0 0 3px rgb(77 155 83 / 45%); } 100% { box-shadow: inset 0 0 0 0 rgb(77 155 83 / 0%); } }
@media (prefers-reduced-motion: reduce) { .alignment-side-block { transition: none; } .alignment-side-block--highlighted { animation: none; } }
</style>
