<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import { Check, ChevronRight, Search, Square, Workflow } from "@lucide/vue";
import { researchClient } from "../domain/research-client";
import type { FeatureSnapshot, ResearchOccurrence, ResearchRun, ResearchSummary, TextRange } from "../domain/research-types";
import ResearchHighlightedText from "./research/ResearchHighlightedText.vue";
import ResearchGroupsPanel from "./research/ResearchGroupsPanel.vue";
import { t, type LocalizedMessage } from "../i18n";

const props = defineProps<{
  bindingId: string | null;
  projectId: string | null;
  revisionId: string;
  available: boolean;
  active: boolean;
  feature: FeatureSnapshot | null;
  translationLabel?: string;
  beforeGroupChange?: () => Promise<boolean>;
}>();
const emit = defineEmits<{
  status: [message: LocalizedMessage]; dirty: [value: boolean];
  reveal: [segmentId: string]; pipeline: []; settings: [];
}>();
const query = ref("");
const fuzzy = ref(false);
const similarity = ref("fuzzy.edit_distance");
const autoLocate = ref(true);
const maxGap = ref(2);
const run = ref<ResearchRun | null>(null);
const runHistory = ref<ResearchRun[]>([]);
const items = ref<ResearchOccurrence[]>([]);
const nextCursor = ref<number | null>(null);
const total = ref(0);
const summary = ref<ResearchSummary | null>(null);
const summaryError = ref<string | null>(null);
const summaryVisible = ref(true);
const groupFilter = ref<string | null>(null);
const loading = ref(false);
const saving = ref(false);
const error = ref<string | null>(null);
const selectedId = ref<string | null>(null);
const draftGroup = ref("");
const draftStrategy = ref("");
const draftRanges = ref<TextRange[]>([]);
const draftKind = ref<"translation" | "omission" | "paraphrase">("translation");
// These two persisted values are protocol vocabulary used for grouping existing
// research records. Presentation labels are translated separately in the template.
const omissionCanonicalLabel = "省译";
const paraphraseCanonicalLabel = "意译";
const draftBaseline = ref("");
const viewport = ref<HTMLElement | null>(null);
let generation = 0;
let resultGeneration = 0;
let summaryGeneration = 0;
let revisionRefreshPending = false;
let timer: ReturnType<typeof setTimeout> | null = null;
let pollCount = 0;
const running = computed(() => run.value?.status === "queued" || run.value?.status === "running");
const canRun = computed(() => props.available && Boolean(props.projectId && props.bindingId) && props.feature?.status === "ready");
const canConfirm = computed(() => canRun.value && run.value?.status === "completed" && !loading.value);
const selected = computed(() => items.value.find(item => item.occurrence_id === selectedId.value) ?? null);
const alignmentCoverageMessage = computed(() => {
  switch (selected.value?.alignment_coverage) {
    case "partial": return t("wfCoveragePartial");
    case "no_links": return t("wfCoverageNoLinks");
    case "context_missing": return t("wfCoverageMissing");
    case "not_requested": return t("wfCoverageNotRequested");
    default: return null;
  }
});
const serializedDraft = () => JSON.stringify({ group: draftGroup.value, strategy: draftStrategy.value, ranges: draftRanges.value, kind: draftKind.value });
const dirty = computed(() => Boolean(selected.value && draftBaseline.value && serializedDraft() !== draftBaseline.value));
watch([dirty, saving], ([draftDirty, inFlight]) => emit("dirty", draftDirty || inFlight));
watch(run, value => { if (value) runHistory.value = runHistory.value.map(item => item.run_id === value.run_id ? value : item); });
const virtualizer = useVirtualizer<HTMLElement, HTMLButtonElement>(computed(() => ({
  count: items.value.length, getScrollElement: () => viewport.value,
  estimateSize: () => 146, overscan: 5, getItemKey: index => items.value[index]?.occurrence_id ?? index,
})));
const virtualRows = computed(() => virtualizer.value.getVirtualItems());
const totalSize = computed(() => virtualizer.value.getTotalSize());
const measureRow = (element: unknown) => { if (element instanceof HTMLButtonElement) virtualizer.value.measureElement(element); };
const statusLabel = computed<Record<string, string>>(() => ({ queued: t("wfQueued"), running: t("wfAnalyzing"), completed: t("wfCompleted"), cancelled: t("wfStopped"), failed: t("wfRunFailed"), interrupted: t("wfRunInterrupted") }));
const readable = (cause: unknown) => cause instanceof Error ? cause.message : cause && typeof cause === "object" && "message" in cause ? String(cause.message) : String(cause);
const stopPolling = () => { if (timer !== null) clearTimeout(timer); timer = null; };
const current = (epoch: number, binding: string | null) => generation === epoch && props.bindingId === binding;

