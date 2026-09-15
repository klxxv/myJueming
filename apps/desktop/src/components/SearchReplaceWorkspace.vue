<script setup lang="ts">
import { computed, ref } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import { Check, Filter, Replace, Search, X } from "@lucide/vue";
import { t } from "../i18n";

defineOptions({ name: "SearchReplaceWorkspace" });

export type SearchSide = "both" | "source" | "target";

export interface SearchResult {
  id: string;
  label?: string;
  sourceId?: string | null;
  targetId?: string | null;
  sourceText: string;
  targetText: string;
  alignmentId?: string | null;
  alignmentLabel?: string;
}

export interface SearchQueryOptions {
  query: string;
  side: SearchSide;
  regex: boolean;
  caseSensitive: boolean;
}

export interface ReplacePreview {
  /** Immutable query/replacement inputs captured when this preview was requested. */
  options: SearchQueryOptions;
  replacement: string;
  resultIds: string[];
  source: Array<{ resultId: string; before: string; after: string }>;
  target: Array<{ resultId: string; before: string; after: string }>;
}

const props = withDefaults(
  defineProps<{
    results: SearchResult[];
    query?: string;
    side?: SearchSide;
    regex?: boolean;
    caseSensitive?: boolean;
    replacement?: string;
    projectLabel?: string;
    loading?: boolean;
    /** Results already evaluated by the native Kernel; do not re-filter them in JavaScript. */
    authoritativeResults?: boolean;
    /** Immutable native replacement preview, adapted into source and target display entries by the owner. */
    nativePreview?: ReplacePreview | null;
    previewLoading?: boolean;
    previewError?: string | null;
  }>(),
  {
    query: "", side: "both", regex: false, caseSensitive: false, replacement: "", projectLabel: "", loading: false,
    authoritativeResults: false, nativePreview: null, previewLoading: false, previewError: null,
  },
);

const emit = defineEmits<{
  "update:query": [value: string];
  "update:side": [value: SearchSide];
  "update:regex": [value: boolean];
  "update:caseSensitive": [value: boolean];
  "update:replacement": [value: string];
  search: [options: SearchQueryOptions];
  "select-result": [result: SearchResult];
  /** The owner freezes its request/revision and asks the native Kernel for a preview. */
  "replace-preview": [];
  /** Always carries the currently displayed immutable preview, never live input values. */
  "apply-replace": [preview: ReplacePreview];
  reset: [];
}>();

const replaceOpen = ref(false);
const displayedProjectLabel = computed(() => props.projectLabel || t("rwCurrentProject"));

const options = computed<SearchQueryOptions>(() => ({
  query: props.query.trim(),
  side: props.side,
  regex: props.regex,
  caseSensitive: props.caseSensitive,
}));

const escaped = (value: string) => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const makePattern = (value: string) => {
  if (!value) return null;
  try {
    return new RegExp(props.regex ? value : escaped(value), props.caseSensitive ? "g" : "gi");
  } catch {
    return null;
  }
};

const matches = (result: SearchResult) => {
  const pattern = makePattern(options.value.query);
  if (!pattern) return false;
  const test = (value: string) => {
    pattern.lastIndex = 0;
    return pattern.test(value);
  };
  if (props.side === "source") return test(result.sourceText);
  if (props.side === "target") return test(result.targetText);
  return test(result.sourceText) || test(result.targetText);
};

const matchingResults = computed(() => props.authoritativeResults ? props.results : props.results.filter(matches));
const resultViewport = ref<HTMLElement | null>(null);
const resultVirtualizer = useVirtualizer<HTMLElement, HTMLButtonElement>(computed(() => ({
  count: matchingResults.value.length,
  getScrollElement: () => resultViewport.value,
  estimateSize: () => 57,
  overscan: 8,
  getItemKey: (index: number) => matchingResults.value[index]?.id ?? "missing-search-result",
})));
const virtualResults = computed(() => resultVirtualizer.value.getVirtualItems().flatMap((virtualRow) => {
  const result = matchingResults.value[virtualRow.index];
  return result ? [{ result, virtualRow }] : [];
}));

