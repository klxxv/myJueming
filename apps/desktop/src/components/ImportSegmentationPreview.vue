<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { ArrowRight, ChevronLeft, ChevronRight, RefreshCw } from "@lucide/vue";
import { encodingLabels } from "../domain/import-encoding";
import type { ImportPreviewResponse, SegmentPreview } from "../domain/kernel-client";
import { t } from "../i18n";

const props = defineProps<{ preview: ImportPreviewResponse | null; loading: boolean; error: string | null }>();
const emit = defineEmits<{ retry: [] }>();
const page = ref(0);
const rowsContainer = ref<HTMLElement | null>(null);
const pageSize = 20;
const pageCount = computed(() => Math.max(1, Math.ceil((props.preview?.preview.segments.length ?? 0) / pageSize)));
const rows = computed(() => props.preview?.preview.segments.slice(page.value * pageSize, (page.value + 1) * pageSize) ?? []);
watch(() => props.preview, () => { page.value = 0; });
watch([page, () => props.preview], () => { if (rowsContainer.value) rowsContainer.value.scrollTop = 0; }, { flush: "post" });
const detectionKeys = {
  manual: "npDetectionManual",
  bom: "npDetectionBom",
  utf8: "npDetectionUtf8",
  statistical: "npDetectionStatistical",
  unicode_text: "npDetectionUnicode",
} as const satisfies Record<ImportPreviewResponse["encoding_detection"], string>;
const cleanupKeys = {
  seg_wrappers: "npCleanupSeg",
  pos_suffixes_and_whitespace: "npCleanupPos",
  cjk_spaces: "npCleanupCjk",
  punctuation_spaces: "npCleanupPunctuation",
} as const satisfies Record<SegmentPreview["cleanups"][number], string>;
function detectionLabel(detection: ImportPreviewResponse["encoding_detection"]) {
  return t(detectionKeys[detection]);
}
function cleanupLabel(cleanup: SegmentPreview["cleanups"][number]) {
  return t(cleanupKeys[cleanup], cleanup === "seg_wrappers" ? { p0: "<seg> / </seg>" } : {});
}
function reason(segment: SegmentPreview) {
  if (segment.boundary === "sentence_punctuation") return t("npReasonPunctuation", { p0: segment.boundary_marker });
  if (segment.boundary === "text_end") return t("npReasonTextEnd");
  return t("npReasonLine");
}
function previewWarning(warning: string) {
  return warning === "文本未产生可用 Segment" ? t("npNoSegments") : warning;
}
</script>

<template>
  <section class="import-preview" :aria-label="t('npPreviewAria')" :aria-busy="loading">
    <div v-if="loading" class="preview-message" role="status">{{ t("npPreviewLoading") }}</div>
    <div v-else-if="error" class="preview-message preview-message--error" role="alert"><p>{{ error }}</p><button type="button" @click="emit('retry')"><RefreshCw :size="14" />{{ t("npRetryPreview") }}</button></div>
    <div v-else-if="!preview" class="preview-message">{{ t("npPreviewEmpty") }}</div>
    <template v-else>
      <div class="preview-summary" role="status"><strong>{{ t("npPreviewCount", { p0: preview.preview.segments.length }) }}</strong><span>{{ preview.label }}</span><small v-if="preview.had_bom">{{ t("npBomDetected") }}</small></div>
      <p class="preview-encoding" role="status" :aria-label="t('npDecodeResult')">
        <strong>{{ t("npEncodingSummary", { p0: t(preview.encoding_detection === "manual" ? "npUsingEncoding" : "npDetectedEncoding"), p1: encodingLabels[preview.profile.encoding] }) }}</strong>
        <span>{{ detectionLabel(preview.encoding_detection) }}</span>
      </p>
      <p v-if="preview.encoding_detection === 'statistical'" class="preview-warning">{{ t("npStatisticalWarning") }}</p>
      <p v-for="warning in preview.preview.warnings" :key="warning" class="preview-warning" role="status">{{ previewWarning(warning) }}</p>
      <div v-if="!preview.preview.segments.length" class="preview-message">{{ t("npNoSegments") }}</div>
      <template v-else>
        <div class="preview-columns" aria-hidden="true"><span>{{ t("npOriginalFragment") }}</span><span></span><span>{{ t("npSegmentedResult") }}</span></div>
        <div ref="rowsContainer" class="preview-rows">
          <article v-for="segment in rows" :key="segment.ordinal" class="preview-row">
            <header><strong>{{ t("npSegmentOrdinal", { p0: segment.ordinal + 1 }) }}</strong><span>{{ reason(segment) }}</span></header>
            <div class="preview-comparison"><pre>{{ segment.original_text }}</pre><ArrowRight :size="16" aria-hidden="true" /><pre class="preview-result">{{ segment.content }}</pre></div>
            <ul v-if="segment.cleanups.length" class="preview-cleanups" :aria-label="t('npCleanupAria')"><li v-for="cleanup in segment.cleanups" :key="cleanup">{{ cleanupLabel(cleanup) }}</li></ul>
            <p v-else class="preview-unchanged">{{ t("npUnchanged") }}</p>
          </article>
        </div>
        <footer class="preview-pagination"><span>{{ t("npPagination", { p0: page * pageSize + 1, p1: Math.min((page + 1) * pageSize, preview.preview.segments.length), p2: preview.preview.segments.length }) }}</span><button type="button" :aria-label="t('npPreviousPreview')" :disabled="page === 0" @click="page--"><ChevronLeft :size="15" /></button><span>{{ t("npPageNumber", { p0: page + 1, p1: pageCount }) }}</span><button type="button" :aria-label="t('npNextPreview')" :disabled="page + 1 >= pageCount" @click="page++"><ChevronRight :size="15" /></button></footer>
      </template>
    </template>
  </section>