function resetDraft() {
  const item = selected.value;
  const judgement = item?.judgement;
  draftGroup.value = judgement?.group_name ?? item?.candidates[0]?.text ?? "";
  draftStrategy.value = judgement?.strategy ?? "";
  draftRanges.value = (judgement?.target_ranges ?? item?.candidates[0]?.ranges ?? []).map(range => ({ ...range }));
  draftKind.value = judgement?.kind ?? "translation";
  draftBaseline.value = serializedDraft();
}
function discardDraft() { resetDraft(); }
function selectItem(item: ResearchOccurrence) {
  if (dirty.value || saving.value) { emit("status", () => t("wfHandleResearchDraft")); return; }
  selectedId.value = item.occurrence_id; resetDraft();
}

async function loadRunHistory() {
  if (!props.projectId || !props.bindingId || !props.available) return;
  const epoch = generation; const binding = props.bindingId;
  try {
    const history = await researchClient.listRuns(binding);
    if (!current(epoch, binding)) return;
    runHistory.value = history.filter(item => item.project_id === props.projectId);
    if (!run.value && runHistory.value[0]) await selectRun(runHistory.value[0].run_id);
  } catch (cause) { if (current(epoch, binding)) error.value = readable(cause); }
}
async function selectRun(id: string) {
  if (dirty.value || saving.value) { emit("status", () => t("wfHandleResearchDraft")); return; }
  if (!props.bindingId || run.value?.run_id === id) return;
  stopPolling(); const epoch = ++generation; const binding = props.bindingId;
  loading.value = true; error.value = null; pollCount = 0;
  try {
    const next = await researchClient.getRun(id, binding);
    if (!current(epoch, binding)) return;
    run.value = next; items.value = []; selectedId.value = null; draftBaseline.value = ""; nextCursor.value = null; total.value = 0; summary.value = null; summaryError.value = null; groupFilter.value = null;
    if (next.status === "completed") { await loadResults(); await refreshSummary(); }
    else schedulePoll();
  } catch (cause) { if (current(epoch, binding)) error.value = readable(cause); }
  finally { if (current(epoch, binding)) loading.value = false; }
}
function changeHistory(event: Event) {
  const control = event.target as HTMLSelectElement;
  const id = control.value;
  control.value = run.value?.run_id ?? "";
  void selectRun(id);
}

async function loadResults(append = false) {
  if (!run.value || !props.bindingId || (append && nextCursor.value === null)) return;
  if (!append && (dirty.value || saving.value)) { emit("status", () => t("wfDraftKeptRefresh")); return; }
  const epoch = generation; const binding = props.bindingId; const id = run.value.run_id;
  const requestedRevision = props.revisionId;
  const request = ++resultGeneration;
  loading.value = true;
  try {
    const page = await researchClient.result(id, append ? nextCursor.value ?? 0 : 0, 100, binding, groupFilter.value);
    if (!current(epoch, binding) || run.value?.run_id !== id || request !== resultGeneration) return;
    if (requestedRevision !== props.revisionId) { revisionRefreshPending = true; return; }
    if (!append && (dirty.value || saving.value)) { emit("status", () => t("wfDraftKeptRefresh")); return; }
    run.value = page.run; total.value = page.total; nextCursor.value = page.next_cursor;
    if (append) {
      const ids = new Set(items.value.map(item => item.occurrence_id));
      items.value.push(...page.items.filter(item => !ids.has(item.occurrence_id)));
    } else items.value = page.items;
    if (!items.value.some(item => item.occurrence_id === selectedId.value)) {
      selectedId.value = items.value[0]?.occurrence_id ?? null;
      resetDraft();
    } else if (!dirty.value && !saving.value) resetDraft();
  } catch (cause) { if (current(epoch, binding) && request === resultGeneration) error.value = readable(cause); }
  finally { if (current(epoch, binding) && request === resultGeneration) loading.value = false; }
}

