<script setup lang="ts">
import { computed } from 'vue';
import type { AlignmentDto, SegmentDto } from '../domain/kernel-client';
import { t } from '../i18n';
const props = defineProps<{ source: SegmentDto[]; target: SegmentDto[]; alignments: AlignmentDto[] }>();
const counts = computed(() => {
  const linked = new Set(props.alignments.flatMap(item => [...item.sourceIds, ...item.targetIds]));
  return [props.source, props.target].map(rows => rows.filter(row => linked.has(row.id)).length);
});
</script>
<template>
  <div class="footer-progress"><span class="footer-processed"><span class="footer-processed-label">{{ t('shellFooterProcessed') }}</span><strong>{{ counts[0] }}/{{ source.length }} : {{ counts[1] }}/{{ target.length }}</strong></span></div>
</template>