</template>

<style scoped>
.preview-encoding { display: flex; align-items: baseline; flex-wrap: wrap; gap: 6px 12px; margin: 0; padding: 10px 14px; font-size: var(--jm-font-size-callout); }
.preview-encoding span { @apply text-ink-500; }
.import-preview { min-width: 0; border: 1px solid var(--line); border-radius: 8px; overflow: hidden; @apply bg-raised; }
.preview-summary { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; padding: 12px 14px; border-bottom: 1px solid var(--line); }
.preview-summary strong { @apply text-accent-strong; }.preview-summary span { min-width: 0; overflow-wrap: anywhere; @apply text-ink-700; }.preview-summary small { @apply text-ink-500; }
.preview-message { display: flex; min-height: 180px; flex-direction: column; align-items: center; justify-content: center; gap: 12px; padding: 24px; text-align: center; @apply text-ink-500; }.preview-message p { margin: 0; overflow-wrap: anywhere; }.preview-message--error { color: var(--text-danger, #a24c4c); @apply bg-danger-soft; }
button { display: inline-flex; min-width: 30px; min-height: 30px; align-items: center; justify-content: center; gap: 6px; padding: 4px 9px; border: 1px solid var(--line); border-radius: 6px; @apply text-ink-700 bg-raised; cursor: pointer; }
.preview-warning { margin: 0; padding: 10px 14px; @apply text-ink-700 bg-warm-soft; line-height: 1.5; }
.preview-columns, .preview-comparison { display: grid; grid-template-columns: minmax(0, 1fr) 20px minmax(0, 1fr); gap: 10px; }.preview-columns { padding: 9px 14px; @apply text-ink-500 bg-subtle; font-size: var(--jm-font-size-callout); }
.preview-rows { max-height: 330px; overflow: auto; overscroll-behavior: contain; }.preview-row { padding: 12px 14px; border-top: 1px solid var(--line); }.preview-row header { display: flex; flex-wrap: wrap; gap: 8px 12px; align-items: baseline; margin-bottom: 9px; }.preview-row header strong { @apply text-accent-strong; white-space: nowrap; }.preview-row header span { @apply text-ink-700; font-size: var(--jm-font-size-callout); line-height: 1.5; }
.preview-comparison { align-items: center; }.preview-comparison svg { @apply text-ink-500; }.preview-comparison pre { align-self: stretch; max-height: 160px; overflow: auto; margin: 0; padding: 10px; border-radius: 5px; @apply bg-subtle text-ink-700; font: inherit; line-height: 1.65; white-space: pre-wrap; overflow-wrap: anywhere; }.preview-comparison .preview-result { @apply text-ink-900 bg-green-soft; }
.preview-cleanups { display: flex; flex-wrap: wrap; gap: 6px; margin: 9px 0 0; padding: 0; list-style: none; }.preview-cleanups li { padding: 3px 6px; border-radius: 4px; @apply text-accent-strong bg-green-soft; font-size: var(--jm-font-size-subheadline); line-height: 1.5; }.preview-unchanged { margin: 8px 0 0; @apply text-ink-500; font-size: var(--jm-font-size-subheadline); }
.preview-pagination { display: flex; align-items: center; gap: 10px; padding: 10px 14px; border-top: 1px solid var(--line); @apply text-ink-500; font-size: var(--jm-font-size-callout); }.preview-pagination > span:first-child { margin-right: auto; }
@media (max-width: 650px) { .preview-columns { display: none; }.preview-comparison { grid-template-columns: minmax(0, 1fr); }.preview-comparison svg { transform: rotate(90deg); justify-self: center; }.preview-pagination { flex-wrap: wrap; } }
</style>
