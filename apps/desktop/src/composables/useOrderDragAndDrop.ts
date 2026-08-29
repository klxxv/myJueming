import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { combine } from "@atlaskit/pragmatic-drag-and-drop/utils/combine";
import { draggable, dropTargetForElements, monitorForElements } from "@atlaskit/pragmatic-drag-and-drop/adapter/element-adapter";
import type { LanguageSide, SegmentDto } from "../domain/kernel-client";

export type DropEdge = "before" | "after";
export type DragPointer = { x: number; y: number };
export interface ReorderIntent {
  segmentId: string;
  targetId: string;
  edge: DropEdge;
}
export type RegisterOrderSegment = (element: HTMLElement, segmentId: string, side: LanguageSide) => () => void;
export type RegisterOrderDropTarget = (
  element: HTMLElement,
  targetId: string,
  side: LanguageSide,
  edge: DropEdge,
) => () => void;

const isSegmentDrag = (data: Record<string | symbol, unknown>) => data.kind === "jueming-segment"
  && typeof data.segmentId === "string"
  && (data.side === "source" || data.side === "target");
const debugDrag = (event: string, details: Record<string, unknown>) => {
  if (!import.meta.env.DEV) return;
  console.debug(`[jueming:dnd] ${event} ${JSON.stringify(details)}`);
};