async function refreshSummary() {
  if (!run.value || !props.bindingId) return;
  const epoch = generation; const binding = props.bindingId; const id = run.value.run_id; const request = ++summaryGeneration;
  const requestedRevision = props.revisionId;
  try {
    const value = await researchClient.summary(id, binding);
    if (!current(epoch, binding) || run.value?.run_id !== id || request !== summaryGeneration) return;
    if (requestedRevision !== props.revisionId) { revisionRefreshPending = true; return; }
    summary.value = value; summaryError.value = null;
  } catch (cause) { if (current(epoch, binding) && request === summaryGeneration) { summary.value = null; summaryError.value = readable(cause); } }
}
function refreshRevisionProjection() {
  if (!revisionRefreshPending || !props.active || run.value?.status !== "completed" || dirty.value || saving.value || loading.value) return;
  revisionRefreshPending = false;
  void Promise.all([loadResults(), refreshSummary()]);
}
async function guardGroupChange() {
  if (props.beforeGroupChange) return props.beforeGroupChange();
  if (dirty.value || saving.value) { emit("status", () => t("wfHandleResearchDraft")); return false; }
  return true;
}
async function filterGroup(name: string | null) {
  if (!run.value || loading.value || saving.value || name === groupFilter.value) return;
  const epoch = generation; const binding = props.bindingId; const runId = run.value.run_id;
  if (!await guardGroupChange() || !current(epoch, binding) || run.value?.run_id !== runId) return;
  groupFilter.value = name; items.value = []; selectedId.value = null; draftBaseline.value = ""; nextCursor.value = null;
  virtualizer.value.scrollToOffset(0); await loadResults();
}
async function mergeGroups(from: string, to: string) {
  if (!run.value || !props.bindingId || !canConfirm.value || saving.value || from === to) return;
  const epoch = generation; const binding = props.bindingId; const runId = run.value.run_id;
  if (!await guardGroupChange() || !current(epoch, binding) || run.value?.run_id !== runId) return;
  saving.value = true; error.value = null;
  try {
    await researchClient.mergeGroups({ run_id: runId, from_group: from, to_group: to }, binding);
    if (!current(epoch, binding) || run.value?.run_id !== runId) return;
    if (groupFilter.value === from) groupFilter.value = to;
    saving.value = false;
    await Promise.all([refreshSummary(), loadResults()]);
    emit("status", () => t("wfGroupsMerged", { p0: from, p1: to }));
  } catch (cause) { if (current(epoch, binding)) error.value = readable(cause); }
  finally { if (current(epoch, binding)) saving.value = false; }
}

function schedulePoll() {
  stopPolling();
  if (!props.active || !running.value || pollCount >= 600) return;
  timer = setTimeout(() => { timer = null; void refreshRun(); }, 1000);
}
async function refreshRun() {
  if (!run.value || !props.bindingId) return;
  const epoch = generation; const binding = props.bindingId; const id = run.value.run_id;
  try {
    const next = await researchClient.getRun(id, binding);
    if (!current(epoch, binding) || run.value?.run_id !== id) return;
    run.value = next; pollCount++;
    if (next.status === "completed") { await loadResults(); await refreshSummary(); const count = next.total; emit("status", () => t("wfResearchCompleted", { p0: count })); }
    else if (["failed", "interrupted"].includes(next.status)) error.value = next.error ?? statusLabel.value[next.status] ?? next.status;
    else if (pollCount >= 600 && running.value) error.value = t("wfPollingPausedClick");
    schedulePoll();
  } catch (cause) { if (current(epoch, binding)) { error.value = readable(cause); stopPolling(); } }
}