const parts = (value: string) => {
  const pattern = makePattern(options.value.query);
  if (!pattern || !value) return [{ text: value, match: false }];
  const output: Array<{ text: string; match: boolean }> = [];
  let cursor = 0;
  value.replace(pattern, (match, ...args: unknown[]) => {
    const offset = Number(args[args.length - 2]);
    if (offset > cursor) output.push({ text: value.slice(cursor, offset), match: false });
    output.push({ text: match, match: true });
    cursor = offset + match.length;
    return match;
  });
  if (cursor < value.length) output.push({ text: value.slice(cursor), match: false });
  return output.length ? output : [{ text: value, match: false }];
};

const replacementPreview = computed<ReplacePreview>(() => {
  const replacement = props.replacement;
  const pattern = makePattern(options.value.query);
  const transform = (value: string) => {
    if (!pattern) return value;
    pattern.lastIndex = 0;
    return value.replace(pattern, replacement);
  };
  return {
    options: options.value,
    replacement,
    resultIds: matchingResults.value.map((result) => result.id),
    source: matchingResults.value.map((result) => ({ resultId: result.id, before: result.sourceText, after: transform(result.sourceText) })),
    target: matchingResults.value.map((result) => ({ resultId: result.id, before: result.targetText, after: transform(result.targetText) })),
  };
});

const displayedPreview = computed<ReplacePreview | null>(() => props.authoritativeResults
  ? props.nativePreview
  : replacementPreview.value);
const previewEntries = computed(() => {
  const preview = displayedPreview.value;
  if (!preview) return [];
  return [
    ...preview.source.map(item => ({ side: t("rwSource"), item })),
    ...preview.target.map(item => ({ side: t("rwCurrentTranslation"), item })),
  ];
});
const previewReady = computed(() => Boolean(displayedPreview.value)
  && (!props.authoritativeResults || (!props.previewLoading && !props.previewError)));
const canApply = computed(() => props.authoritativeResults
  ? previewReady.value && (displayedPreview.value?.resultIds.length ?? 0) > 0
  : Boolean(props.replacement && matchingResults.value.length));

const runSearch = () => emit("search", options.value);
const selectResult = (event: MouseEvent, result: SearchResult) => {
  const selection = window.getSelection();
  const row = event.currentTarget as HTMLElement;
  if (event.detail !== 0 && selection && !selection.isCollapsed
    && (row.contains(selection.anchorNode) || row.contains(selection.focusNode))) return;
  emit("select-result", result);
};
const openReplacePreview = () => {
  replaceOpen.value = true;
  emit("replace-preview");
};
const applyDisplayedPreview = () => {
  const preview = displayedPreview.value;
  if (!preview || !canApply.value) return;
  emit("apply-replace", preview);
};
</script>