export const useOrderDragAndDrop = ({
  getSegments,
  onReorder,
  afterReorderApplied,
}: {
  getSegments: (side: LanguageSide) => SegmentDto[];
  onReorder: (side: LanguageSide, orderedSegmentIds: string[], intent: ReorderIntent) => void;
  afterReorderApplied?: () => void | Promise<void>;
}) => {
  const draggedId = ref<string | null>(null);
  const dropTargetId = ref<string | null>(null);
  const dropEdge = ref<DropEdge | null>(null);
  const dragPointer = ref<DragPointer | null>(null);
  const listInstanceIds: Record<LanguageSide, symbol> = {
    source: Symbol("jueming-source-order-list"),
    target: Symbol("jueming-target-order-list"),
  };
  let pendingOrder: { side: LanguageSide; signature: string } | null = null;
  let pendingDropTimeout: ReturnType<typeof setTimeout> | null = null;

  const draggedSide = ref<LanguageSide | null>(null);
  const draggedSegment = computed(() => draggedId.value && draggedSide.value
    ? getSegments(draggedSide.value).find((segment) => segment.id === draggedId.value) ?? null
    : null);
  const updateDragPointer = (input: { clientX: number; clientY: number }) => {
    dragPointer.value = { x: input.clientX, y: input.clientY };
  };
  const clearDropFeedback = () => {
    dropTargetId.value = null;
    dropEdge.value = null;
  };
  const clearDragVisualState = () => {
    draggedId.value = null;
    draggedSide.value = null;
    clearDropFeedback();
    dragPointer.value = null;
  };
  const clearPendingDrop = () => {
    pendingOrder = null;
    if (pendingDropTimeout !== null) clearTimeout(pendingDropTimeout);
    pendingDropTimeout = null;
  };
  const clearDragState = () => {
    clearDragVisualState();
    clearPendingDrop();
  };
  const commitDrop = (side: LanguageSide, segmentId: string, targetId: string, edge: DropEdge) => {
    const order = [...getSegments(side)]
      .sort((a, b) => a.order - b.order)
      .map((segment) => segment.id);
    const currentOrder = order.join("\u0000");
    const fromIndex = order.indexOf(segmentId);
    if (fromIndex < 0) return;
    order.splice(fromIndex, 1);
    const targetIndex = order.indexOf(targetId);
    if (targetIndex < 0) return;
    order.splice(edge === "after" ? targetIndex + 1 : targetIndex, 0, segmentId);
    pendingOrder = { side, signature: order.join("\u0000") };
    if (pendingOrder.signature === currentOrder) {
      clearDragState();
      return;
    }
    pendingDropTimeout = setTimeout(clearPendingDrop, 5000);
    onReorder(side, order, { segmentId, targetId, edge });
  };
  const edgeForElement = (element: HTMLElement, clientY: number): DropEdge => {
    const bounds = element.getBoundingClientRect();
    return clientY < bounds.top + bounds.height / 2 ? "before" : "after";
  };
  const showDropFeedback = (side: LanguageSide, sourceId: string, targetId: string, edge: DropEdge) => {
    if (sourceId === targetId) return false;
    const ids = [...getSegments(side)]
      .sort((a, b) => a.order - b.order)
      .map((segment) => segment.id);
    const sourceIndex = ids.indexOf(sourceId);
    const targetIndex = ids.indexOf(targetId);
    if (sourceIndex < 0 || targetIndex < 0) return false;
    if (sourceIndex === targetIndex - 1 && edge === "before") return false;
    if (sourceIndex === targetIndex + 1 && edge === "after") return false;
    return true;
  };
  const updateDropFeedback = (side: LanguageSide, sourceId: unknown, targetId: string, edgeData: unknown) => {
    const edge: DropEdge = edgeData === "after" ? "after" : "before";
    if (typeof sourceId !== "string" || !showDropFeedback(side, sourceId, targetId, edge)) {
      clearDropFeedback();
      return;
    }
    dropTargetId.value = targetId;
    dropEdge.value = edge;
  };

  // Match the official PDD list pattern: the full Segment is the draggable and
  // drop target, while the nested button is only the drag handle. This keeps
  // source and target geometry identical and avoids a button-sized drag source.
  const registerOrderSegment: RegisterOrderSegment = (element, segmentId, side) => {
    const dragHandle = element.querySelector<HTMLElement>("[data-order-drag-handle]");
    if (!dragHandle) return () => undefined;
    const instanceId = listInstanceIds[side];
    const data = { kind: "jueming-segment", segmentId, side, instanceId };
    return combine(
    draggable({
      element,
      dragHandle,
      getInitialData: () => data,
      onDragStart: ({ location }) => {
        clearDragState();
        draggedId.value = segmentId;
        draggedSide.value = side;
        updateDragPointer(location.initial.input);
        debugDrag("start", {
          segmentId,
          side,
          runtime: "__TAURI_INTERNALS__" in window ? "tauri" : "browser",
        });
      },
      onDrag: ({ location }) => updateDragPointer(location.current.input),
      // Persistence can round-trip through Rust; native drag feedback must end
      // immediately when the pointer is released.
      onDrop: () => requestAnimationFrame(clearDragVisualState),
    }),
    dropTargetForElements({
      element,
      canDrop: ({ source }) => isSegmentDrag(source.data)
        && source.data.instanceId === instanceId
        && source.data.side === side,
      getDropEffect: () => "move",
      getData: ({ input }) => ({
        kind: "jueming-segment-target",
        targetId: segmentId,
        side,
        edge: edgeForElement(element, input.clientY),
        instanceId,
      }),
      onDragEnter: ({ source, self }) => {
        updateDropFeedback(side, source.data.segmentId, segmentId, self.data.edge);
        debugDrag("target", {
          sourceId: source.data.segmentId,
          targetId: segmentId,
          side,
          edge: self.data.edge,
        });
      },
      onDrag: ({ source, self }) => {
        updateDropFeedback(side, source.data.segmentId, segmentId, self.data.edge);
      },
      onDragLeave: () => {
        if (dropTargetId.value === segmentId) clearDropFeedback();
      },
      onDrop: clearDropFeedback,
    }),
  );
  };

  const registerOrderDropTarget: RegisterOrderDropTarget = (element, targetId, side, edge) => {
    const instanceId = listInstanceIds[side];
    return dropTargetForElements({
      element,
      canDrop: ({ source }) => isSegmentDrag(source.data)
        && source.data.instanceId === instanceId
        && source.data.side === side,
      getDropEffect: () => "move",
      getData: () => ({
        kind: "jueming-segment-target",
        targetId,
        side,
        edge,
        instanceId,
      }),
      onDragEnter: ({ source }) => {
        updateDropFeedback(side, source.data.segmentId, targetId, edge);
        debugDrag("spacer-target", {
          sourceId: source.data.segmentId,
          targetId,
          side,
          edge,
        });
      },
      onDrag: ({ source }) => updateDropFeedback(side, source.data.segmentId, targetId, edge),
      onDragLeave: () => {
        if (dropTargetId.value === targetId && dropEdge.value === edge) clearDropFeedback();
      },
      onDrop: clearDropFeedback,
    });
  };

  // As in the official example, one list monitor owns the mutation. Segment
  // drop targets only describe intent and visual feedback.
  const cleanupMonitor = monitorForElements({
    canMonitor: ({ source }) => isSegmentDrag(source.data)
      && source.data.instanceId === listInstanceIds[source.data.side as LanguageSide],
    onDrop: ({ location, source }) => {
      const target = location.current.dropTargets[0];
      if (!target || target.data.instanceId !== source.data.instanceId || target.data.side !== source.data.side) {
        debugDrag("rejected-drop", {
          sourceId: source.data.segmentId,
          sourceSide: source.data.side,
          target: target?.data ?? null,
        });
        return;
      }
      const sourceId = source.data.segmentId;
      const targetId = target.data.targetId;
      const side = source.data.side;
      if ((side === "source" || side === "target")
        && typeof sourceId === "string"
        && typeof targetId === "string"
        && sourceId !== targetId) {
        debugDrag("commit", {
          sourceId,
          targetId,
          side,
          edge: target.data.edge,
        });
        commitDrop(side, sourceId, targetId, target.data.edge === "after" ? "after" : "before");
      }
    },
  });

  watch(
    () => ({
      source: [...getSegments("source")].sort((a, b) => a.order - b.order).map((segment) => segment.id).join("\u0000"),
      target: [...getSegments("target")].sort((a, b) => a.order - b.order).map((segment) => segment.id).join("\u0000"),
    }),
    async (currentOrders) => {
      if (pendingOrder === null || currentOrders[pendingOrder.side] !== pendingOrder.signature) return;
      await nextTick();
      await afterReorderApplied?.();
      clearPendingDrop();
    },
  );

  onBeforeUnmount(() => {
    cleanupMonitor();
    clearDragState();
  });
  return {
    draggedId,
    draggedSide,
    dropTargetId,
    dropEdge,
    dragPointer,
    draggedSegment,
    registerOrderSegment,
    registerOrderDropTarget,
  };
};