async function start() {
  if (!canRun.value || !props.bindingId || !query.value.trim() || running.value || loading.value) return;
  if (!Number.isInteger(maxGap.value) || maxGap.value < 0 || maxGap.value > 10) { error.value = t("wfMaxGapInvalid"); return; }
  if (dirty.value || saving.value) { emit("status", () => t("wfHandleResearchDraft")); return; }
  stopPolling(); const epoch = ++generation; const binding = props.bindingId;
  loading.value = true; error.value = null; items.value = []; selectedId.value = null; draftBaseline.value = ""; run.value = null; total.value = 0; nextCursor.value = null; pollCount = 0; summary.value = null; summaryError.value = null; groupFilter.value = null;
  try {
    const next = await researchClient.start({ query: query.value.trim(), fuzzy: fuzzy.value, similarity_operator: similarity.value, auto_locate: autoLocate.value, max_gap: maxGap.value }, binding);
    if (!current(epoch, binding)) return;
    run.value = next;
    runHistory.value = [next, ...runHistory.value.filter(item => item.run_id !== next.run_id)];
    if (next.status === "completed") { await loadResults(); await refreshSummary(); }
    else schedulePoll();
  } catch (cause) { if (current(epoch, binding)) error.value = readable(cause); }
  finally { if (current(epoch, binding)) loading.value = false; }
}
async function cancel() {
  if (!run.value || !props.bindingId) return;
  const epoch = generation; const binding = props.bindingId;
  try { await researchClient.cancelRun(run.value.run_id, binding); if (current(epoch, binding)) await refreshRun(); }
  catch (cause) { if (current(epoch, binding)) error.value = readable(cause); }
}

async function saveDraft(kind = draftKind.value): Promise<boolean> {
  const item = selected.value;
  if (!item || !run.value || !props.bindingId || !canConfirm.value || saving.value) return false;
  if (kind === "translation" && !draftRanges.value.length) { error.value = t("wfSelectTargetRange"); return false; }
  const epoch = generation; const binding = props.bindingId; const runId = run.value.run_id;
  const request = { run_id: runId, occurrence_id: item.occurrence_id, kind, target_ranges: kind === "omission" ? [] : draftRanges.value.map(range => ({ ...range })), group_name: kind === "omission" ? omissionCanonicalLabel : draftGroup.value.trim() || undefined, strategy: draftStrategy.value.trim() || (kind === "omission" ? omissionCanonicalLabel : kind === "paraphrase" ? paraphraseCanonicalLabel : undefined) };
  saving.value = true; error.value = null;
  try {
    await researchClient.confirm(request, binding);
    if (!current(epoch, binding)) return false;
    // Only show a confirmation after the authoritative command succeeds.
    item.judgement = { kind, group_name: request.group_name ?? "", strategy: request.strategy ?? "", target_ranges: request.target_ranges };
    item.status = "confirmed"; resetDraft();
    await refreshSummary();
    if (groupFilter.value !== null) { saving.value = false; await loadResults(); }
    emit("status", () => t("wfResearchConfirmationSaved")); return true;
  } catch (cause) { if (current(epoch, binding)) error.value = readable(cause); return false; }
  finally { if (current(epoch, binding)) saving.value = false; }
}
function captureRange(segmentId: string, event: MouseEvent) {
  if (!selected.value || saving.value || !canConfirm.value) return;
  const root = event.currentTarget as HTMLElement;
  const selection = window.getSelection();
  if (!selection || selection.isCollapsed || !selection.rangeCount) return;
  const range = selection.getRangeAt(0);
  if (!root.contains(range.startContainer) || !root.contains(range.endContainer)) return;
  const before = document.createRange(); before.selectNodeContents(root); before.setEnd(range.startContainer, range.startOffset);
  const encoder = new TextEncoder(); const start = encoder.encode(before.toString()).length;
  draftRanges.value = [{ segment_id: segmentId, start_utf8: start, end_utf8: start + encoder.encode(range.toString()).length }];
  draftKind.value = "translation"; draftGroup.value = range.toString();
}
function chooseCandidate(index: number) {
  const candidate = selected.value?.candidates[index]; if (!candidate || saving.value || !canConfirm.value) return;
  draftRanges.value = candidate.ranges.map(range => ({ ...range })); draftGroup.value = candidate.text; draftKind.value = "translation";
}
function keyboard(event: KeyboardEvent) {
  if (!props.active || event.isComposing || event.key === "Process" || event.metaKey || event.ctrlKey || event.altKey) return;
  const target = event.target as HTMLElement | null;
  if (target?.isContentEditable || target?.closest("input,textarea,select,button")) return;
  if (!selected.value) return;
  if (event.key === "Enter") { event.preventDefault(); void saveDraft(); }
  else if (event.key.toLowerCase() === "o") { event.preventDefault(); void saveDraft("omission"); }
  else if (event.key.toLowerCase() === "p") { event.preventDefault(); void saveDraft("paraphrase"); }
  else if (event.key.toLowerCase() === "e") { event.preventDefault(); emit("reveal", selected.value.source.segment_id); }
}
watch(() => [props.projectId, props.bindingId] as const, () => {
  generation++; resultGeneration++; summaryGeneration++; revisionRefreshPending = false; stopPolling(); run.value = null; runHistory.value = []; items.value = []; selectedId.value = null; draftBaseline.value = ""; loading.value = false; saving.value = false; error.value = null; total.value = 0; nextCursor.value = null; summary.value = null; summaryError.value = null; groupFilter.value = null;
  void loadRunHistory();
}, { immediate: true });
watch(() => props.active, active => { if (active && running.value) { pollCount = 0; void refreshRun(); } else if (!active) stopPolling(); });
watch(() => props.revisionId, () => {
  revisionRefreshPending = true;
  if (dirty.value) emit("status", () => t("wfRevisionChangedDraftKept"));
  refreshRevisionProjection();
});
watch([() => props.active, dirty, saving, loading], refreshRevisionProjection);
watch(() => props.feature, feature => { if (feature && !run.value) { similarity.value = feature.default_similarity; autoLocate.value = feature.auto_locate; } }, { immediate: true });
onBeforeUnmount(() => { generation++; stopPolling(); emit("dirty", false); });
defineExpose({ hasDirtyDraft: () => dirty.value || saving.value, saveDraft, discardDraft, refresh: refreshRun });
</script>

