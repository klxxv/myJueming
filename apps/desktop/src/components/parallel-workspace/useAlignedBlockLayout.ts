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
  const lines = Math.max(1, Math.ceil((segment.contentLength ?? segment.text.length) / charactersPerLine));
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
  const measurements = shallowRef({ source: new Map<string, number>(), target: new Map<string, number>() });
  let measurementFrame: number | null = null;
  const pendingMeasurements = { source: new Map<string, number>(), target: new Map<string, number>() };
  const blocksById = computed(() => new Map(rows.value.map(block => [block.alignmentId, block])));

  const columnWidth = computed(() => Math.max(320, (viewportWidth.value - 176) / 2));
  const estimateSideHeight = (block: AlignmentBlockView, side: LanguageSide) => {
    const segments = side === "source" ? block.sourceSegments : block.targetSegments;
    if (!segments.length) return 0;
    return segments.reduce((height, segment) => height + estimateSegmentHeight(segment, side, columnWidth.value), 0);
  };

  const positions = computed<AlignedBlockPosition[]>(() => {
    let bandTop = 0;
    return rows.value.map((block) => {
      const sourceHeight = measurements.value.source.get(block.alignmentId)
        ?? estimateSideHeight(block, "source");
      const targetHeight = measurements.value.target.get(block.alignmentId)
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
    const all = positions.value;
    let low = 0;
    let high = all.length;
    while (low < high) {
      const middle = (low + high) >>> 1;
      if (all[middle].bandTop + all[middle].bandHeight < start) low = middle + 1;
      else high = middle;
    }
    let last = low;
    while (last < all.length && all[last].bandTop <= end) last++;
    return all.slice(low, last);
  });

  const reportHeight = (alignmentId: string, side: LanguageSide, height: number) => {
    const block = blocksById.value.get(alignmentId);
    if (block && (side === "source" ? block.sourceSegments : block.targetSegments).some(segment => segment.loaded === false)) return;
    if (!Number.isFinite(height) || height <= 0) return;
    const previousHeight = pendingMeasurements[side].get(alignmentId) ?? measurements.value[side].get(alignmentId);
    if (previousHeight !== undefined && Math.abs(previousHeight - height) < 0.5) return;
    pendingMeasurements[side].set(alignmentId, height);
    if (measurementFrame !== null) return;
    measurementFrame = requestAnimationFrame(() => {
      measurementFrame = null;
      // Publish both columns together so layout and anchor updates share one batch.
      const next = { source: new Map(measurements.value.source), target: new Map(measurements.value.target) };
      for (const column of ["source", "target"] as const) {
        for (const [id, measuredHeight] of pendingMeasurements[column]) {
          if (blocksById.value.has(id)) next[column].set(id, measuredHeight);
        }
        pendingMeasurements[column].clear();
      }
      measurements.value = next;
    });
  };

  const resetMeasurements = () => {
    if (measurementFrame !== null) cancelAnimationFrame(measurementFrame);
    measurementFrame = null;
    pendingMeasurements.source.clear();
    pendingMeasurements.target.clear();
    measurements.value = { source: new Map(), target: new Map() };
  };

  watch(
    () => rows.value.map((row) => [
      row.alignmentId,
      ...row.sourceSegments.map((segment) => `${segment.id}:${segment.contentHash ?? segment.text}`),
      ...row.targetSegments.map((segment) => `${segment.id}:${segment.contentHash ?? segment.text}`),
    ].join("\u0000")).join("\u0001"),
    resetMeasurements,
  );

  let debugFrame: number | null = null;
  if (import.meta.env.DEV && import.meta.env.VITE_ALIGNMENT_LAYOUT_DEBUG === "true") {
    watch(
      [positions, visiblePositions],
      () => {
        if (debugFrame !== null) return;
        debugFrame = requestAnimationFrame(() => {
          debugFrame = null;
          const largestGap = positions.value.reduce((largest, position) => Math.max(
            largest,
            Math.abs(position.sourceHeight - position.targetHeight),
          ), 0);
          console.debug("[jueming:alignment-layout]", {
            blocks: positions.value.length,
            visibleBlocks: visiblePositions.value.length,
            measuredSource: measurements.value.source.size,
            measuredTarget: measurements.value.target.size,
            totalHeight: totalHeight.value,
            alignmentGap: ALIGNMENT_GAP,
            connectorY: FIRST_LINE_CONNECTOR_Y,
            largestComputedGap: largestGap,
          });
        });
      },
      { immediate: true },
    );
  }

  onBeforeUnmount(() => {
    if (debugFrame !== null) cancelAnimationFrame(debugFrame);
    if (measurementFrame !== null) cancelAnimationFrame(measurementFrame);
  });

  return {
    positions,
    totalHeight,
    visiblePositions,
    reportHeight,
    resetMeasurements,
  };
}