<template>
  <section class="search-replace-workspace" :aria-label="t('rwSearchReplace')">
    <form class="sr-toolbar" @submit.prevent="runSearch">
      <label class="sr-query-label" for="sr-query">{{ t('rwQuery') }}</label>
      <div class="sr-input-group">
        <Search :size="18" aria-hidden="true" />
        <input id="sr-query" :value="query" type="search" :placeholder="t('rwSearchCurrentProject')" @input="emit('update:query', ($event.target as HTMLInputElement).value)" />
        <button v-if="query" type="button" :title="t('rwClearQuery')" @click="emit('update:query', '')"><X :size="15" /></button>
      </div>
      <button class="sr-primary" type="submit" :disabled="loading"><Search :size="15" />{{ t('rwSearch') }}</button>
      <select :value="side" :aria-label="t('rwSearchScope')" @change="emit('update:side', ($event.target as HTMLSelectElement).value as SearchSide)"><option value="both">{{ t('rwAllDocuments') }}</option><option value="source">{{ t('rwSource') }}</option><option value="target">{{ t('rwCurrentTranslation') }}</option></select>
      <label class="sr-check"><input :checked="regex" type="checkbox" @change="emit('update:regex', ($event.target as HTMLInputElement).checked)" />{{ t('rwRegex') }}</label>
      <label class="sr-check"><input :checked="caseSensitive" type="checkbox" @change="emit('update:caseSensitive', ($event.target as HTMLInputElement).checked)" />{{ t('rwCaseSensitive') }}</label>
      <span class="sr-project" :title="displayedProjectLabel"><Check :size="14" /><span>{{ displayedProjectLabel }}</span></span>
    </form>

    <div class="sr-replacebar">
      <label for="sr-replacement">{{ t('rwReplaceWith') }}</label>
      <input id="sr-replacement" :value="replacement" :placeholder="t('rwReplacementPlaceholder')" @input="emit('update:replacement', ($event.target as HTMLInputElement).value)" />
      <button class="sr-secondary" type="button" :disabled="!query || !matchingResults.length || previewLoading" @click="openReplacePreview"><Replace :size="15" />{{ t('rwPreviewReplace') }}</button>
      <button class="sr-secondary" type="button" :disabled="!canApply" @click="applyDisplayedPreview">{{ t('rwApplyReplace') }}</button>
      <button class="sr-reset" type="button" @click="emit('reset')">{{ t('rwReset') }}</button>
    </div>

    <div class="sr-summary">{{ t('rwFoundResults', { count: matchingResults.length }) }}</div>
    <div ref="resultViewport" class="sr-table">
      <div class="sr-table-head"><span>ID</span><span>{{ t('rwSourceContext') }}</span><span>{{ t('rwMatch') }}</span><span>{{ t('rwTargetContext') }}</span><span>{{ t('rwAlignmentId') }}</span></div>
      <div v-if="matchingResults.length" class="sr-virtual-body" :style="{ height: `${resultVirtualizer.getTotalSize()}px` }">
        <button v-for="{ result, virtualRow } in virtualResults" :key="result.id" class="sr-row" type="button" :style="{ transform: `translateY(${virtualRow.start}px)` }" @click="selectResult($event, result)">
          <span class="sr-id" :title="result.id">{{ result.label ?? result.id }}</span>
          <span class="sr-context" :data-segment-id="result.sourceId ?? undefined" data-segment-content><template v-for="(chunk, index) in parts(result.sourceText)" :key="`${result.id}-source-${index}`"><mark v-if="chunk.match">{{ chunk.text }}</mark><template v-else>{{ chunk.text }}</template></template></span>
          <mark class="sr-match">{{ query || "—" }}</mark>
          <span class="sr-context" :data-segment-id="result.targetId ?? undefined" data-segment-content><template v-for="(chunk, index) in parts(result.targetText)" :key="`${result.id}-target-${index}`"><mark v-if="chunk.match">{{ chunk.text }}</mark><template v-else>{{ chunk.text }}</template></template></span>
          <span class="sr-alignment" :title="result.alignmentId ?? undefined">{{ result.alignmentLabel ?? result.alignmentId ?? "—" }}</span>
        </button>
      </div>
      <div v-if="!matchingResults.length" class="sr-empty"><Filter :size="18" />{{ t('rwSearchEmpty') }}</div>
    </div>

    <div v-if="replaceOpen" class="sr-preview" role="dialog" :aria-label="t('rwReplacePreview')">
      <header><div><span class="sr-eyebrow">{{ t('rwReplacePreviewEyebrow') }}</span><h2>{{ t('rwReplacePreview') }} <small>{{ t('rwSegmentCount', { count: displayedPreview?.resultIds.length ?? 0 }) }}</small></h2></div><button type="button" :title="t('rwClosePreview')" @click="replaceOpen = false"><X :size="17" /></button></header>
      <p v-if="authoritativeResults" class="sr-preview-note">{{ t('rwPreviewNote') }}</p>
      <p v-if="previewLoading" class="sr-preview-status">{{ t('rwGeneratingPreview') }}</p>
      <p v-else-if="previewError" class="sr-preview-status sr-preview-status--error" role="alert">{{ previewError }}</p>
      <p v-else-if="authoritativeResults && !nativePreview" class="sr-preview-status">{{ t('rwPreviewStale') }}<button class="sr-preview-retry" type="button" @click="openReplacePreview">{{ t('rwPreviewAgain') }}</button></p>
      <div class="sr-preview-list">
        <article v-for="entry in previewEntries" :key="`preview-${entry.side}-${entry.item.resultId}`"><span>{{ entry.side }} · {{ entry.item.resultId }}</span><p><del>{{ entry.item.before }}</del><b>{{ entry.item.after }}</b></p></article>
      </div>
      <footer><button class="sr-secondary" type="button" @click="replaceOpen = false">{{ t('rwCancel') }}</button><button class="sr-primary sr-primary--compact" type="button" :disabled="!canApply" @click="applyDisplayedPreview(); replaceOpen = false">{{ t('rwConfirmApply') }}</button></footer>
    </div>
  </section>