<template>
  <section class="research-workspace" :aria-label="t('wfTranslationResearch')" @keydown="keyboard">
    <header class="research-header"><div><h2>{{ t('wfTranslationResearch') }}</h2><p>{{ t('wfResearchSubtitle') }}</p></div><button type="button" @click="emit('pipeline')"><Workflow :size="15" />{{ t('wfPipeline') }}</button><button type="button" @click="emit('settings')">{{ t('wfFeatureSettings') }}</button></header>
    <div v-if="!available || !projectId" class="research-empty"><Search :size="32" /><h3>{{ available ? t('wfOpenProjectResearch') : t('wfUseDesktopResearch') }}</h3><p>{{ t('wfRealProjectRequired') }}</p></div>
    <template v-else>
      <p v-if="feature?.status !== 'ready'" class="research-notice" role="status">{{ t('wfFeatureNotReady') }}<button type="button" @click="emit('settings')">{{ t('wfViewPreparation') }}</button></p>
      <form class="research-query" @submit.prevent="start"><label class="query-text"><span>{{ t('wfSourceQuery') }}</span><input v-model="query" :placeholder="t('wfQueryExample')" :aria-label="t('wfSourceQueryAria')" /></label><label><span>{{ t('wfQueryMode') }}</span><select v-model="fuzzy" :aria-label="t('wfQueryMode')"><option :value="false">{{ t('wfExactProximity') }}</option><option :value="true">{{ t('wfFuzzyMatch') }}</option></select></label><label v-if="fuzzy"><span>{{ t('wfFuzzyAlgorithm') }}</span><select v-model="similarity" :aria-label="t('wfFuzzyAlgorithm')"><option value="fuzzy.edit_distance">{{ t('wfEditDistance') }}</option><option value="fuzzy.char_ngram">{{ t('wfCharNgram') }}</option></select></label><label><span>{{ t('wfMaxWordGap') }}</span><input v-model.number="maxGap" type="number" min="0" max="10" step="1" required :aria-label="t('wfMaxWordGap')" /></label><label class="check-option"><input v-model="autoLocate" type="checkbox" />{{ t('wfAutoLocateTranslation') }}</label><button v-if="running" type="button" @click="cancel"><Square :size="14" />{{ t('wfStop') }}</button><button v-else class="primary" :disabled="!canRun || loading || !query.trim()" type="submit"><Search :size="15" />{{ t('wfSearch') }}</button></form>
      <div v-if="runHistory.length" class="research-history"><label>{{ t('wfQueryHistory') }} <select :aria-label="t('wfQueryHistory')" :value="run?.run_id ?? ''" :disabled="loading || saving" @change="changeHistory"><option v-for="item in runHistory" :key="item.run_id" :value="item.run_id">{{ t('wfInputRevision', { p0: item.input_revision_id }) }} · {{ statusLabel[item.status] ?? item.status }} · {{ item.run_id.slice(0, 8) }}</option></select></label></div>
      <p v-if="error" class="research-error" role="alert">{{ error }}<button v-if="run" type="button" @click="pollCount = 0; refreshRun()">{{ t('wfRefreshStatus') }}</button></p>
      <div v-if="run" class="research-run" role="status"><span>{{ statusLabel[run.status] ?? run.status }} · {{ run.completed }} / {{ run.total }}</span><span>{{ t('wfInputRevision', { p0: run.input_revision_id }) }}<strong v-if="run.input_revision_id !== revisionId"> · {{ t('wfHistoricalRevisionWarning') }}</strong></span><span>{{ t(groupFilter !== null ? 'wfCurrentGroup' : 'wfHits') }} {{ total }}<template v-if="summary"> · {{ t('wfConfirmedCount', { p0: summary.confirmed }) }} · {{ t('wfPendingCount', { p0: summary.pending }) }}</template></span><button v-if="summary" type="button" :aria-expanded="summaryVisible" @click="summaryVisible = !summaryVisible">{{ t(summaryVisible ? 'wfCollapseGroups' : 'wfExpandGroups') }}</button></div>
      <div class="research-body">
        <div class="research-results"><div ref="viewport" class="research-viewport" :aria-label="t('wfSourceHitsList')"><div :style="{ height: `${totalSize}px`, position: 'relative' }"><button v-for="row in virtualRows" :key="String(row.key)" :ref="measureRow" :data-index="row.index" type="button" class="research-row" :class="{ selected: items[row.index]?.occurrence_id === selectedId }" :style="{ position: 'absolute', top: 0, left: 0, width: '100%', transform: `translateY(${row.start}px)` }" @click="items[row.index] && selectItem(items[row.index]!)"><template v-if="items[row.index]"><small :class="{ 'review-required': items[row.index]!.status === 'needs_review' }">{{ t(items[row.index]!.status === 'needs_review' ? 'wfNeedsReview' : items[row.index]!.status === 'confirmed' && items[row.index]!.judgement ? 'wfConfirmed' : 'wfPending') }}</small><p><ResearchHighlightedText :text="items[row.index]!.source.text" :ranges="items[row.index]!.source.ranges" /></p><p class="candidate-summary">{{ items[row.index]!.candidates[0]?.text ?? t('wfNoMatchingFragment') }}</p><ChevronRight :size="14" /></template></button></div><p v-if="!items.length" class="research-empty-text">{{ t(running ? 'wfAnalyzingHits' : run?.status === 'completed' ? 'wfNoQueryHits' : 'wfEnterSourceQuery') }}</p></div><button v-if="nextCursor !== null" type="button" :disabled="loading" @click="loadResults(true)">{{ t(loading ? 'wfLoading' : 'wfLoadMoreHits') }}</button></div>
        <section class="research-review" tabindex="0" :aria-label="t('wfReviewAria')">
          <template v-if="selected"><header><h3>{{ t('wfConfirmTranslationFragment') }}</h3><button type="button" @click="emit('reveal', selected.source.segment_id)">{{ t('wfReturnEditor') }} · E</button></header><p v-if="selected.status === 'needs_review'" class="research-notice" role="status">{{ t('wfNeedsReviewNotice') }}</p><p v-if="alignmentCoverageMessage" class="research-notice alignment-coverage" role="status">{{ alignmentCoverageMessage }}</p><p class="review-source"><ResearchHighlightedText :text="selected.source.text" :ranges="selected.source.ranges" /></p><div v-if="selected.candidates.length > 1" class="candidate-options"><button v-for="(candidate, index) in selected.candidates" :key="index" type="button" :disabled="saving || !canConfirm" @click="chooseCandidate(index)">{{ candidate.text }}</button></div><p v-if="!selected.targets.length" class="research-notice">{{ t('wfNoAlignedTranslation') }}</p><div v-for="target in selected.targets" :key="target.segment_id" class="target-text" :data-segment-id="target.segment_id"><p data-segment-content @mouseup="captureRange(target.segment_id, $event)"><ResearchHighlightedText :text="target.text" :ranges="draftRanges.filter(range => range.segment_id === target.segment_id)" /></p></div><p class="research-help">{{ t('wfAutoLocateHelp') }}</p><small v-if="selected.candidates[0]">{{ t(selected.candidates[0].provider_id.toLowerCase().includes('xlm') ? 'wfWordAlignmentXlm' : 'wfAutoLocateCandidate') }}<template v-if="selected.candidates[0].score !== null"> · {{ t('wfLocationScore', { p0: selected.candidates[0].score.toFixed(3) }) }}</template></small><label>{{ t('wfGroupName') }}<input v-model="draftGroup" :disabled="saving || !canConfirm" :placeholder="t('wfGroupNamePlaceholder')" /></label><label>{{ t('wfStrategyCode') }}<input v-model="draftStrategy" :disabled="saving || !canConfirm" :placeholder="t('wfStrategyPlaceholder')" /></label><div class="review-actions"><button class="primary" type="button" :disabled="saving || !canConfirm" @click="saveDraft('translation')"><Check :size="14" />{{ t(saving ? 'wfSaving' : 'wfAcceptEnter') }}</button><button type="button" :disabled="saving || !canConfirm" @click="saveDraft('omission')">{{ t('wfOmission') }} · O</button><button type="button" :disabled="saving || !canConfirm" @click="saveDraft('paraphrase')">{{ t('wfParaphrase') }} · P</button><button v-if="dirty" type="button" :disabled="saving" @click="discardDraft">{{ t('wfDiscardDraft') }}</button></div><p v-if="selected.judgement" class="saved-status" :class="{ 'review-required': selected.status === 'needs_review' }">{{ t(selected.status === 'needs_review' ? 'wfHistoricalJudgement' : 'wfSaved') }}{{ t(selected.judgement.kind === 'omission' ? 'wfOmission' : selected.judgement.kind === 'paraphrase' ? 'wfParaphrase' : 'wfTranslationFragment') }}<template v-if="selected.judgement.group_name"> · {{ selected.judgement.group_name }}</template></p></template><p v-else class="research-empty-text">{{ t('wfSelectHitToReview') }}</p>
        </section>
      </div>
      <p v-if="summaryError" class="research-error" role="alert">{{ t('wfSummaryReadFailed', { p0: summaryError }) }}<button type="button" @click="refreshSummary">{{ t('wfRetrySummary') }}</button></p>
      <div v-if="summary" v-show="summaryVisible" class="research-summary"><ResearchGroupsPanel :summary="summary" :active-group="groupFilter" :writable="canConfirm" :busy="saving || loading" :translation-label="translationLabel ?? t('wfCurrentTranslation')" @filter="filterGroup" @merge="mergeGroups" /></div>
    </template>
  </section>
