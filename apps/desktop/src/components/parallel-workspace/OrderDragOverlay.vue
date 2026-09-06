<script setup lang="ts">
import { GripVertical } from "@lucide/vue";
import type { SegmentDto } from "../../domain/kernel-client";
import type { DragPointer } from "../../composables/useOrderDragAndDrop";

defineProps<{
  draggedSegment: SegmentDto | null;
  dragPointer: DragPointer | null;
}>();
</script>

<template>
  <Teleport to="body">
    <div v-if="draggedSegment && dragPointer" class="drag-overlay" :style="{ left: `${dragPointer.x}px`, top: `${dragPointer.y}px` }" aria-hidden="true">
      <GripVertical :size="15" /><span>{{ draggedSegment.text }}</span><i></i>
    </div>
  </Teleport>
</template>

<style scoped>
.drag-overlay { position: fixed; z-index: 43; display: flex; align-items: center; gap: 7px; width: min(250px, calc(100vw - 32px)); padding: 11px 13px; transform: translate(-20px, -50%); border: 1px solid #79ad7e; border-radius: 7px; color: var(--ink-900); background: var(--surface-raised); box-shadow: 0 12px 28px rgb(24 49 29 / 24%); pointer-events: none; }
.drag-overlay span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }
.drag-overlay i { width: 7px; height: 7px; margin-left: auto; border-radius: 50%; background: var(--green-700); }
</style>
