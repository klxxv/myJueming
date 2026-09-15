<script setup lang="ts">
import { computed, ref } from 'vue';
import { t } from '../../i18n';
const props = defineProps<{ scrollTop: number; viewportHeight: number; totalHeight: number; markers: { id: string; top: number; height: number }[] }>();
const emit = defineEmits<{ scroll: [ratio: number]; jump: [id: string]; dragging: [active: boolean] }>();
const track = ref<HTMLElement | null>(null);
const drag = ref<{ pointer: number; offset: number } | null>(null);
const fraction = computed(() => Math.min(1, props.viewportHeight / Math.max(1, props.totalHeight)));
const thumbSize = computed(() => Math.max(.05, fraction.value));
const ratio = computed(() => Math.min(1, Math.max(0, props.scrollTop / Math.max(1, props.totalHeight - props.viewportHeight))));
const ranges = computed(() => props.markers.map(marker => {
  const center = (marker.top + marker.height / 2) / Math.max(1, props.totalHeight);
  const size = Math.min(1, Math.max(.05, marker.height / Math.max(1, props.totalHeight)));
  const start = Math.max(0, Math.min(1 - size, center - size / 2));
  return { ...marker, center, start, end: start + size };
}));
const background = computed(() => {
  const merged: { start: number; end: number }[] = [];
  for (const range of ranges.value) {
    const previous = merged[merged.length - 1];
    if (previous && range.start <= previous.end) previous.end = Math.max(previous.end, range.end);
    else merged.push({ start: range.start, end: range.end });
  }
  const stops = ['transparent 0%'];
  for (const range of merged) stops.push(`transparent ${range.start * 100}%`, `var(--scrollbar-unlinked) ${range.start * 100}% ${range.end * 100}%`, `transparent ${range.end * 100}%`);
  stops.push('transparent 100%');
  return `linear-gradient(to bottom, ${stops.join(', ')})`;
});
function pointerRatio(event: PointerEvent) {
  const rect = track.value!.getBoundingClientRect();
  return Math.max(0, Math.min(1, (event.clientY - rect.top) / Math.max(1, rect.height)));
}
function startThumb(event: PointerEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  track.value?.focus({ preventScroll: true });
  track.value?.setPointerCapture(event.pointerId);
  drag.value = { pointer: event.pointerId, offset: pointerRatio(event) - ratio.value * (1 - thumbSize.value) };
  emit('dragging', true);
}
function move(event: PointerEvent) {
  if (!drag.value || drag.value.pointer !== event.pointerId) return;
  emit('scroll', Math.max(0, Math.min(1, (pointerRatio(event) - drag.value.offset) / Math.max(.001, 1 - thumbSize.value))));
}
function finish(event: PointerEvent) {
  if (!drag.value) return;
  move(event);
  drag.value = null;
  emit('dragging', false);
  if (track.value?.hasPointerCapture(event.pointerId)) track.value.releasePointerCapture(event.pointerId);
}
function cancel() { drag.value = null; emit('dragging', false); }
function clickTrack(event: PointerEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  track.value?.focus({ preventScroll: true });
  const position = pointerRatio(event);
  const marker = ranges.value.filter(range => range.start <= position && position <= range.end)
    .sort((a, b) => Math.abs(a.center - position) - Math.abs(b.center - position))[0];
  if (marker) emit('jump', marker.id);
  else emit('scroll', Math.max(0, Math.min(1, (position - thumbSize.value / 2) / Math.max(.001, 1 - thumbSize.value))));
}
function key(event: KeyboardEvent) {
  const step = Math.max(40, props.viewportHeight * .1) / Math.max(1, props.totalHeight - props.viewportHeight);
  const value = event.key === 'Home' ? 0 : event.key === 'End' ? 1 : event.key === 'ArrowDown' ? ratio.value + step : event.key === 'ArrowUp' ? ratio.value - step : event.key === 'PageDown' ? ratio.value + step * 9 : event.key === 'PageUp' ? ratio.value - step * 9 : null;
  if (value === null) return;
  event.preventDefault(); emit('scroll', Math.max(0, Math.min(1, value)));
}
</script>
<template>
  <div ref="track" class="workspace-scrollbar" role="scrollbar" tabindex="0" aria-orientation="vertical" :aria-label="t('shellFooterProgressAria')" :aria-valuemin="0" :aria-valuemax="100" :aria-valuenow="Math.round(ratio * 100)" @pointerdown="clickTrack" @pointermove="move" @pointerup="finish" @pointercancel="cancel" @lostpointercapture="cancel" @keydown="key">
    <div class="workspace-scrollbar__markers" :style="{ background }" aria-hidden="true" />
    <div class="workspace-scrollbar__thumb" :style="{ top: `${ratio * (1 - thumbSize) * 100}%`, height: `${thumbSize * 100}%` }" @pointerdown.stop="startThumb" aria-hidden="true" />
  </div>
</template>
<style scoped>
.workspace-scrollbar { --scrollbar-unlinked: var(--ink-500); position: relative; flex: 0 0 28px; min-height: 0; background: var(--surface-muted, var(--line)); border-left: 1px solid var(--line); touch-action: none; user-select: none; cursor: pointer; }
.workspace-scrollbar__markers { position: absolute; inset: 0 15px 0 2px; pointer-events: none; opacity: .9; }
.workspace-scrollbar__thumb { position: absolute; right: 2px; width: 12px; box-sizing: border-box; background: rgb(245 245 245 / 65%); border: 1px solid rgb(110 115 120 / 65%); border-radius: 3px; cursor: ns-resize; }
.workspace-scrollbar:focus-visible { outline: 2px solid var(--green-700); outline-offset: -2px; }
</style>
