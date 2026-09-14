<script setup lang="ts">
import { computed, onBeforeUnmount, watch } from 'vue';
import { Link2, Link2Off } from '@lucide/vue';
import type { CoreAlignment, SegmentDto } from '../../domain/kernel-client';
import type { ComparisonBandLayout } from '../../domain/comparison-projection';
import SegmentCard from './SegmentCard.vue';
import { t } from '../../i18n';
const props = defineProps<{ segments: SegmentDto[]; layout: ComparisonBandLayout; windowStart: number; windowEnd: number; relations: CoreAlignment[]; selected: string; source: boolean; labels: Record<string, string>; writable: boolean; jumpHighlightSegmentId?: string }>();
const emit = defineEmits<{ resize: [id: string, height: number]; select: [id: string]; editPair: [id: string]; unlink: [id: string] }>();
const activeIds = computed(() => new Set(props.relations.filter(relation => relation.alignment_id === props.selected).flatMap(relation => [...relation.source_segment_ids, ...relation.target_segment_ids])));
const visibleSegments = computed(() => props.segments.filter(segment => {
  const position = props.layout.segments.get(segment.id)!;
  return position.top + position.height >= props.windowStart && position.top <= props.windowEnd;
}));
const memberships = computed(() => {
  const result = new Map<string, CoreAlignment[]>();
  for (const relation of props.relations) for (const id of props.source ? relation.source_segment_ids : relation.target_segment_ids) {
    const relations = result.get(id) ?? []; relations.push(relation); result.set(id, relations);
  }
  return result;
});
const elements = new Map<string, HTMLElement>();
let frame: number | undefined;
const report = () => {
  if (frame !== undefined) return;
  frame = requestAnimationFrame(() => { frame = undefined; for (const segment of visibleSegments.value) {
    const element = elements.get(segment.id);
    if (element && segment.loaded !== false) emit('resize', segment.id, element.offsetHeight);
  } });
};
const observer = new ResizeObserver(report);
const bind = (id: string, element: unknown) => {
  const previous = elements.get(id);
  if (previous === element) return;
  if (previous) observer.unobserve(previous);
  elements.delete(id);
  if (element instanceof HTMLElement) { elements.set(id, element); observer.observe(element); report(); }
};
watch(() => props.segments.map(segment => `${segment.id}:${segment.contentHash}:${segment.loaded}`).join(), report, { flush: 'post' });
onBeforeUnmount(() => { observer.disconnect(); if (frame !== undefined) cancelAnimationFrame(frame); });
</script>
<template>
  <article class="comparison-cell" :style="{ height: `${layout.height}px` }" :aria-label="layout.crossed ? t('puiCrossedAria') : undefined">
    <div v-for="segment in visibleSegments" :key="segment.id" :ref="element => bind(segment.id, element)" class="comparison-cell__segment" :class="{ 'comparison-cell__segment--unlinked': !memberships.get(segment.id)?.length }" :style="{ top: `${layout.segments.get(segment.id)!.top}px` }" :data-comparison-segment-id="segment.id">
      <div class="comparison-cell__relations">
        <span v-for="relation in memberships.get(segment.id) ?? []" :key="relation.alignment_id" class="comparison-binding" :class="{ 'comparison-binding--active': selected === relation.alignment_id }" :data-relation-id="relation.alignment_id">
          <button type="button" :aria-pressed="selected === relation.alignment_id" :title="t('puiViewBinding', { p0: labels[relation.alignment_id], p1: relation.alignment_id })" @click="emit('select', relation.alignment_id)" @dblclick="emit('editPair', relation.alignment_id)"><Link2 :size="14" />{{ labels[relation.alignment_id] }} · {{ relation.source_segment_ids.length }}:{{ relation.target_segment_ids.length }}</button>
          <button v-if="writable" type="button" :aria-label="t('puiUnlinkBinding', { p0: labels[relation.alignment_id] })" @click="emit('unlink', relation.alignment_id)"><Link2Off :size="13" /></button>
        </span>
        <span v-if="!memberships.get(segment.id)?.length" class="comparison-cell__pending"><Link2Off :size="14" />{{ t("puiUnaligned") }}</span>
        <span v-if="layout.crossed" class="comparison-cell__pending" :title="t('puiCrossedBindingTitle')">{{ t("puiCrossedBinding") }}</span>
      </div>
      <SegmentCard :segment="segment" :side="source ? 'source' : 'target'" :alignment-id="''" mode="review" :writable="false" :jump-highlighted="jumpHighlightSegmentId === segment.id" :fragmented="segment === segments[segments.length - 1] && segments[segments.length - 1].order - segments[0].order + 1 !== segments.length" :selected="activeIds.has(segment.id)" />
    </div>
  </article>
</template>
<style scoped>
.comparison-cell { position: relative; }
.comparison-cell__segment { position: absolute; left: 0; right: 0; min-height: 76px; background: var(--surface-green-soft); }
.comparison-cell__segment--unlinked { background: var(--surface-warm-soft); }
.comparison-cell__relations { display: flex; flex-wrap: nowrap; overflow-x: auto; align-items: center; gap: 5px; padding: 7px 12px; height: 38px; border-bottom: 1px solid var(--line); }
.comparison-binding { flex: none; display: inline-flex; max-width: 100%; overflow: hidden; border: 1px solid var(--line); border-radius: 99px; background: var(--surface-raised); color: var(--green-700); }
.comparison-binding--active { background: var(--surface-green-selected); outline: 2px solid var(--green-700); }
.comparison-binding button { display: inline-flex; align-items: center; justify-content: center; gap: 5px; padding: 4px 7px; border: 0; color: inherit; background: transparent; font-size: 11px; cursor: pointer; }
.comparison-binding button:first-child { min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
.comparison-binding button + button { border-left: 1px solid var(--line); }
.comparison-cell__pending { flex: none; display: inline-flex; gap: 5px; color: var(--text-muted); font-size: 11px; }
</style>
