<script setup lang="ts">
import { computed } from "vue";
import { Check, Copy, Link2, RotateCcw, Split } from "@lucide/vue";
import { t } from "../i18n";

defineOptions({ name: "HistoryWorkspace" });

export interface RevisionItem {
  id: string;
  label: string;
  timestamp: string;
  action: string;
  summary: string;
  current?: boolean;
}

export interface HistoryDiff {
  segmentId: string;
  sourceOld: string;
  sourceNew: string;
  targetOld: string;
  targetNew: string;
  summary?: string;
  deletedLines?: number;
  addedLines?: number;
}

const props = withDefaults(
  defineProps<{
    revisions: RevisionItem[];
    diff: HistoryDiff | null;
    baseRevisionId?: string | null;
    selectedRevisionId?: string | null;
    currentRevisionId?: string | null;
    loading?: boolean;
  }>(),
  { baseRevisionId: null, selectedRevisionId: null, currentRevisionId: null, loading: false },
);

const emit = defineEmits<{
  "select-revision": [revisionId: string];
  compare: [baseRevisionId: string, selectedRevisionId: string];
  restore: [revisionId: string];
  "copy-value": [value: string];
}>();

const selected = computed(() => props.selectedRevisionId ?? props.revisions.find((revision) => !revision.current)?.id ?? props.revisions[0]?.id ?? null);
const current = computed(() => props.currentRevisionId ?? props.revisions.find((revision) => revision.current)?.id ?? props.revisions[0]?.id ?? null);
const base = computed(() => props.baseRevisionId ?? props.revisions.find((revision) => revision.id !== selected.value)?.id ?? null);
const canCompare = computed(() => Boolean(base.value && selected.value && base.value !== selected.value));
const selectRevision = (id: string) => emit("select-revision", id);
const compare = () => { if (base.value && selected.value) emit("compare", base.value, selected.value); };
</script>

<template>
  <section class="history-workspace" :aria-label="t('rwHistoryVersion')">
    <header class="history-heading">
      <div><span class="history-eyebrow">{{ t('rwHistoryEyebrow') }}</span><h2>{{ t('rwHistoryTitle') }}</h2><p>{{ t('rwHistoryDescription') }}</p></div>
      <div class="history-actions"><button class="history-tool" type="button" :disabled="!canCompare || loading" @click="compare"><Split :size="15" />{{ t('rwCompare') }}</button><button class="history-tool" type="button" :disabled="!selected || selected === current || loading" @click="selected && emit('restore', selected)"><RotateCcw :size="15" />{{ t('rwRestoreVersion') }}</button><span v-if="current" class="current-version"><Check :size="14" />{{ t('rwCurrent', { revision: current }) }}</span></div>
    </header>
    <div class="history-layout">
      <aside class="revision-list" :aria-label="t('rwRevisionList')">
        <button v-for="revision in revisions" :key="revision.id" class="revision-item" :class="{ active: revision.id === selected }" type="button" @click="selectRevision(revision.id)">
          <strong>{{ revision.label }} <small v-if="revision.current">{{ t('rwCurrentVersion') }}</small></strong><time>{{ revision.timestamp }}</time><b>{{ revision.action }}</b><span>{{ revision.summary }}</span>
        </button>
        <div v-if="!revisions.length" class="history-empty"><RotateCcw :size="18" />{{ t('rwRevisionsEmpty') }}</div>
      </aside>
      <section v-if="diff" class="revision-diff" :aria-label="t('rwSideBySideDiff')">
        <div class="diff-caption"><span>{{ t('rwCompareLabel') }} <b>{{ base ?? "—" }} → {{ selected ?? "—" }}</b></span><span>{{ t('rwSegmentLabel') }} {{ diff.segmentId }} <Link2 :size="15" /></span></div>
        <div class="diff-grid">
          <article><header><small>{{ base ?? t('rwBaseRevision') }} {{ t('rwBaseRevisionSuffix') }}</small><button type="button" :title="t('rwCopyBaseRevision')" @click="emit('copy-value', diff.targetOld)"><Copy :size="14" /></button></header><div class="diff-language"><span>{{ t('rwSourceColumn') }}</span><p class="diff-old">{{ diff.sourceOld || t('rwNoContent') }}</p></div><div class="diff-language"><span>{{ t('rwTargetColumn') }}</span><p class="diff-old">{{ diff.targetOld || t('rwNoContent') }}</p></div></article>
          <article><header><small>{{ selected ?? t('rwSelectedRevision') }} {{ t('rwSelectedRevisionSuffix') }}</small><button type="button" :title="t('rwCopySelectedRevision')" @click="emit('copy-value', diff.targetNew)"><Copy :size="14" /></button></header><div class="diff-language"><span>{{ t('rwSourceColumn') }}</span><p class="diff-new">{{ diff.sourceNew || t('rwNoContent') }}</p></div><div class="diff-language"><span>{{ t('rwTargetColumn') }}</span><p class="diff-new">{{ diff.targetNew || t('rwNoContent') }}</p></div></article>
        </div>
        <div class="change-summary"><b>{{ t('rwChangeSummary') }}</b><p><span class="diff-delete">{{ t('rwRemovedLines', { count: diff.deletedLines ?? 1 }) }}</span><span class="diff-add">{{ t('rwAddedLines', { count: diff.addedLines ?? 1 }) }}</span></p><p>{{ diff.summary ?? t('rwDiffReady') }}</p></div>
      </section>
      <section v-else class="history-empty history-empty--detail"><RotateCcw :size="22" /><p>{{ t('rwSelectRevisionHint') }}</p></section>
    </div>
  </section>
