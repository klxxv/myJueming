<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { AlignmentDto, SegmentDto } from "../domain/kernel-client";
import { t } from "../i18n";

const props = defineProps<{ source: SegmentDto[]; target: SegmentDto[]; alignments: AlignmentDto[]; scrollProgress: number }>();
const emit = defineEmits<{ navigate: [ratio: number] }>();
const cursorRatio = ref(0);
const positions = computed(() => [props.source, props.target].map(rows => Math.round(cursorRatio.value * Math.max(0, rows.length - 1))));
const sides = computed(() => {
  const linked = new Set(props.alignments.flatMap(alignment => [...alignment.sourceIds, ...alignment.targetIds]));
  return [props.source, props.target].map(rows => {
    const segments = [...rows].sort((a, b) => a.order - b.order);
    const states = segments.map(segment => linked.has(segment.id));
    const stops: string[] = [];
    let start = 0;
    for (let end = 1; end <= states.length; end += 1) {
      if (end < states.length && states[end] === states[start]) continue;
      stops.push(`${states[start] ? 'var(--green-700)' : 'var(--line)'} ${start / states.length * 100}% ${end / states.length * 100}%`);
      start = end;
    }
    return { segments, count: states.filter(Boolean).length, background: stops.length ? `linear-gradient(to right, ${stops.join(', ')})` : 'var(--line)' };
  });
});
const percentage = computed(() => {
  const total = props.source.length + props.target.length;
  return total ? Math.round(sides.value.reduce((sum, side) => sum + side.count, 0) / total * 100) : 0;
});
function navigate(side: number, index: number) {
  const last = sides.value[side].segments.length - 1;
  cursorRatio.value = last > 0 ? index / last : 0;
  const segment = sides.value[side]?.segments[index];
  if (segment) emit('navigate', cursorRatio.value);
}
const dragging = ref<number | null>(null);
watch(() => props.scrollProgress, value => {
  if (dragging.value === null) cursorRatio.value = Math.max(0, Math.min(1, value));
}, { immediate: true });
function updatePointer(event: PointerEvent) {
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  const scale = rect.width / Math.max(1, (event.currentTarget as HTMLElement).offsetWidth);
  const thumbWidth = 12 * scale;
  cursorRatio.value = Math.max(0, Math.min(1, (event.clientX - rect.left - thumbWidth / 2) / Math.max(1, rect.width - thumbWidth)));
}
function startDrag(event: PointerEvent, side: number) {
  if (event.button !== 0 || !sides.value[side].segments.length) return;
  event.preventDefault();
  const track = event.currentTarget as HTMLElement;
  track.focus({ preventScroll: true });
  track.setPointerCapture(event.pointerId);
  dragging.value = side;
  updatePointer(event);
}
function finishDrag(event: PointerEvent, side: number) {
  if (dragging.value !== side) return;
  updatePointer(event);
  dragging.value = null;
  (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
  const segment = sides.value[side].segments[positions.value[side]];
  if (segment) emit("navigate", cursorRatio.value);
}
const thumbLeft = computed(() => `calc(${cursorRatio.value * 100}% - ${cursorRatio.value * 12}px)`);
function keyTrack(event: KeyboardEvent, side: number) {
  const last = sides.value[side].segments.length - 1;
  if (last < 0) return;
  const current = Math.min(positions.value[side] ?? 0, last);
  const index = event.key === 'Home' ? 0 : event.key === 'End' ? last
    : ['ArrowRight', 'ArrowUp'].includes(event.key) ? Math.min(last, current + 1)
      : ['ArrowLeft', 'ArrowDown'].includes(event.key) ? Math.max(0, current - 1) : null;
  if (index === null) return;
  event.preventDefault();
  navigate(side, index);
}
</script>

<template>
  <div class="footer-progress">
    <span class="footer-processed"><span class="footer-processed-label">{{ t('shellFooterProcessed') }}</span><strong>{{ sides[0].count }}/{{ source.length }} : {{ sides[1].count }}/{{ target.length }}</strong></span>
    <span class="footer-progress-label">{{ t('shellFooterProgress') }}</span>
    <div class="progress-minimap">
      <div v-for="(side, index) in sides" :key="index" class="progress-minimap-track" role="slider"
        :tabindex="side.segments.length ? 0 : -1" :aria-disabled="!side.segments.length"
        :aria-label="`${t('shellFooterProgressAria')} · ${t(index === 0 ? 'shellSourceSegments' : 'targetSegments')}`"
        :title="t(index === 0 ? 'shellSourceSegments' : 'targetSegments')"
        :aria-valuemin="1" :aria-valuemax="Math.max(1, side.segments.length)"
        :aria-valuenow="Math.max(1, Math.min((positions[index] ?? 0) + 1, side.segments.length))"
        :style="{ background: side.background }" @pointerdown="startDrag($event, index)"
        @pointermove="dragging === index && updatePointer($event)" @pointerup="finishDrag($event, index)"
        @pointercancel="dragging = null" @lostpointercapture="dragging = null" @keydown="keyTrack($event, index)">
        <span v-if="side.segments.length" class="progress-minimap-thumb" :style="{ left: thumbLeft }" aria-hidden="true" />
      </div>
    </div>
    <strong class="footer-progress-percent">{{ percentage }}%</strong>
  </div>
</template>

<style scoped>
.progress-minimap { display: grid; gap: 3px; width: clamp(96px, 10vw, 188px); padding-block: 3px; }
.progress-minimap-track { position: relative; height: 9px; border-radius: 3px; cursor: pointer; touch-action: none; }
.progress-minimap-thumb { position: absolute; top: -2px; width: 12px; height: 13px; box-sizing: border-box; border: 1px solid #8a9097; border-radius: 3px; background: #f0f1f2; box-shadow: 0 1px 2px rgb(0 0 0 / 20%); cursor: ew-resize; }
.progress-minimap-track:focus-visible { outline: 2px solid var(--green-700); outline-offset: 2px; }
.progress-minimap-track[aria-disabled="true"] { cursor: default; }
</style>
