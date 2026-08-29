<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";
import type {
  DropEdge,
  RegisterOrderDropTarget,
} from "../../composables/useOrderDragAndDrop";
import type { LanguageSide } from "../../domain/kernel-client";

const props = defineProps<{
  side: LanguageSide;
  targetId: string;
  edge: DropEdge;
  registerOrderDropTarget: RegisterOrderDropTarget;
}>();

const rootRef = ref<HTMLElement | null>(null);
let cleanupDropTarget: (() => void) | null = null;

watch(
  [rootRef, () => props.targetId, () => props.side, () => props.edge, () => props.registerOrderDropTarget],
  ([element]) => {
    cleanupDropTarget?.();
    cleanupDropTarget = null;
    if (!(element instanceof HTMLElement)) return;
    cleanupDropTarget = props.registerOrderDropTarget(element, props.targetId, props.side, props.edge);
  },
  { immediate: true, flush: "post" },
);

onBeforeUnmount(() => cleanupDropTarget?.());
</script>

<template>
  <div
    ref="rootRef"
    class="order-drop-zone"
    :data-order-drop-side="side"
    :data-order-drop-target="targetId"
    :data-order-drop-edge="edge"
    aria-hidden="true"
  ></div>
</template>

<style scoped>
.order-drop-zone { position: absolute; z-index: 2; right: 0; left: 0; min-height: 1px; background: transparent; }
</style>
