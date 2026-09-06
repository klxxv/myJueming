<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import type { EditSession } from "../../composables/useViewModeController";
import type {
  DropEdge,
  RegisterOrderDropTarget,
  RegisterOrderSegment,
} from "../../composables/useOrderDragAndDrop";
import type { AlignmentGapEdge, LanguageSide, WorkspaceMode } from "../../domain/kernel-client";
import type { AlignmentBlockView, OrderSelection } from "../../domain/workspace-projection";
import AlignmentRelationRail from "./AlignmentRelationRail.vue";
import AlignmentSideBlock from "./AlignmentSideBlock.vue";
import OrderDropZone from "./OrderDropZone.vue";
import { useAlignedBlockLayout } from "./useAlignedBlockLayout";

const props = withDefaults(defineProps<{
  rows: AlignmentBlockView[];
  mode: WorkspaceMode;
  writable: boolean;
  smoothNavigation?: boolean;
  reorderEnabled?: boolean;
  selectedAlignmentId?: string;
  selectedAlignmentIds: Set<string>;
  hasOperationSelection?: boolean;
  selectedSourceIds: Set<string>;
  selectedTargetIds: Set<string>;
  orderSelection?: OrderSelection | null;
  editSession?: EditSession | null;
  bookmarkedSegmentIds: Set<string>;
  annotatedSegmentIds: Set<string>;
  highlightedAlignmentIds?: Set<string>;
  jumpHighlightSegmentId?: string;
  draggedId?: string | null;
  dropTargetId?: string | null;
  dropEdge?: DropEdge | null;
  registerOrderSegment: RegisterOrderSegment;
  registerOrderDropTarget: RegisterOrderDropTarget;
  canInsertGap: (side: LanguageSide, segmentId: string, edge: AlignmentGapEdge) => boolean;
}>(), {
  reorderEnabled: false,
  smoothNavigation: true,
  selectedAlignmentId: "",
  hasOperationSelection: false,
  orderSelection: null,
  editSession: null,
  highlightedAlignmentIds: () => new Set<string>(),
  jumpHighlightSegmentId: "",
  draggedId: null,
  dropTargetId: null,
  dropEdge: null,
});

const emit = defineEmits<{
  selectAlignment: [block: AlignmentBlockView, event: MouseEvent];
  quickRelationAction: [block: AlignmentBlockView];
  selectSegment: [side: LanguageSide, segmentId: string, alignmentId: string, event: MouseEvent];
  selectOrderSegment: [side: LanguageSide, segmentId: string, alignmentId: string];
  requestEdit: [segmentId: string, alignmentId: string];
  insertGap: [segmentId: string, edge: AlignmentGapEdge];
  bookmark: [segmentId: string, alignmentId: string | null];
  annotation: [segmentId: string, alignmentId: string | null];
  editDraft: [draft: string];
  commitEdit: [exitAfterSave: boolean];
  cancelEdit: [];
  escapeEdit: [];
  scrollbarWidth: [width: number];
}>();

const viewportRef = ref<HTMLElement | null>(null);
const viewportWidth = ref(0);
const viewportHeight = ref(0);
const scrollTop = ref(0);
const rowList = computed(() => props.rows);
const { positions, totalHeight, visiblePositions, reportHeight, resetMeasurements } = useAlignedBlockLayout({
  rows: rowList,
  viewportWidth,
  scrollTop,
  viewportHeight,
});

interface OrderDropZoneView {
  key: string;
  side: LanguageSide;
  top: number;
  height: number;
  targetId: string;
  edge: DropEdge;
}

