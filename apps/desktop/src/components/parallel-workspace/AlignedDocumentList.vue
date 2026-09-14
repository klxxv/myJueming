<script setup lang="ts">
import type { CoreAlignment } from '../../domain/kernel-client';
import type { ComparisonBand, ComparisonBandLayout } from '../../domain/comparison-projection';
import ComparisonListCell from './ComparisonListCell.vue';
export interface ComparisonPosition { band: ComparisonBand; layout: ComparisonBandLayout; top: number; height: number }
const props = defineProps<{ documentId: string; source: boolean; positions: ComparisonPosition[]; totalHeight: number; selected: string; labels: Record<string, string>; writable: boolean; windowStart: number; windowEnd: number; jumpHighlightSegmentId?: string }>();
const emit = defineEmits<{ resize: [id: string, height: number]; select: [id: string]; editPair: [id: string]; unlink: [id: string] }>();
const relations = (band: ComparisonBand): CoreAlignment[] => {
  if (props.source) return band.relations;
  const ids = new Set(band.cells.get(props.documentId)?.map(segment => segment.id));
  return band.relations.filter(relation => ids.has(relation.target_segment_ids[0]));
};
</script>
<template>
  <div class="aligned-document-list" role="list" :style="{ height: `${totalHeight}px` }">
    <div v-for="position in positions" :key="position.band.key" class="aligned-document-list__band" role="listitem" :data-band-id="position.band.key" :style="{ top: `${position.top}px`, minHeight: `${position.height}px` }">
      <ComparisonListCell v-if="position.band.cells.get(documentId)?.length" :layout="position.layout" :window-start="windowStart - position.top" :window-end="windowEnd - position.top" :segments="position.band.cells.get(documentId)!" :relations="relations(position.band)" :source="source" :jump-highlight-segment-id="jumpHighlightSegmentId" :selected="selected" :labels="labels" :writable="writable" @resize="(id, height) => emit('resize', id, height)" @select="emit('select', $event)" @edit-pair="emit('editPair', $event)" @unlink="emit('unlink', $event)" />
    </div>
  </div>
</template>
<style scoped>
.aligned-document-list { position: relative; min-height: 100%; }
.aligned-document-list__band { position: absolute; top: 0; left: 0; right: 0; }
</style>
