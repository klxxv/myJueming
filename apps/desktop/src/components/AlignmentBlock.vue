<script setup lang="ts">
import { Link2, Link2Off, Star } from "@lucide/vue";
import type { EditSession } from "../composables/useViewModeController";
import type { DropEdge, RegisterOrderSegment } from "../composables/useOrderDragAndDrop";
import type { AlignmentGapEdge, LanguageSide, WorkspaceMode } from "../domain/kernel-client";
import type { AlignmentBlockView, OrderSelection } from "../domain/workspace-projection";
import SegmentText from "./SegmentText.vue";

const props = withDefaults(defineProps<{
  block: AlignmentBlockView;
  mode: WorkspaceMode;
  writable: boolean;
  alignmentActive?: boolean;
  alignmentOperationSelected?: boolean;
  orderSelection?: OrderSelection | null;
  selectedSourceIds: Set<string>;
  selectedTargetIds: Set<string>;
  editSession?: EditSession | null;
  bookmarked?: boolean;
  highlighted?: boolean;
  draggedId?: string | null;
  dropTargetId?: string | null;
  dropEdge?: DropEdge | null;
  registerOrderSegment: RegisterOrderSegment;
  canInsertGap: (side: LanguageSide, segmentId: string, edge: AlignmentGapEdge) => boolean;
}>(), {
  alignmentActive: false,
  alignmentOperationSelected: false,
  orderSelection: null,
  editSession: null,
  bookmarked: false,
  highlighted: false,
  draggedId: null,
  dropTargetId: null,
  dropEdge: null,
});

const emit = defineEmits<{
  selectAlignment: [event: MouseEvent];
  selectSegment: [side: LanguageSide, segmentId: string, event: MouseEvent];
  selectOrderSegment: [side: LanguageSide, segmentId: string];
  requestEdit: [segmentId: string];
  insertGap: [segmentId: string, edge: AlignmentGapEdge];
  bookmark: [];
  editDraft: [draft: string];
  commitEdit: [exitAfterSave: boolean];
  cancelEdit: [];
  escapeEdit: [];
}>();

const isOrderSelected = (side: LanguageSide, segmentId: string) => props.mode === "order"
  && props.orderSelection?.side === side
  && props.orderSelection.segmentId === segmentId;
const alignmentKind = () => props.block.linked
  ? `${props.block.sourceSegments.length}:${props.block.targetSegments.length}`
  : "未对齐";
</script>

<template>
  <article
    class="alignment-block"
    :class="{
      'alignment-block--active': alignmentActive,
      'alignment-block--operation-selected': alignmentOperationSelected,
      'alignment-block--unlinked': !block.linked,
      'alignment-block--highlighted': highlighted,
      'alignment-block--order': mode === 'order',
    }"
    :data-alignment-id="block.alignmentId"
  >
    <section class="alignment-block__side alignment-block__side--source" aria-label="中文 Segments">
      <SegmentText
        v-for="segment in block.sourceSegments"
        :key="segment.id"
        :segment="segment"
        side="source"
        :alignment-id="block.alignmentId"
        :mode="mode"
        :writable="writable"
        :selected="selectedSourceIds.has(segment.id)"
        :order-selected="isOrderSelected('source', segment.id)"
        :editing="mode === 'edit' && editSession?.segmentId === segment.id"
        :edit-session="editSession"
        :show-gap-controls="isOrderSelected('source', segment.id)"
        :can-insert-before="canInsertGap('source', segment.id, 'before')"
        :can-insert-after="canInsertGap('source', segment.id, 'after')"
        :dragged="draggedId === segment.id"
        :drop-edge="dropTargetId === segment.id ? dropEdge : null"
        :register-order-segment="registerOrderSegment"
        @select="emit('selectSegment', 'source', segment.id, $event)"
        @select-order="emit('selectOrderSegment', 'source', segment.id)"
        @request-edit="emit('requestEdit', segment.id)"
        @insert-gap="emit('insertGap', segment.id, $event)"
        @edit-draft="emit('editDraft', $event)"
        @commit-edit="emit('commitEdit', $event)"
        @cancel-edit="emit('cancelEdit')"
        @escape-edit="emit('escapeEdit')"
      />
      <div v-if="!block.sourceSegments.length" class="alignment-block__empty-side">中文侧空位</div>
      <button
        v-if="block.linked"
        class="alignment-block__bookmark"
        :class="{ 'alignment-block__bookmark--on': bookmarked }"
        type="button"
        :disabled="!writable"
        :aria-label="bookmarked ? '移除书签' : '添加书签'"
        @click.stop="emit('bookmark')"
      ><Star :size="18" :fill="bookmarked ? 'currentColor' : 'none'" /></button>
    </section>

    <button
      class="alignment-block__relation"
      :class="{ 'alignment-block__relation--active': alignmentActive || alignmentOperationSelected }"
      type="button"
      :aria-pressed="alignmentOperationSelected"
      :title="block.linked ? '选择整个 Alignment；按 Ctrl/⌘ 多选后 Group' : '未对齐；请分别选择两侧 Segment 后 Link'"
      @click.stop="emit('selectAlignment', $event)"
    >
      <span class="alignment-block__relation-icon"><Link2 v-if="block.linked" :size="22" /><Link2Off v-else :size="22" /></span>
      <span v-if="mode === 'order'" class="alignment-block__block-index">{{ String(block.index + 1).padStart(2, '0') }}</span>
      <span class="alignment-block__kind">{{ alignmentKind() }}</span>
    </button>

    <section class="alignment-block__side alignment-block__side--target" aria-label="英文 Segments">
      <SegmentText
        v-for="segment in block.targetSegments"
        :key="segment.id"
        :segment="segment"
        side="target"
        :alignment-id="block.alignmentId"
        :mode="mode"
        :writable="writable"
        :selected="selectedTargetIds.has(segment.id)"
        :order-selected="isOrderSelected('target', segment.id)"
        :editing="mode === 'edit' && editSession?.segmentId === segment.id"
        :edit-session="editSession"
        :show-gap-controls="isOrderSelected('target', segment.id)"
        :can-insert-before="canInsertGap('target', segment.id, 'before')"
        :can-insert-after="canInsertGap('target', segment.id, 'after')"
        :dragged="draggedId === segment.id"
        :drop-edge="dropTargetId === segment.id ? dropEdge : null"
        :register-order-segment="registerOrderSegment"
        @select="emit('selectSegment', 'target', segment.id, $event)"
        @select-order="emit('selectOrderSegment', 'target', segment.id)"
        @request-edit="emit('requestEdit', segment.id)"
        @insert-gap="emit('insertGap', segment.id, $event)"
        @edit-draft="emit('editDraft', $event)"
        @commit-edit="emit('commitEdit', $event)"
        @cancel-edit="emit('cancelEdit')"
        @escape-edit="emit('escapeEdit')"
      />
      <div v-if="!block.targetSegments.length" class="alignment-block__empty-side">英文侧空位</div>
    </section>
  </article>