const orderDropZones = computed<OrderDropZoneView[]>(() => {
  if (!props.reorderEnabled) return [];
  const positionIndex = new Map(positions.value.map((position, index) => [position.block.alignmentId, index]));
  const zones: OrderDropZoneView[] = [];
  for (const position of visiblePositions.value) {
    const index = positionIndex.get(position.block.alignmentId);
    if (index === undefined) continue;
    const nextPosition = positions.value[index + 1];
    for (const side of ["source", "target"] as const) {
      const segments = side === "source" ? position.block.sourceSegments : position.block.targetSegments;
      const sideHeight = side === "source" ? position.sourceHeight : position.targetHeight;
      const sideTop = side === "source" ? position.sourceTop : position.targetTop;
      const lastSegment = segments[segments.length - 1];
      const tailHeight = Math.max(0, position.bandHeight - sideHeight);
      const nextSegment = positions.value
        .slice(index + 1)
        .flatMap((candidate) => side === "source"
          ? candidate.block.sourceSegments
          : candidate.block.targetSegments)[0];
      const tailTarget = lastSegment ?? nextSegment;
      if (tailTarget && (tailHeight > 0 || !segments.length)) {
        zones.push({
          key: `${side}-${position.block.alignmentId}-tail`,
          side,
          top: segments.length ? sideTop + sideHeight : position.bandTop,
          height: segments.length ? tailHeight : position.bandHeight,
          targetId: tailTarget.id,
          edge: lastSegment ? "after" : "before",
        });
      }

      if (!nextPosition) continue;
      const gapTop = position.bandTop + position.bandHeight;
      const gapHeight = Math.max(0, nextPosition.bandTop - gapTop);
      const gapTarget = nextSegment ?? lastSegment;
      if (gapTarget && gapHeight > 0) {
        zones.push({
          key: `${side}-${position.block.alignmentId}-gap`,
          side,
          top: gapTop,
          height: gapHeight,
          targetId: gapTarget.id,
          edge: nextSegment ? "before" : "after",
        });
      }
    }
  }
  return zones;
});
const sourceDropZones = computed(() => orderDropZones.value.filter((zone) => zone.side === "source"));
const targetDropZones = computed(() => orderDropZones.value.filter((zone) => zone.side === "target"));

const isAlignmentActive = (block: AlignmentBlockView) => props.selectedAlignmentIds.has(block.alignmentId)
  || (!props.hasOperationSelection && block.alignmentId === props.selectedAlignmentId);

const measureViewport = () => {
  const viewport = viewportRef.value;
  if (!viewport) return;
  viewportWidth.value = viewport.clientWidth;
  viewportHeight.value = viewport.clientHeight;
  emit("scrollbarWidth", Math.max(0, viewport.offsetWidth - viewport.clientWidth));
};
const handleScroll = () => {
  scrollTop.value = viewportRef.value?.scrollTop ?? 0;
};

watch(positions, (currentPositions, previousPositions) => {
  const viewport = viewportRef.value;
  const currentScrollTop = viewport?.scrollTop ?? 0;
  if (!viewport || currentScrollTop <= 0 || !previousPositions.length || !currentPositions.length) return;
  let previousAnchor = previousPositions[0];
  for (const position of previousPositions) {
    if (position.bandTop > currentScrollTop) break;
    previousAnchor = position;
  }
  const currentAnchor = currentPositions.find(
    (position) => position.block.alignmentId === previousAnchor.block.alignmentId,
  );
  if (!currentAnchor) return;
  const anchoredScrollTop = Math.max(0, currentAnchor.bandTop + currentScrollTop - previousAnchor.bandTop);
  if (Math.abs(anchoredScrollTop - currentScrollTop) < 0.5) return;
  viewport.scrollTop = anchoredScrollTop;
  scrollTop.value = anchoredScrollTop;
}, { flush: "sync" });

let resizeObserver: ResizeObserver | null = null;
watch(viewportRef, async (viewport) => {
  resizeObserver?.disconnect();
  resizeObserver = null;
  if (!viewport) return;
  await nextTick();
  measureViewport();
  handleScroll();
  resizeObserver = new ResizeObserver(measureViewport);
  resizeObserver.observe(viewport);
}, { immediate: true, flush: "post" });
onBeforeUnmount(() => resizeObserver?.disconnect());

