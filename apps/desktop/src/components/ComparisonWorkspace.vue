<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, shallowRef, watch } from 'vue';
import { ArrowDown, ArrowUp, Search, X } from '@lucide/vue';
import { useTransientSegmentJump } from '../composables/useTransientSegmentJump';
import type { SegmentText, WorkspaceProject } from '../domain/kernel-client';
import { buildComparisonBands, comparisonDocuments, layoutComparisonBand } from '../domain/comparison-projection';
import WorkspacePanelStrip from './workspace-panels/WorkspacePanelStrip.vue';
import AlignedDocumentList, { type ComparisonPosition } from './parallel-workspace/AlignedDocumentList.vue';
import { t } from '../i18n';
import { formatError, rawErrorMessage } from '../i18n/kernel-messages';

const props = defineProps<{ project: WorkspaceProject; texts: ReadonlyMap<string, SegmentText>; writable: boolean; findInView?: (query: string, cancelled: () => boolean) => Promise<string[]> }>();
const emit = defineEmits<{ scrollProgress: [ratio: number]; visibleSegments: [ids: string[]]; editPair: [documentId: string, alignmentId?: string]; unlink: [id: string] }>();
const viewport = ref<HTMLElement | null>(null);
const top = ref(0);
const height = ref(600);
const selected = ref('');
const widths = shallowRef<Record<string, number>>({});
const measured = shallowRef(new Map<string, number>());
const documents = computed(() => comparisonDocuments(props.project, props.texts));
const bands = computed(() => buildComparisonBands(documents.value, props.project.alignments));
const panels = computed(() => documents.value.map(({ document, segments }, index) => ({
  id: document.document_id,
  title: document.title,
  subtitle: t('puiComparisonSubtitle', {
    p0: index === 0 ? t('puiDefaultSourceTitle') : t('puiTranslationRole', { p0: index }),
    p1: document.language_id,
    p2: segments.length,
  }),
})));
const documentBySegment = computed(() => new Map(documents.value.flatMap(doc => doc.segments.map(segment => [segment.id, doc.document] as const))));
const labels = computed(() => Object.fromEntries(props.project.alignments.map((relation) => {
  const document = documentBySegment.value.get(relation.target_segment_ids[0]);
  return [relation.alignment_id, `T${props.project.documents.findIndex(doc => doc.document_id === document?.document_id)}`];
})));
const positions = computed<ComparisonPosition[]>(() => {
  let offset = 0;
  return bands.value.map(band => {
    const segmentHeights = new Map<string, number>();
    for (const [id, segments] of band.cells) {
      const width = widths.value[id] ?? 360;
      for (const segment of segments) {
        const estimate = 38 + Math.max(76, 36 + Math.max(1, Math.ceil((segment.contentLength ?? segment.text.length) / Math.max(12, Math.floor((width - 142) / (segment.side === 'source' ? 16 : 8.2))))) * 26);
        segmentHeights.set(segment.id, measured.value.get(`${id}:${segment.id}`) ?? estimate);
      }
    }
    const layout = layoutComparisonBand(band, segmentHeights);
    const position = { band, layout, top: offset, height: layout.height }; offset += layout.height + 12; return position;
  });
});
const totalHeight = computed(() => { const last = positions.value[positions.value.length - 1]; return last ? last.top + last.height : 0; });
const visible = computed(() => positions.value.filter(position => position.top + position.height >= top.value - 720 && position.top <= top.value + height.value + 720));
watch(() => [props.project.project.current_revision_id, visible.value.flatMap(position => [...position.layout.segments].filter(([, segment]) => position.top + segment.top + segment.height >= top.value - 720 && position.top + segment.top <= top.value + height.value + 720).map(([id]) => id)).join(',')], ([, ids]) => emit('visibleSegments', ids ? ids.split(',') : []), { immediate: true });
const report = (id: string, key: string, value: number) => {
  const cacheKey = `${id}:${key}`;
  if (!Number.isFinite(value) || value <= 0 || measured.value.get(cacheKey) === value) return;
  const next = new Map(measured.value); next.set(cacheKey, value); measured.value = next;
};
watch(() => `${props.project.project.project_id}:${props.project.project.current_revision_id}`, () => { measured.value = new Map(); if (!props.project.alignments.some(relation => relation.alignment_id === selected.value)) selected.value = ''; });
watch(widths, (next, previous) => {
  const changed = Object.keys(next).filter(id => next[id] !== previous[id]);
  measured.value = new Map([...measured.value].filter(([key]) => !changed.some(id => key.startsWith(`${id}:`))));
});
watch(positions, (next, previous) => {
  const element = viewport.value; if (!element || element.scrollTop <= 0) return;
  const band = previous.find(position => position.top + position.height >= element.scrollTop);
  if (!band) return;
  let anchor: { id: string; top: number } | undefined;
  for (const [id, segment] of band.layout.segments) {
    const segmentTop = band.top + segment.top;
    if (segmentTop <= element.scrollTop && (!anchor || segmentTop > anchor.top)) anchor = { id, top: segmentTop };
  }
  if (!anchor) return;
  const updatedBand = next.find(position => position.layout.segments.has(anchor!.id));
  const updatedSegment = updatedBand?.layout.segments.get(anchor.id);
  if (updatedBand && updatedSegment) {
    const difference = updatedBand.top + updatedSegment.top - anchor.top;
    if (Math.abs(difference) >= .5) { element.scrollTop += difference; top.value = element.scrollTop; }
  }
}, { flush: 'post' });
let observer: ResizeObserver | undefined;
watch(viewport, element => { observer?.disconnect(); if (element) { const measure = () => { height.value = element.clientHeight; }; observer = new ResizeObserver(measure); observer.observe(element); measure(); } }, { flush: 'post' });
onBeforeUnmount(() => observer?.disconnect());
const findOpen = ref(false), findQuery = ref(''), findPending = ref(false), findError = ref('');
const findInput = ref<HTMLInputElement | null>(null);
const findMatches = ref<string[]>([]), findCursor = ref(-1);
let findGeneration = 0, findTimer: ReturnType<typeof setTimeout> | undefined;
const { highlightedSegmentId, jumpToSegment } = useTransientSegmentJump({ focus: async (id) => {
  const position = positions.value.find(position => [...position.band.cells.values()].some(segments => segments.some(segment => segment.id === id)));
  const element = viewport.value;
  if (!position || !element) return false;
  element.scrollTop = Math.max(0, position.top + (position.layout.segments.get(id)?.top ?? 0) - 60);
  top.value = element.scrollTop;
  const doc = documentBySegment.value.get(id);
  const panel = doc && element.querySelector<HTMLElement>(`[data-panel-id="${CSS.escape(doc.document_id)}"]`);
  if (panel) { const panelLeft = (panel.getBoundingClientRect().left - element.getBoundingClientRect().left) / (element.getBoundingClientRect().width / element.offsetWidth) + element.scrollLeft; if (panelLeft < element.scrollLeft || panelLeft + panel.offsetWidth > element.scrollLeft + element.clientWidth) element.scrollLeft = panelLeft; }
  await nextTick(); return true;
} });
const openFind = async () => { findOpen.value = true; await nextTick(); findInput.value?.focus(); };
const navigateFind = async (direction: number) => {
  if (!findMatches.value.length) return;
  findCursor.value = (findCursor.value + direction + findMatches.value.length) % findMatches.value.length;
  await jumpToSegment(findMatches.value[findCursor.value]);
};
watch([findQuery, () => props.project.project.current_revision_id], () => {
  const generation = ++findGeneration; clearTimeout(findTimer); findMatches.value = []; findCursor.value = -1; findError.value = '';
  const query = findQuery.value.trim().toLocaleLowerCase(); findPending.value = !!query;
  if (!query) return;
  findTimer = setTimeout(async () => {
    try {
      const matches = props.findInView ? await props.findInView(query, () => generation !== findGeneration)
        : documents.value.flatMap(doc => doc.segments.filter(segment => segment.text.toLocaleLowerCase().includes(query)).map(segment => segment.id));
      if (generation !== findGeneration) return;
      findMatches.value = matches; await navigateFind(1);
    } catch (error) { if (generation === findGeneration) findError.value = rawErrorMessage(error); }
    finally { if (generation === findGeneration) findPending.value = false; }
  }, 180);
});
onBeforeUnmount(() => { findGeneration++; clearTimeout(findTimer); });
watch([top, totalHeight, height], () => emit('scrollProgress', Math.min(1, Math.max(0, top.value / Math.max(1, totalHeight.value - height.value)))), { immediate: true, flush: 'post' });
const scrollToProgress = (ratio: number) => {
  const element = viewport.value;
  if (!element) return;
  element.scrollTo({ top: Math.max(0, Math.min(1, ratio)) * Math.max(0, totalHeight.value - element.clientHeight), behavior: 'instant' });
  top.value = element.scrollTop;
};
defineExpose({ openFind, navigateFind, scrollToProgress });
const openRelation = (id: string) => {
  const relation = props.project.alignments.find(item => item.alignment_id === id);
  const document = documents.value.find(doc => doc.segments.some(segment => segment.id === relation?.target_segment_ids[0]));
  if (document) emit('editPair', document.document.document_id, id);
};
</script>
<template>
  <section class="comparison-workspace" :aria-label="t('puiComparisonWorkspaceAria')">
    <header class="comparison-toolbar"><strong>{{ project.project.name }}</strong><span>{{ t('puiComparisonCount', { p0: documents.length - 1 }) }}</span><span class="comparison-toolbar__hint">{{ t('puiComparisonLayoutHint') }}</span><button type="button" :aria-label="t('puiFindComparisonAria')" @click="openFind"><Search :size="15" /></button><button v-if="selected" type="button" @click="openRelation(selected)">{{ t('puiEditSelectedBinding') }}</button></header>
    <div class="comparison-pairs"><span>{{ t('puiAlignmentEditing') }}</span><button v-for="document in documents.slice(1)" :key="document.document.document_id" type="button" @click="emit('editPair', document.document.document_id)">{{ document.document.title }}</button></div>
    <div v-if="findOpen" class="comparison-find" role="search" :aria-label="t('puiComparisonFindAria')"><Search :size="16" /><input ref="findInput" v-model="findQuery" :aria-label="t('puiFindAllTranslationsAria')" :placeholder="t('puiFindAllPlaceholder')" @keydown.enter.prevent="navigateFind($event.shiftKey ? -1 : 1)" @keydown.esc.prevent="findOpen = false" /><span>{{ findError ? t('puiFindFailed', { p0: formatError(findError) }) : findPending ? t('puiFinding') : t('puiFindPosition', { p0: findMatches.length ? findCursor + 1 : 0, p1: findMatches.length }) }}</span><button type="button" :aria-label="t('puiPreviousResult')" :disabled="findPending || !findMatches.length" @click="navigateFind(-1)"><ArrowUp :size="16" /></button><button type="button" :aria-label="t('puiNextResult')" :disabled="findPending || !findMatches.length" @click="navigateFind(1)"><ArrowDown :size="16" /></button><button type="button" :aria-label="t('puiCloseComparisonFind')" @click="findOpen = false"><X :size="16" /></button></div>
    <div ref="viewport" class="comparison-viewport" @scroll.passive="top = viewport?.scrollTop ?? 0">
      <WorkspacePanelStrip :panels="panels" :storage-key="`jueming-comparison-panels:${project.project.project_id}`" @layout="widths = $event">
        <template #default="{ panel }"><AlignedDocumentList :key="`${panel.id}:${project.project.current_revision_id}`" :document-id="panel.id" :source="panel.id === project.documents[0].document_id" :jump-highlight-segment-id="highlightedSegmentId" :window-start="top - 720" :window-end="top + height + 720" :positions="visible" :total-height="totalHeight" :selected="selected" :labels="labels" :writable="writable" @resize="(key, value) => report(panel.id, key, value)" @select="selected = $event" @edit-pair="openRelation" @unlink="emit('unlink', $event)" /></template>
      </WorkspacePanelStrip>
    </div>
  </section>