</template>

<style scoped>
.research-summary { flex: none; max-height: 35%; min-height: 200px; overflow: auto; padding: 14px 20px; border-top: 1px solid var(--line); background: var(--surface-raised); }.research-workspace { height: 100%; min-height: 0; display: flex; flex-direction: column; color: var(--ink-900); background: var(--surface-app); }
.research-header, .research-query, .research-run, .research-review header, .review-actions { display: flex; align-items: center; gap: 12px; }
.research-header { padding: 18px 22px; border-bottom: 1px solid var(--line); background: var(--surface-raised); }.research-header > div { margin-right: auto; }.research-header h2 { margin: 0; font-size: 19px; }.research-header p { margin: 5px 0 0; font-size: 12px; color: var(--ink-500); }
button, input { font: inherit; font-size: 12px; color: var(--ink-900); }button { display: inline-flex; align-items: center; justify-content: center; gap: 5px; padding: 7px 10px; min-height: 34px; border: 1px solid var(--line); border-radius: 6px; background: var(--surface-raised); cursor: pointer; }button:disabled { opacity: .55; cursor: default; }.primary { color: var(--paper); background: var(--green-700); border-color: var(--green-700); }input { padding: 8px; border: 1px solid var(--line); border-radius: 5px; background: var(--surface-input); }input[type="checkbox"] { accent-color: var(--green-700); }
.research-query { flex-wrap: wrap; padding: 15px 20px; align-items: end; border-bottom: 1px solid var(--line); }.research-query label { display: flex; flex-direction: column; gap: 5px; font-size: 11px; color: var(--ink-500); }.query-text { flex: 1; min-width: 170px; }.research-query input[type="number"] { width: 68px; }.research-query .check-option { flex-direction: row; align-items: center; min-height: 34px; }
.research-history { padding: 8px 20px; font-size: 12px; border-bottom: 1px solid var(--line); }.research-history select { margin-left: 8px; }.research-run { flex-wrap: wrap; padding: 10px 20px; font-size: 11px; color: var(--ink-500); border-bottom: 1px solid var(--line); }.research-run strong { font-weight: 500; color: var(--ink-900); }.research-body { flex: 1; min-height: 0; display: grid; grid-template-columns: minmax(240px, 43%) minmax(280px, 1fr); }.research-results { display: flex; flex-direction: column; min-height: 0; border-right: 1px solid var(--line); }.research-viewport { flex: 1; min-height: 0; overflow: auto; }.research-row { display: block; text-align: left; padding: 18px 28px 18px 18px; border-radius: 0; border-width: 0 0 1px; min-height: 130px; line-height: 1.7; }.research-row.selected { background: var(--surface-green-soft); box-shadow: inset 3px 0 var(--green-700); }.research-row p { margin: 8px 0; overflow-wrap: anywhere; }.research-row small { font-size: 11px; color: var(--ink-500); }.research-row > svg { position: absolute; right: 9px; top: 30px; }.candidate-summary { color: var(--ink-500); }.research-review { overflow: auto; padding: 20px; outline-offset: -3px; }.research-review header { justify-content: space-between; flex-wrap: wrap; }.research-review h3 { margin: 0; font-size: 15px; }.review-source { line-height: 1.8; padding-bottom: 14px; border-bottom: 1px solid var(--line); }.target-text p { margin: 15px 0; line-height: 1.9; white-space: pre-wrap; overflow-wrap: anywhere; }.research-help, .research-review > small { color: var(--ink-500); font-size: 11px; line-height: 1.7; }.research-review > label { display: flex; flex-direction: column; gap: 7px; margin-top: 17px; font-size: 12px; }.review-actions { margin-top: 20px; flex-wrap: wrap; }.candidate-options { display: flex; flex-wrap: wrap; gap: 6px; margin: 10px 0; }.research-notice, .research-error { margin: 0; padding: 10px 20px; font-size: 12px; line-height: 1.7; background: var(--surface-green-soft); }.research-error { color: var(--ink-900); }.research-notice button, .research-error button { margin-left: 8px; }.research-empty { margin: auto; padding: 32px; text-align: center; color: var(--ink-500); }.research-empty-text { padding: 28px; font-size: 13px; color: var(--ink-500); line-height: 1.8; }.saved-status { font-size: 12px; color: var(--green-900); }
.research-row small.review-required, .saved-status.review-required { color: var(--ink-900); font-weight: 600; }
@media(max-width: 1000px) { .research-body { grid-template-columns: minmax(200px, 40%) minmax(260px, 1fr); }.research-header { padding: 14px; }.research-query { padding: 12px; gap: 8px; }.research-review { padding: 14px; } }
</style>