const focusPosition = (alignmentId: string) => {
  const viewport = viewportRef.value;
  const position = positions.value.find((candidate) => candidate.block.alignmentId === alignmentId);
  if (!viewport || !position) return false;
  const target = Math.max(0, position.bandTop - (viewport.clientHeight - position.bandHeight) / 2);
  viewport.scrollTo({
    top: target,
    behavior: props.smoothNavigation ? "smooth" : "auto",
  });
  return true;
};
const focusAlignment = async (alignmentId: string) => {
  await nextTick();
  return focusPosition(alignmentId);
};
const focusSegment = async (segmentId: string) => {
  const block = props.rows.find((row) => [...row.sourceSegments, ...row.targetSegments]
    .some((segment) => segment.id === segmentId));
  return block ? focusAlignment(block.alignmentId) : false;
};
const focusSegments = async (sourceIds: string[], targetIds: string[]) => {
  await nextTick();
  const viewport = viewportRef.value;
  if (!viewport) return false;

  const groupCenter = (side: LanguageSide, segmentIds: string[]) => {
    const wantedIds = new Set(segmentIds);
    const bounds = positions.value.flatMap((position) => {
      const segments = side === "source" ? position.block.sourceSegments : position.block.targetSegments;
      if (!segments.some((segment) => wantedIds.has(segment.id))) return [];
      const top = side === "source" ? position.sourceTop : position.targetTop;
      const height = side === "source" ? position.sourceHeight : position.targetHeight;
      return [{ top, bottom: top + height }];
    });
    if (!bounds.length) return null;
    return (Math.min(...bounds.map((bound) => bound.top))
      + Math.max(...bounds.map((bound) => bound.bottom))) / 2;
  };

  const centers = [
    groupCenter("source", sourceIds),
    groupCenter("target", targetIds),
  ].filter((center): center is number => center !== null);
  if (!centers.length) return false;
  const combinedCenter = centers.reduce((sum, center) => sum + center, 0) / centers.length;
  const maximumScrollTop = Math.max(0, totalHeight.value - viewport.clientHeight);
  const target = Math.min(maximumScrollTop, Math.max(0, combinedCenter - viewport.clientHeight / 2));
  viewport.scrollTo({
    top: target,
    behavior: props.smoothNavigation ? "smooth" : "auto",
  });
  return true;
};
const focusViewport = () => viewportRef.value?.focus();
const measure = async () => {
  resetMeasurements();
  await nextTick();
  measureViewport();
};
defineExpose({ focusAlignment, focusSegment, focusSegments, focusViewport, measure });
</script>

