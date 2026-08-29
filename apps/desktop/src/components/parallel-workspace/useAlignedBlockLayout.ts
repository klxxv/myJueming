import { computed, onBeforeUnmount, shallowRef, watch, type ComputedRef, type Ref } from "vue";
import type { LanguageSide, SegmentDto } from "../../domain/kernel-client";
import type { AlignmentBlockView } from "../../domain/workspace-projection";

export interface AlignedBlockPosition {
  block: AlignmentBlockView;
  bandTop: number;
  bandHeight: number;
  sourceHeight: number;
  targetHeight: number;
  sourceTop: number;
  targetTop: number;
  connectorY: number;
}

const MIN_ALIGNMENT_HEIGHT = 76;
const ALIGNMENT_GAP = 12;
const FIRST_LINE_CONNECTOR_Y = 31;
const OVERSCAN_PX = 720;

const estimateSegmentHeight = (segment: SegmentDto, side: LanguageSide, columnWidth: number) => {
  const contentWidth = Math.max(180, columnWidth - 142);
  const characterWidth = side === "source" ? 16 : 8.2;
  const charactersPerLine = Math.max(12, Math.floor(contentWidth / characterWidth));
  const lines = Math.max(1, Math.ceil(segment.text.length / charactersPerLine));
  return Math.max(MIN_ALIGNMENT_HEIGHT, 36 + lines * 26);
};

export function useAlignedBlockLayout({
  rows,
  viewportWidth,
  scrollTop,
  viewportHeight,
}: {
  rows: ComputedRef<AlignmentBlockView[]>;
  viewportWidth: Ref<number>;
  scrollTop: Ref<number>;
  viewportHeight: Ref<number>;
}) {
  const measuredSourceHeights = shallowRef(new Map<string, number>());
  const measuredTargetHeights = shallowRef(new Map<string, number>());

  const columnWidth = computed(() => Math.max(320, (viewportWidth.value - 176) / 2));
  const estimateSideHeight = (block: AlignmentBlockView, side: LanguageSide) => {
    const segments = side === "source" ? block.sourceSegments : block.targetSegments;
    if (!segments.length) return 0;
    return segments.reduce((height, segment) => height + estimateSegmentHeight(segment, side, columnWidth.value), 0);
  };

  const positions = computed<AlignedBlockPosition[]>(() => {
    let bandTop = 0;
    return rows.value.map((block) => {
      const sourceHeight = measuredSourceHeights.value.get(block.alignmentId)
        ?? estimateSideHeight(block, "source");
      const targetHeight = measuredTargetHeights.value.get(block.alignmentId)
        ?? estimateSideHeight(block, "target");
      const bandHeight = Math.max(MIN_ALIGNMENT_HEIGHT, sourceHeight, targetHeight);
      const position = {
        block,
        bandTop,
        bandHeight,
        sourceHeight,
        targetHeight,
        sourceTop: bandTop,
        targetTop: bandTop,
        connectorY: FIRST_LINE_CONNECTOR_Y,
      };
      bandTop += bandHeight + ALIGNMENT_GAP;
      return position;
    });
  });

  const totalHeight = computed(() => {
    const last = positions.value[positions.value.length - 1];
    return last ? last.bandTop + last.bandHeight : 0;
  });

  const visiblePositions = computed(() => {
    const start = Math.max(0, scrollTop.value - OVERSCAN_PX);
    const end = scrollTop.value + viewportHeight.value + OVERSCAN_PX;
    return positions.value.filter((position) => (
      position.bandTop + position.bandHeight >= start && position.bandTop <= end
    ));
  });

  const reportHeight = (alignmentId: string, side: LanguageSide, height: number) => {
    if (!Number.isFinite(height) || height <= 0) return;
    const target = side === "source" ? measuredSourceHeights : measuredTargetHeights;
    const previousHeight = target.value.get(alignmentId);
    if (previousHeight !== undefined && Math.abs(previousHeight - height) < 0.001) return;
    const next = new Map(target.value);
    next.set(alignmentId, height);
    target.value = next;
  };

  const resetMeasurements = () => {
    measuredSourceHeights.value = new Map();
    measuredTargetHeights.value = new Map();
  };

  watch(
    () => rows.value.map((row) => [
      row.alignmentId,
      ...row.sourceSegments.map((segment) => `${segment.id}:${segment.text}`),
      ...row.targetSegments.map((segment) => `${segment.id}:${segment.text}`),
    ].join("\u0000")).join("\u0001"),
    resetMeasurements,
  );

  let debugFrame: number | null = null;
  watch(
    [positions, visiblePositions],
    () => {
      if (!import.meta.env.DEV || debugFrame !== null) return;
      debugFrame = requestAnimationFrame(() => {
        debugFrame = null;
        const largestGap = positions.value.reduce((largest, position) => Math.max(
          largest,
          Math.abs(position.sourceHeight - position.targetHeight),
        ), 0);
        console.debug("[jueming:alignment-layout]", {
          blocks: positions.value.length,
          visibleBlocks: visiblePositions.value.length,
          measuredSource: measuredSourceHeights.value.size,
          measuredTarget: measuredTargetHeights.value.size,
          totalHeight: totalHeight.value,
          alignmentGap: ALIGNMENT_GAP,
          connectorY: FIRST_LINE_CONNECTOR_Y,
          largestComputedGap: largestGap,
        });
      });
    },
    { immediate: true },
  );

  onBeforeUnmount(() => {
    if (debugFrame !== null) cancelAnimationFrame(debugFrame);
  });

  return {
    positions,
    totalHeight,
    visiblePositions,
    reportHeight,
    resetMeasurements,
  };
}