</template>

<style scoped>
.alignment-block { position: relative; display: grid; grid-template-columns: minmax(0, 1fr) 66px minmax(0, 1fr); min-height: 106px; border-bottom: 1px solid var(--line); background: var(--surface-raised); transition: background-color 180ms ease, box-shadow 180ms ease; }
.alignment-block--active { background: linear-gradient(90deg, var(--green-050) 0%, var(--surface-raised) 48%, var(--target-soft) 100%); }
.alignment-block--operation-selected { box-shadow: inset 0 0 0 2px #80b686; }
.alignment-block--unlinked { background: var(--surface-disabled); }
.alignment-block--highlighted { animation: alignment-block-highlight 820ms ease-out; }
.alignment-block--order { user-select: none; }
.alignment-block__side { position: relative; min-width: 0; }
.alignment-block__side--source { z-index: 2; }
.alignment-block__side--target { z-index: 1; }
.alignment-block__empty-side { min-height: 76px; padding: 26px; color: var(--ink-500); font-size: 12px; font-style: italic; }
.alignment-block__bookmark { position: absolute; z-index: 8; top: 18px; right: 14px; padding: 4px; border: 0; color: #bfc8c0; background: transparent; cursor: pointer; }
.alignment-block__bookmark--on { color: #e9aa2f; }
.alignment-block__bookmark:disabled { cursor: not-allowed; opacity: .55; }
.alignment-block__relation { position: relative; z-index: 5; display: flex; min-width: 0; flex-direction: column; align-items: center; justify-content: center; gap: 5px; padding: 12px 4px; border: 0; border-right: 1px solid var(--line); border-left: 1px solid var(--line); color: #aab2ac; background: var(--surface-muted); cursor: pointer; }
.alignment-block__relation:hover { color: var(--green-700); background: var(--surface-green-soft); }
.alignment-block__relation--active { color: var(--green-700); background: var(--surface-green-selected); box-shadow: inset 0 0 0 2px #80b686; }
.alignment-block__relation-icon { display: grid; width: 34px; height: 30px; place-items: center; border: 1px solid currentColor; border-radius: 6px; background: var(--surface-raised); }
.alignment-block__block-index, .alignment-block__kind { color: var(--ink-500); font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: 10px; }
.alignment-block__kind { color: #5f8367; }
.alignment-block--unlinked .alignment-block__kind { color: #929a94; }
@keyframes alignment-block-highlight { 0% { box-shadow: inset 0 0 0 3px rgb(77 155 83 / 45%), 0 0 0 0 rgb(77 155 83 / 24%); } 45% { box-shadow: inset 0 0 0 2px rgb(77 155 83 / 26%), 0 0 0 8px rgb(77 155 83 / 0%); } 100% { box-shadow: inset 0 0 0 0 rgb(77 155 83 / 0%); } }
@media (prefers-reduced-motion: reduce) { .alignment-block { transition: none; } .alignment-block--highlighted { animation: none; } }
</style>