<template>
  <div
    ref="viewportRef"
    class="aligned-workspace-viewport"
    tabindex="-1"
    aria-label="双语 Alignment 计算对齐工作区"
    @scroll.passive="handleScroll"
  >
    <div class="aligned-workspace-canvas" :style="{ height: `${totalHeight}px` }">
      <section class="aligned-workspace-column aligned-workspace-column--source" aria-label="中文 Segment 列">
        <OrderDropZone
          v-for="zone in sourceDropZones"
          :key="zone.key"
          :style="{ top: `${zone.top}px`, height: `${zone.height}px` }"
          :side="zone.side"
          :target-id="zone.targetId"
          :edge="zone.edge"
          :register-order-drop-target="registerOrderDropTarget"
        />
        <AlignmentSideBlock
          v-for="position in visiblePositions.filter((item) => item.block.sourceSegments.length)"
          :key="`source-${position.block.alignmentId}`"
          class="aligned-workspace-positioned-block"
          :style="{ transform: `translateY(${position.sourceTop}px)` }"
          :block="position.block"
          side="source"
          :mode="mode"
          :writable="writable"
          :reorder-enabled="reorderEnabled"
          :alignment-active="isAlignmentActive(position.block)"
          :alignment-operation-selected="selectedAlignmentIds.has(position.block.alignmentId)"
          :order-selection="orderSelection"
          :selected-segment-ids="selectedSourceIds"
          :edit-session="editSession"
          :bookmarked-segment-ids="bookmarkedSegmentIds"
          :annotated-segment-ids="annotatedSegmentIds"
          :highlighted="highlightedAlignmentIds.has(position.block.alignmentId)"
          :jump-highlight-segment-id="jumpHighlightSegmentId"
          :dragged-id="draggedId"
          :drop-target-id="dropTargetId"
          :drop-edge="dropEdge"
          :register-order-segment="registerOrderSegment"
          :can-insert-gap="canInsertGap"
          @resize="reportHeight(position.block.alignmentId, 'source', $event)"
          @select-alignment="emit('selectAlignment', position.block, $event)"
          @select-segment="(segmentId, event) => emit('selectSegment', 'source', segmentId, position.block.alignmentId, event)"
          @select-order-segment="emit('selectOrderSegment', 'source', $event, position.block.alignmentId)"
          @request-edit="emit('requestEdit', $event, position.block.alignmentId)"
          @insert-gap="(segmentId, edge) => emit('insertGap', segmentId, edge)"
          @bookmark="emit('bookmark', $event, position.block.linked ? position.block.alignmentId : null)"
          @annotation="emit('annotation', $event, position.block.linked ? position.block.alignmentId : null)"
          @edit-draft="emit('editDraft', $event)"
          @commit-edit="emit('commitEdit', $event)"
          @cancel-edit="emit('cancelEdit')"
          @escape-edit="emit('escapeEdit')"
        />
      </section>

      <section class="aligned-workspace-relations" aria-label="Alignment 关系轨">
        <AlignmentRelationRail
          v-for="position in visiblePositions"
          :key="`relation-${position.block.alignmentId}`"
          class="aligned-workspace-positioned-relation"
          :style="{ transform: `translateY(${position.bandTop}px)`, height: `${position.bandHeight}px` }"
          :block="position.block"
          :connector-y="position.connectorY"
          :writable="writable"
          :active="isAlignmentActive(position.block)"
          :operation-selected="selectedAlignmentIds.has(position.block.alignmentId)"
          @select="emit('selectAlignment', position.block, $event)"
          @quick-action="emit('quickRelationAction', position.block)"
        />
      </section>

      <section class="aligned-workspace-column aligned-workspace-column--target" aria-label="英文 Segment 列">
        <OrderDropZone
          v-for="zone in targetDropZones"
          :key="zone.key"
          :style="{ top: `${zone.top}px`, height: `${zone.height}px` }"
          :side="zone.side"
          :target-id="zone.targetId"
          :edge="zone.edge"
          :register-order-drop-target="registerOrderDropTarget"
        />
        <AlignmentSideBlock
          v-for="position in visiblePositions.filter((item) => item.block.targetSegments.length)"
          :key="`target-${position.block.alignmentId}`"
          class="aligned-workspace-positioned-block"
          :style="{ transform: `translateY(${position.targetTop}px)` }"
          :block="position.block"
          side="target"
          :mode="mode"
          :writable="writable"
          :reorder-enabled="reorderEnabled"
          :alignment-active="isAlignmentActive(position.block)"
          :alignment-operation-selected="selectedAlignmentIds.has(position.block.alignmentId)"
          :order-selection="orderSelection"
          :selected-segment-ids="selectedTargetIds"
          :edit-session="editSession"
          :bookmarked-segment-ids="bookmarkedSegmentIds"
          :annotated-segment-ids="annotatedSegmentIds"
          :highlighted="highlightedAlignmentIds.has(position.block.alignmentId)"
          :jump-highlight-segment-id="jumpHighlightSegmentId"
          :dragged-id="draggedId"
          :drop-target-id="dropTargetId"
          :drop-edge="dropEdge"
          :register-order-segment="registerOrderSegment"
          :can-insert-gap="canInsertGap"
          @resize="reportHeight(position.block.alignmentId, 'target', $event)"
          @select-alignment="emit('selectAlignment', position.block, $event)"
          @select-segment="(segmentId, event) => emit('selectSegment', 'target', segmentId, position.block.alignmentId, event)"
          @select-order-segment="emit('selectOrderSegment', 'target', $event, position.block.alignmentId)"
          @request-edit="emit('requestEdit', $event, position.block.alignmentId)"
          @insert-gap="(segmentId, edge) => emit('insertGap', segmentId, edge)"
          @bookmark="emit('bookmark', $event, position.block.linked ? position.block.alignmentId : null)"
          @annotation="emit('annotation', $event, position.block.linked ? position.block.alignmentId : null)"
          @edit-draft="emit('editDraft', $event)"
          @commit-edit="emit('commitEdit', $event)"
          @cancel-edit="emit('cancelEdit')"
          @escape-edit="emit('escapeEdit')"
        />
      </section>
    </div>
  </div>
</template>

<style scoped>
.aligned-workspace-viewport { --alignment-gutter: 176px; min-height: 0; flex: 1 1 auto; overflow: auto; overscroll-behavior: contain; scrollbar-gutter: stable; background: var(--surface-raised); }
.aligned-workspace-canvas { position: relative; display: grid; grid-template-columns: minmax(0, var(--source-column-fr)) var(--alignment-gutter) minmax(0, var(--target-column-fr)); width: 100%; min-height: 100%; }
.aligned-workspace-column, .aligned-workspace-relations { position: relative; min-width: 0; height: 100%; }
.aligned-workspace-relations { border-right: 1px solid var(--line); border-left: 1px solid var(--line); background: var(--surface-subtle); }
.aligned-workspace-positioned-block, .aligned-workspace-positioned-relation { position: absolute; z-index: 3; top: 0; right: 0; left: 0; }
.aligned-workspace-positioned-relation { z-index: 5; }
@media (max-width: 1280px) { .aligned-workspace-viewport { --alignment-gutter: 138px; } }
</style>