</template>

<style scoped>
.search-replace-workspace { position: relative; display: flex; flex-direction: column; height: 100%; min-height: 0; padding: 25px 28px; @apply bg-paper text-ink-900; }
.sr-toolbar, .sr-replacebar { display: flex; align-items: center; gap: 10px; }
.sr-query-label, .sr-replacebar > label { width: 42px; @apply text-ink-700; font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }
.sr-input-group { display: flex; align-items: center; width: min(520px, 42vw); height: 42px; padding: 0 12px; border: 1px solid #c9d1ca; border-radius: 7px 0 0 7px; @apply text-ink-500; }
.sr-input-group input, .sr-replacebar input { flex: 1; min-width: 0; border: 0; outline: none; @apply text-ink-900; background: transparent; }
.sr-input-group input { padding: 0 9px; }.sr-input-group button { padding: 3px; border: 0; @apply text-ink-500; background: transparent; cursor: pointer; }
.sr-toolbar select { min-height: 38px; }
.sr-primary, .sr-secondary, .sr-reset { display: inline-flex; align-items: center; justify-content: center; gap: 6px; height: 38px; padding: 0 14px; border: 1px solid var(--line); border-radius: 6px; @apply bg-raised; cursor: pointer; white-space: nowrap; }
.sr-primary { height: 42px; margin-left: -10px; @apply border-accent-strong; border-radius: 0 6px 6px 0; color: #fff; @apply bg-accent-solid; }.sr-primary:disabled { cursor: wait; }
.sr-check { display: inline-flex; align-items: center; gap: 6px; margin-left: 14px; @apply text-ink-700; font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }.sr-project { display: inline-flex; align-items: center; gap: 4px; margin-left: auto; @apply text-accent-strong; font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }
.sr-replacebar { margin-top: 13px; padding: 11px 0 13px; border-top: 1px solid var(--line); border-bottom: 1px solid var(--line); }.sr-replacebar input { height: 34px; padding: 0 10px; border: 1px solid #c9d1ca; border-radius: 5px; }.sr-replacebar > label { font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }.sr-reset { @apply text-ink-700; }.sr-secondary:hover:not(:disabled), .sr-reset:hover { border-color: #a5caa9; @apply text-accent-strong; }.sr-summary { padding: 15px 5px 12px; @apply text-ink-700; font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }.sr-summary strong { @apply text-accent-strong; }.sr-summary span { @apply text-ink-500; }
.sr-table { min-height: 0; overflow: auto; border: 1px solid var(--line); border-radius: 7px; }.sr-table-head, .sr-row { display: grid; grid-template-columns: 92px 1.2fr 100px 1.45fr 100px; align-items: center; gap: 14px; padding: 0 17px; }.sr-table-head { position: sticky; z-index: 1; top: 0; height: 43px; @apply text-ink-700 bg-raised; font-size: var(--jm-font-size-callout); line-height: var(--jm-line-height-callout); }.sr-virtual-body { position: relative; min-width: 600px; }.sr-row { position: absolute; width: 100%; height: 57px; box-sizing: border-box; border: 0; border-top: 1px solid var(--line); @apply text-ink-900 bg-raised; font-size: var(--jm-font-size-body); text-align: left; cursor: pointer; line-height: var(--jm-line-height-body); }.sr-row:hover { @apply bg-accent-subtle; }.sr-row > span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }.sr-id, .sr-alignment { @apply text-ink-500; font-family: var(--jm-font-mono); font-size: var(--jm-font-size-callout); line-height: var(--jm-line-height-callout); }.sr-row mark, .sr-match { padding: 2px 5px; color: var(--text-warning, #72591b); @apply bg-warm-soft; font-weight: var(--jm-font-weight-semibold); }.sr-context mark { padding: 1px 2px; }.sr-empty { display: flex; align-items: center; justify-content: center; gap: 8px; min-height: 180px; @apply text-ink-500; font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }
.sr-preview { position: absolute; z-index: 2; top: 100px; right: 28px; left: 28px; overflow: hidden; border: 1px solid #cbd6cc; border-radius: 9px; @apply bg-raised; box-shadow: 0 18px 48px rgb(29 48 32 / 18%); }.sr-preview header, .sr-preview footer { display: flex; align-items: center; justify-content: space-between; padding: 14px 18px; border-bottom: 1px solid var(--line); }.sr-preview header button { padding: 4px; border: 0; @apply text-ink-500; background: transparent; cursor: pointer; }.sr-preview h2 { margin: 3px 0 0; font-size: var(--jm-font-size-title-3); line-height: var(--jm-line-height-title-3); font-weight: var(--jm-font-weight-semibold); }.sr-preview h2 small { margin-left: 7px; @apply text-accent; font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-medium); line-height: var(--jm-line-height-subheadline); }.sr-eyebrow { @apply text-accent; font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-semibold); letter-spacing: .1em; line-height: var(--jm-line-height-subheadline); }.sr-preview-note, .sr-preview-status { margin: 0; padding: 10px 18px; @apply text-ink-700 bg-accent-subtle; font-size: var(--jm-font-size-callout); line-height: var(--jm-line-height-callout); }.sr-preview-status--error { color: var(--text-danger, #a24c4c); @apply bg-warm-soft; }.sr-preview-retry { margin-left: 8px; padding: 0; border: 0; @apply text-accent-strong; background: transparent; font: inherit; font-weight: var(--jm-font-weight-semibold); text-decoration: underline; cursor: pointer; }.sr-preview-list { max-height: 240px; overflow: auto; padding: 4px 18px; }.sr-preview-list article { display: grid; grid-template-columns: 85px 1fr; gap: 12px; padding: 9px 0; border-bottom: 1px solid #edf1ed; }.sr-preview-list article > span { @apply text-ink-500; font-family: var(--jm-font-mono); font-size: var(--jm-font-size-subheadline); line-height: var(--jm-line-height-subheadline); }.sr-preview-list p { display: grid; gap: 5px; margin: 0; font-size: var(--jm-font-size-callout); line-height: var(--jm-line-height-callout); }.sr-preview-list del { color: var(--text-danger, #a24c4c); @apply bg-warm-soft; }.sr-preview-list b { color: var(--green-900, #387946); @apply bg-green-soft; font-weight: var(--jm-font-weight-medium); }.sr-preview footer { justify-content: flex-end; gap: 8px; border-top: 1px solid var(--line); border-bottom: 0; }.sr-primary--compact { height: 36px; border-radius: 6px; }
.sr-toolbar, .sr-replacebar { flex-wrap: wrap; }
.sr-query-label, .sr-replacebar > label { flex: 0 0 42px; white-space: nowrap; }
.sr-input-group { flex: 1 1 240px; min-width: 160px; width: auto; }
.sr-check { flex: none; margin-left: 0; white-space: nowrap; }
.sr-project { max-width: 190px; min-width: 0; }
.sr-project > span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.sr-project svg { flex: none; }
.sr-replacebar input { flex: 1 1 200px; }
.sr-table-head, .sr-row { min-width: 600px; grid-template-columns: 62px minmax(150px, 1fr) 74px minmax(150px, 1.2fr) 70px; gap: 10px; padding-inline: 12px; }
.sr-context { user-select: text; }
</style>