</template>
<style scoped>
.comparison-workspace { display: flex; flex-direction: column; min-height: 0; height: 100%; @apply bg-raised; }
.comparison-toolbar, .comparison-pairs { display: flex; align-items: center; gap: 12px; padding: 10px 16px; flex-wrap: wrap; border-bottom: 1px solid var(--line); }
.comparison-toolbar span, .comparison-pairs > span { color: var(--text-muted); font-size: 12px; }
.comparison-toolbar__hint { margin-left: auto; }
.comparison-toolbar button, .comparison-pairs button { padding: 5px 10px; border: 1px solid var(--line); border-radius: 5px; color: var(--text-main); @apply bg-subtle; cursor: pointer; }
.comparison-find { display: flex; align-items: center; gap: 10px; padding: 8px 16px; border-bottom: 1px solid var(--line); @apply bg-subtle; }
.comparison-find input { flex: 1; min-width: 100px; padding: 6px 10px; border: 1px solid var(--line); border-radius: 5px; @apply bg-raised; color: var(--text-main); }
.comparison-find button { display: grid; place-items: center; border: 0; padding: 4px; color: var(--text-main); background: transparent; cursor: pointer; }
.comparison-find span { font-size: 12px; color: var(--text-muted); }
.comparison-viewport { flex: 1; min-height: 0; overflow: auto; overflow-anchor: none; overscroll-behavior: contain; scrollbar-gutter: stable; }
</style>