</template>

<style scoped>
.history-workspace { display: flex; flex-direction: column; height: 100%; min-height: 0; color: var(--ink-900); background: #fbfcfb; }.history-heading { display: flex; align-items: flex-start; justify-content: space-between; padding: 24px 30px 20px; border-bottom: 1px solid var(--line); }.history-eyebrow { color: var(--green-700); font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-semibold); letter-spacing: .1em; line-height: var(--jm-line-height-subheadline); }.history-heading h2 { margin: 4px 0; font-size: var(--jm-font-size-title-2); line-height: var(--jm-line-height-title-2); font-weight: var(--jm-font-weight-regular); }.history-heading p { margin: 0; color: var(--ink-500); font-size: var(--jm-font-size-callout); line-height: var(--jm-line-height-callout); }.history-actions { display: flex; align-items: center; gap: 9px; }.history-tool { display: inline-flex; align-items: center; gap: 6px; height: 35px; padding: 0 11px; border: 1px solid var(--line); border-radius: 6px; color: var(--ink-700); background: #fff; cursor: pointer; }.history-tool:hover:not(:disabled) { border-color: #a5caa9; color: var(--green-900); }.current-version { display: inline-flex; align-items: center; gap: 4px; color: var(--green-700); font-size: var(--jm-font-size-subheadline); line-height: var(--jm-line-height-subheadline); }.history-layout { display: grid; grid-template-columns: 290px minmax(0, 1fr); min-height: 0; flex: 1; }.revision-list { overflow: auto; padding: 18px 14px 18px 24px; border-right: 1px solid var(--line); }.revision-item { display: grid; grid-template-columns: 1fr auto; gap: 7px; width: 100%; margin-bottom: 11px; padding: 14px; border: 1px solid #e1e7e1; border-radius: 8px; color: var(--ink-900); background: #fff; text-align: left; cursor: pointer; }.revision-item:hover, .revision-item.active { border-color: #a7cfa9; background: #f5fbf3; }.revision-item strong { color: var(--green-900); font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }.revision-item strong small { margin-left: 4px; padding: 2px 5px; border-radius: 4px; color: var(--green-700); background: #e4f2e2; font-size: var(--jm-font-size-subheadline); line-height: var(--jm-line-height-subheadline); }.revision-item time { color: var(--ink-500); font-size: var(--jm-font-size-subheadline); line-height: var(--jm-line-height-subheadline); }.revision-item b, .revision-item span { grid-column: 1 / -1; }.revision-item b { font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }.revision-item span { color: var(--ink-700); font-size: var(--jm-font-size-callout); line-height: 1.5; }.revision-diff { overflow: auto; padding: 24px 28px; }.diff-caption { display: flex; justify-content: space-between; margin-bottom: 19px; color: var(--ink-700); font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }.diff-caption b { color: var(--ink-900); }.diff-caption span:last-child { display: flex; align-items: center; gap: 7px; }.diff-grid { display: grid; grid-template-columns: 1fr 1fr; border: 1px solid var(--line); border-radius: 7px; overflow: hidden; }.diff-grid > article { min-width: 0; padding: 15px 18px; }.diff-grid > article + article { border-left: 1px solid var(--line); }.diff-grid article > header { display: flex; align-items: center; justify-content: space-between; }.diff-grid small { color: var(--ink-500); font-size: var(--jm-font-size-subheadline); line-height: var(--jm-line-height-subheadline); }.diff-grid header button { padding: 2px; border: 0; color: var(--ink-500); background: transparent; cursor: pointer; }.diff-language { margin-top: 15px; }.diff-language > span { color: var(--ink-500); font-size: var(--jm-font-size-subheadline); line-height: var(--jm-line-height-subheadline); }.diff-grid p { margin: 6px 0 0; padding: 12px; color: var(--ink-900); font-size: var(--jm-font-size-body); line-height: 1.65; }.diff-old { background: #fff0f0; text-decoration: line-through; text-decoration-color: #bc7272; }.diff-new { background: #eff9ec; }.change-summary { margin-top: 22px; padding: 15px 17px; border-top: 1px solid var(--line); color: var(--ink-700); font-size: var(--jm-font-size-callout); line-height: var(--jm-line-height-callout); }.change-summary b { color: var(--ink-900); }.change-summary p { margin: 12px 0 0; }.diff-delete, .diff-add { display: inline-block; margin-right: 11px; padding: 4px 7px; border-radius: 4px; }.diff-delete { color: #a24c4c; background: #fde7e7; }.diff-add { color: #387946; background: #e6f5e2; }.history-empty { display: flex; align-items: center; justify-content: center; gap: 7px; min-height: 90px; color: var(--ink-500); font-size: var(--jm-font-size-callout); line-height: var(--jm-line-height-callout); }.history-empty--detail { min-height: 250px; flex-direction: column; }.history-empty p { margin: 0; }
</style>
