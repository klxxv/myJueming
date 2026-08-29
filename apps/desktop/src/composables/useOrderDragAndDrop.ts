import { computed, nextTick, onBeforeUnmount, ref, watch, type DirectiveBinding, type ObjectDirective } from "vue";
import { draggable, dropTargetForElements } from "@atlaskit/pragmatic-drag-and-drop/adapter/element-adapter";
import type { SegmentDto, WorkspaceMode } from "../domain/kernel-client";

export type DropEdge = "before" | "after";
export type DragPointer = { x: number; y: number };
export type OrderDirectiveValue = { mode: WorkspaceMode; segmentId: string; writable: boolean };
export type OrderDropZoneDirectiveValue = { mode: WorkspaceMode; writable: boolean };
type DndRegistration = { key: string; cleanup: () => void };

export const useOrderDragAndDrop = ({
  getSourceSegments,
  onReorder,
  afterReorderApplied,
}: {
  getSourceSegments: () => SegmentDto[];
  onReorder: (orderedSegmentIds: string[]) => void;
  afterReorderApplied?: () => void | Promise<void>;
}) => {
  const draggedId = ref<string | null>(null);
  const dropTargetId = ref<string | null>(null);
  const dropEdge = ref<DropEdge | null>(null);
  const dragPointer = ref<DragPointer | null>(null);
  const dndRegistrations = new WeakMap<HTMLElement, DndRegistration>();
  const dropZoneRegistrations = new WeakMap<HTMLElement, DndRegistration>();
  let pendingSourceOrder: string | null = null;
  let pendingDropTimeout: ReturnType<typeof setTimeout> | null = null;

  const draggedSegment = computed(() => draggedId.value ? getSourceSegments().find((segment) => segment.id === draggedId.value) ?? null : null);
  const updateDragPointer = (input: { clientX: number; clientY: number }) => {
    dragPointer.value = { x: input.clientX, y: input.clientY };
  };
  const clearDragVisualState = () => {
    draggedId.value = null;
    dropTargetId.value = null;
    dropEdge.value = null;
    dragPointer.value = null;
  };
  const clearPendingDrop = () => {
    pendingSourceOrder = null;
    if (pendingDropTimeout !== null) clearTimeout(pendingDropTimeout);
    pendingDropTimeout = null;
  };
  const clearDragState = () => {
    clearDragVisualState();
    clearPendingDrop();
  };
  const commitDrop = (segmentId: string, targetId: string, edge: DropEdge) => {
    const order = [...getSourceSegments()].sort((a, b) => a.order - b.order).map((segment) => segment.id);
    const currentOrder = order.join("\u0000");
    const fromIndex = order.indexOf(segmentId);
    if (fromIndex < 0) return;
    order.splice(fromIndex, 1);
    const targetIndex = order.indexOf(targetId);
    if (targetIndex < 0) return;
    order.splice(edge === "after" ? targetIndex + 1 : targetIndex, 0, segmentId);
    pendingSourceOrder = order.join("\u0000");
    if (pendingSourceOrder === currentOrder) { clearDragState(); return; }
    pendingDropTimeout = setTimeout(clearPendingDrop, 5000);
    onReorder(order);
  };
  const getNearestDropTarget = (dropZone: HTMLElement, sourceId: string, clientY: number) => {
    const sourceIds = new Set(getSourceSegments().map((segment) => segment.id));
    const candidates = [...dropZone.querySelectorAll<HTMLElement>("[data-segment-port]")]
      .map((element) => {
        const targetId = element.dataset.segmentPort;
        if (!targetId || !sourceIds.has(targetId)) return null;
        const bounds = element.getBoundingClientRect();
        return { targetId, bounds, distance: Math.abs(clientY - (bounds.top + bounds.height / 2)) };
      })
      .filter((candidate): candidate is NonNullable<typeof candidate> => candidate !== null)
      .sort((a, b) => a.distance - b.distance);
    const nearest = candidates[0];
    if (!nearest) return { kind: "jueming-source-drop-zone", sourceId, targetId: "", edge: "before" as DropEdge };
    return {
      kind: "jueming-source-drop-zone",
      sourceId,
      targetId: nearest.targetId,
      edge: clientY < nearest.bounds.top + nearest.bounds.height / 2 ? "before" as DropEdge : "after" as DropEdge,
    };
  };
  const updateDropFeedback = (data: Record<string, unknown>) => {
    const sourceId = typeof data.sourceId === "string" ? data.sourceId : "";
    const targetId = typeof data.targetId === "string" ? data.targetId : "";
    if (!targetId || targetId === sourceId) {
      dropTargetId.value = null;
      dropEdge.value = null;
      return;
    }
    dropTargetId.value = targetId;
    dropEdge.value = data.edge === "after" ? "after" : "before";
  };
  const bindOrderDnd = (element: HTMLElement, value: OrderDirectiveValue) => {
    const { segmentId } = value;
    const key = `${value.mode}:${value.writable}:${segmentId}`;
    const existing = dndRegistrations.get(element);
    if (existing?.key === key) return;
    existing?.cleanup();
    if (value.mode !== "order" || !value.writable) { dndRegistrations.delete(element); return; }
    const cleanup = draggable({
        element,
        dragHandle: element.querySelector(".drag-handle") ?? undefined,
        getInitialData: () => ({ kind: "jueming-source-segment", segmentId }),
        onDragStart: ({ location }) => {
          clearDragState();
          draggedId.value = segmentId;
          updateDragPointer(location.initial.input);
        },
        onDrag: ({ location }) => updateDragPointer(location.current.input),
        // The persisted order can take a round trip through the Rust kernel.
        // End the native drag visuals as soon as the pointer is released so a
        // slow or rejected write cannot leave an inert overlay over the UI.
        onDrop: () => requestAnimationFrame(clearDragVisualState),
      });
    dndRegistrations.set(element, { key, cleanup });
  };
  const bindOrderDropZone = (element: HTMLElement, value: OrderDropZoneDirectiveValue) => {
    const key = `${value.mode}:${value.writable}`;
    const existing = dropZoneRegistrations.get(element);
    if (existing?.key === key) return;
    existing?.cleanup();
    if (value.mode !== "order" || !value.writable) { dropZoneRegistrations.delete(element); return; }
    const cleanup = dropTargetForElements({
      element,
      canDrop: ({ source }) => source.data.kind === "jueming-source-segment",
      getDropEffect: () => "move",
      getData: ({ input, source }) => getNearestDropTarget(
        element,
        typeof source.data.segmentId === "string" ? source.data.segmentId : "",
        input.clientY,
      ),
      onDragEnter: ({ self }) => updateDropFeedback(self.data),
      onDrag: ({ self }) => updateDropFeedback(self.data),
      onDragLeave: () => { dropTargetId.value = null; dropEdge.value = null; },
      onDrop: ({ source, self }) => {
        const sourceId = source.data.segmentId;
        const targetId = self.data.targetId;
        if (typeof sourceId === "string" && typeof targetId === "string" && sourceId !== targetId) {
          commitDrop(sourceId, targetId, self.data.edge === "after" ? "after" : "before");
        }
      },
    });
    dropZoneRegistrations.set(element, { key, cleanup });
  };
  const vOrderDnd: ObjectDirective<HTMLElement, OrderDirectiveValue> = {
    mounted: (element, binding: DirectiveBinding<OrderDirectiveValue>) => bindOrderDnd(element, binding.value),
    updated: (element, binding: DirectiveBinding<OrderDirectiveValue>) => bindOrderDnd(element, binding.value),
    unmounted: (element) => { dndRegistrations.get(element)?.cleanup(); dndRegistrations.delete(element); },
  };
  const vOrderDropZone: ObjectDirective<HTMLElement, OrderDropZoneDirectiveValue> = {
    mounted: (element, binding: DirectiveBinding<OrderDropZoneDirectiveValue>) => bindOrderDropZone(element, binding.value),
    updated: (element, binding: DirectiveBinding<OrderDropZoneDirectiveValue>) => bindOrderDropZone(element, binding.value),
    unmounted: (element) => { dropZoneRegistrations.get(element)?.cleanup(); dropZoneRegistrations.delete(element); },
  };

  watch(() => [...getSourceSegments()].sort((a, b) => a.order - b.order).map((segment) => segment.id).join("\u0000"), async (currentOrder) => {
    if (pendingSourceOrder === null || currentOrder !== pendingSourceOrder) return;
    await nextTick();
    await afterReorderApplied?.();
    clearPendingDrop();
  });

  onBeforeUnmount(clearDragState);
  return { draggedId, dropTargetId, dropEdge, dragPointer, draggedSegment, vOrderDnd, vOrderDropZone };
};
