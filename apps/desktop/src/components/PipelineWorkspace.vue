<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { VueFlow, Handle, Position, useVueFlow, type Node, type Edge } from "@vue-flow/core";
import { Check, FileText, GitBranch, Play, Plus, Save, Scissors, Square, Workflow } from "@lucide/vue";
import { pipelineClient } from "../domain/pipeline-client";
import type { PipelineMethod, PipelineMethodSummary, PipelinePlanSnapshot, TokenArtifact } from "../domain/pipeline-types";
import type { SegmentDto } from "../domain/kernel-client";
import PipelineV2Workspace from "./PipelineV2Workspace.vue";
import { t, type LocalizedMessage } from "../i18n";
import "@vue-flow/core/dist/style.css";

const props = withDefaults(defineProps<{ bindingId: string | null; projectId: string | null; revisionId: string; sourceSegments: SegmentDto[]; refreshKey: number; available: boolean; active?: boolean }>(), { active: true });
const emit = defineEmits<{ status: [message: LocalizedMessage]; selectNode: [nodeId: string | null]; dirty: [dirty: boolean] }>();
const methods = ref<PipelineMethodSummary[]>([]);
const method = ref<PipelineMethod | null>(null);
const draft = ref<PipelinePlanSnapshot | null>(null);
const selectedNodeId = ref<string | null>(null);
const segmentId = ref("");
const artifact = ref<TokenArtifact | null>(null);
const busy = ref(false);
const error = ref<string | null>(null);
const remoteChanged = ref(false);
const operationId = ref<string | null>(null);
const dictionaryInput = ref("");
const { fitView } = useVueFlow("jueming-pipeline");
const savedDrafts = new Map<string, { method: PipelineMethod; draft: PipelinePlanSnapshot; selectedNodeId: string | null; segmentId: string }>();
const surface = ref<"v1" | "v2">("v2");
const v2Dirty = ref(false);
const v2Workspace = ref<{ hasDirtyDraft: () => boolean; saveDraft: () => Promise<boolean>; discardDraft: () => void; revealNode: (id: string) => Promise<boolean>; reload: () => Promise<void> } | null>(null);
let requestGeneration = 0;
const labels = computed<Record<string, string>>(() => ({ source: t("wfProjectSource"), normalize: t("wfNormalizeText"), chinese_tokenize: t("wfChineseTokenize"), artifact: t("wfTokenResult") }));
const hints = computed<Record<string, string>>(() => ({ source: t("wfCurrentProjectReadonly"), normalize: t("wfCleanDerivedText"), chinese_tokenize: t("wfJiebaDictionary"), artifact: t("wfSavedSeparatelySourceKept") }));
const icons = { source: FileText, normalize: GitBranch, chinese_tokenize: Scissors, artifact: Check };
const dirty = computed(() => Boolean(method.value && draft.value && JSON.stringify(draft.value) !== JSON.stringify(method.value.current.plan)));
watch([dirty, v2Dirty], ([legacy, graph]) => emit("dirty", legacy || graph));
function switchSurface(next: "v1" | "v2") {
  if (next === surface.value) return;
  if (dirty.value || v2Dirty.value) { emit("status", () => t("wfHandleDraftBeforeEditor")); return; }
  surface.value = next;
}
const selectedNode = computed(() => draft.value?.nodes.find(node => node.node_id === selectedNodeId.value));
const dictionary = computed(() => (selectedNode.value?.config.custom_dictionary ?? []) as Array<{ word: string; frequency: number | null; tag: string | null }>);
const nodes = computed<Node[]>(() => (draft.value?.nodes ?? []).map((node, index) => ({ id: node.node_id, type: "method", position: { x: 35 + index * 240, y: 90 }, data: { operator: node.operator, label: labels.value[node.operator] ?? node.operator, hint: hints.value[node.operator] }, selected: node.node_id === selectedNodeId.value })));
const edges = computed<Edge[]>(() => (draft.value?.nodes ?? []).flatMap(node => node.inputs.map(input => ({ id: `${input}:${node.node_id}`, source: input, target: node.node_id, type: "smoothstep" }))));
const copy = <T,>(value: T): T => JSON.parse(JSON.stringify(value)) as T;
const readable = (cause: unknown) => cause instanceof Error ? cause.message : String(cause);
function accept(next: PipelineMethod) {
  method.value = next; draft.value = copy(next.current.plan); remoteChanged.value = false;
  selectedNodeId.value = next.current.plan.nodes.some(node => node.node_id === selectedNodeId.value) ? selectedNodeId.value : next.current.plan.nodes.find(node => node.operator === "chinese_tokenize")?.node_id ?? next.current.plan.nodes[0]?.node_id ?? null;
}
async function load(id?: string, discardDraft = false) {
  if (!props.bindingId || !props.projectId) return;
  const generation = ++requestGeneration; const binding = props.bindingId;
  try {
    const nextMethods = await pipelineClient.list(binding);
    if (generation !== requestGeneration) return;
    methods.value = nextMethods;
    const targetId = id ?? method.value?.method_id ?? nextMethods[0]?.method_id;
    if (!targetId) return;
    const next = await pipelineClient.get(targetId, binding);
    if (generation !== requestGeneration || binding !== props.bindingId) return;
    if (dirty.value && !discardDraft) { remoteChanged.value = next.current.method_revision_id !== method.value?.current.method_revision_id; return; }
    accept(next); error.value = null;
    await nextTick(); void fitView({ padding: .16, duration: 0 });
  } catch (cause) { if (generation === requestGeneration) error.value = readable(cause); }
}
watch(() => [props.projectId, props.bindingId] as const, ([project, binding], previous) => {
  requestGeneration++;
  if (previous?.[0] && method.value && draft.value) savedDrafts.set(previous[0], { method: copy(method.value), draft: copy(draft.value), selectedNodeId: selectedNodeId.value, segmentId: segmentId.value });
  const saved = project ? savedDrafts.get(project) : undefined;
  method.value = saved?.method ?? null; draft.value = saved?.draft ?? null; selectedNodeId.value = saved?.selectedNodeId ?? null;
  segmentId.value = saved?.segmentId ?? props.sourceSegments[0]?.id ?? "";
  methods.value = []; artifact.value = null; error.value = null; remoteChanged.value = false;
  operationId.value = null; busy.value = false;
  if (binding) void load();
}, { immediate: true });
watch(() => props.refreshKey, () => { void load(); });
watch(selectedNodeId, value => emit("selectNode", value));
async function create() {
  if (!props.bindingId || busy.value) return;
  busy.value = true; error.value = null;
  const binding = props.bindingId;
  try {
    const next = await pipelineClient.createDefault(t("wfChineseTokenize"), binding);
    if (binding !== props.bindingId) return;
    accept(next); await load(next.method_id); emit("status", () => t("wfTokenizerCreated"));
  } catch (cause) { if (binding === props.bindingId) error.value = readable(cause); }
  finally { if (binding === props.bindingId) busy.value = false; }
}
async function save() {
  if (!method.value || !draft.value || !props.bindingId || busy.value) return;
  busy.value = true; error.value = null;
  const binding = props.bindingId;
  const sentDraft = copy(draft.value);
  try {
    const next = await pipelineClient.update({ method_id: method.value.method_id, base_method_revision_id: method.value.current.method_revision_id, name: null, plan: sentDraft }, binding);
    if (binding !== props.bindingId) return;
    if (JSON.stringify(draft.value) === JSON.stringify(sentDraft)) accept(next);
    else method.value = next;
    emit("status", () => t("wfTokenizerSaved"));
  } catch (cause) { if (binding === props.bindingId) error.value = readable(cause); }
  finally { if (binding === props.bindingId) busy.value = false; }
}
async function run() {
  if (!method.value || !props.bindingId || !segmentId.value || busy.value || dirty.value) return;
  const binding = props.bindingId; const revision = props.revisionId;
  const id = crypto.randomUUID(); operationId.value = id; busy.value = true; error.value = null;
  try {
    const result = await pipelineClient.execute({ method_id: method.value.method_id, method_revision_id: method.value.current.method_revision_id, segment_id: segmentId.value, base_revision_id: revision, operation_id: id }, binding);
    if (binding === props.bindingId && operationId.value === id) { artifact.value = result.artifact; const count = result.artifact.tokens.length; emit("status", () => t("wfTokenizerCompleted", { p0: count })); }
  } catch (cause) { if (binding === props.bindingId) error.value = readable(cause); }
  finally { if (operationId.value === id) { operationId.value = null; busy.value = false; } }
}
async function cancel() {
  if (!operationId.value || !props.bindingId) return;
  try { await pipelineClient.cancel(operationId.value, props.bindingId); emit("status", () => t("wfTokenizerStopping")); } catch (cause) { error.value = readable(cause); }
}
function addWord() {
  const word = dictionaryInput.value.trim();
  if (!word || !selectedNode.value || dictionary.value.some(item => item.word === word)) return;
  selectedNode.value.config.custom_dictionary = [...dictionary.value, { word, frequency: null, tag: null }]; dictionaryInput.value = "";
}
function removeWord(word: string) { if (selectedNode.value) selectedNode.value.config.custom_dictionary = dictionary.value.filter(item => item.word !== word); }
async function revealNode(nodeId: string): Promise<boolean> {
  if (!draft.value?.nodes.some(node => node.node_id === nodeId)) await load();
  if (!draft.value?.nodes.some(node => node.node_id === nodeId) && !dirty.value && props.bindingId) {
    const binding = props.bindingId;
    for (const candidate of methods.value) {
      const next = await pipelineClient.get(candidate.method_id, binding);
      if (binding !== props.bindingId || dirty.value) return false;
      if (next.current.plan.nodes.some(node => node.node_id === nodeId)) { accept(next); break; }
    }
  }
  if (!draft.value?.nodes.some(node => node.node_id === nodeId)) return false;
  selectedNodeId.value = nodeId; await nextTick(); await fitView({ nodes: [nodeId], padding: .5, maxZoom: 1, duration: 0 }); return true;
}
defineExpose({
  revealNode: async (nodeId: string) => {
    if (dirty.value || v2Dirty.value) return false;
    surface.value = "v2"; await nextTick();
    if (await v2Workspace.value?.revealNode(nodeId)) return true;
    surface.value = "v1"; await nextTick(); return revealNode(nodeId);
  },
  reload: async () => { await Promise.all([load(), v2Workspace.value?.reload()]); },
  hasDirtyDraft: () => dirty.value || Boolean(v2Workspace.value?.hasDirtyDraft()),
  saveDraft: async () => { if (v2Dirty.value && !await v2Workspace.value?.saveDraft()) return false; if (dirty.value) await save(); return !dirty.value && !v2Workspace.value?.hasDirtyDraft(); },
  discardDraft: () => { if (method.value) accept(method.value); v2Workspace.value?.discardDraft(); },
});
</script>

<template>
  <section class="pipeline-workspace" :aria-label="t('wfPipelineAria')">
    <nav class="pipeline-surfaces" :aria-label="t('wfPipelineEditorAria')"><button type="button" :aria-pressed="surface === 'v2'" @click="switchSurface('v2')">{{ t('wfDataFlowV2') }}</button><button type="button" :aria-pressed="surface === 'v1'" @click="switchSurface('v1')">{{ t('wfTokenizerV1') }}</button></nav>
    <PipelineV2Workspace ref="v2Workspace" v-show="surface === 'v2'" :binding-id="bindingId" :project-id="projectId" :revision-id="revisionId" :available="available" :active="active && surface === 'v2'" :refresh-key="refreshKey" @dirty="v2Dirty = $event" @status="emit('status', $event)" />
    <template v-if="surface === 'v1'">

    <header class="pipeline-header"><div class="pipeline-title"><Workflow :size="22" /><div><h2>{{ t('wfPipeline') }}</h2><p>{{ t('wfMethodsParamsResults') }}</p></div></div><span v-if="method" class="method-version">{{ t('wfMethodVersion', { p0: method.revisions.length }) }}<span v-if="dirty"> · {{ t('wfUnsaved') }}</span></span><button v-if="method" type="button" :disabled="busy || !dirty" @click="save"><Save :size="15" />{{ t('wfSaveMethod') }}</button><button v-if="operationId" type="button" @click="cancel"><Square :size="14" />{{ t('wfStop') }}</button><button v-else-if="method" class="pipeline-run" type="button" :disabled="busy || dirty || !segmentId" @click="run"><Play :size="15" />{{ t('wfRun') }}</button></header>
    <div v-if="!available || !projectId" class="pipeline-empty"><Workflow :size="36" /><h3>{{ t('wfOrganizeFromProject') }}</h3><p>{{ available ? t('wfOpenProjectMethodHint') : t('wfDesktopPipelineHint') }}</p></div>
    <div v-else-if="!method" class="pipeline-empty"><Workflow :size="36" /><h3>{{ t('wfCreateFirstTokenizer') }}</h3><p>{{ t('wfTokenizerFlow') }}</p><button class="pipeline-run" type="button" :disabled="busy || !bindingId" @click="create"><Plus :size="16" />{{ t('wfCreateChineseTokenizer') }}</button><p v-if="error" role="alert">{{ error }}</p></div>
    <template v-else>
      <div class="pipeline-method-bar"><select :value="method.method_id" :aria-label="t('wfCurrentMethod')" :disabled="dirty || busy" @change="load(($event.target as HTMLSelectElement).value)"><option v-for="item in methods" :key="item.method_id" :value="item.method_id">{{ item.name }}</option></select><span>{{ t('wfInputRevision', { p0: revisionId }) }}</span><span>{{ t('wfDerivedSavedSeparately') }}</span></div>
      <p v-if="remoteChanged" class="pipeline-notice" role="status">{{ t('wfRemoteMethodChanged') }}<button type="button" @click="load(undefined, true)">{{ t('wfDiscardLoadNew') }}</button></p>
      <p v-if="error" class="pipeline-notice" role="alert">{{ error }}</p>
      <div class="pipeline-body">
        <div class="pipeline-canvas"><VueFlow id="jueming-pipeline" :nodes="nodes" :edges="edges" :nodes-connectable="false" :edges-updatable="false" :zoom-on-scroll="false" :pan-on-scroll="true" :zoom-on-pinch="true" :min-zoom="0.3" :max-zoom="1.5" :delete-key-code="null" fit-view-on-init @node-click="selectedNodeId = $event.node.id"><template #node-method="{ id, data }"><div class="pipeline-node" :class="{ 'pipeline-node--selected': id === selectedNodeId }" :data-pipeline-node-id="id"><Handle type="target" :position="Position.Left" /><span class="pipeline-node__icon"><component :is="icons[data.operator as keyof typeof icons] ?? Workflow" :size="20" /></span><strong>{{ data.label }}</strong><small>{{ data.hint }}</small><Handle type="source" :position="Position.Right" /></div></template></VueFlow><button class="pipeline-fit" type="button" @click="fitView({ padding: .16, duration: 0 })">{{ t('wfFitCanvas') }}</button></div>
        <aside class="pipeline-inspector" :aria-label="t('wfNodeParams')"><div v-if="selectedNode"><span class="pipeline-eyebrow">{{ t('wfNodeParams') }}</span><h3>{{ labels[selectedNode.operator] }}</h3><template v-if="selectedNode.operator === 'chinese_tokenize'"><label class="pipeline-check"><input v-model="selectedNode.config.hmm" type="checkbox" />{{ t('wfRecognizeUnknown') }}</label><h4>{{ t('wfCustomDictionary') }}</h4><p>{{ t('wfDictionaryHint') }}</p><form class="pipeline-word-form" @submit.prevent="addWord"><input v-model="dictionaryInput" :aria-label="t('wfAddCustomWord')" :placeholder="t('wfWordExample')" /><button type="submit" :disabled="!dictionaryInput.trim()" :aria-label="t('wfAddWord')"><Plus :size="15" /></button></form><ul class="pipeline-dictionary"><li v-for="entry in dictionary" :key="entry.word"><span>{{ entry.word }}</span><button type="button" :aria-label="t('wfDeleteWord', { p0: entry.word })" @click="removeWord(entry.word)">×</button></li></ul><p v-if="!dictionary.length">{{ t('wfNoCustomWords') }}</p></template><template v-else-if="selectedNode.operator === 'normalize'"><label class="pipeline-check"><input v-model="selectedNode.config.trim" type="checkbox" />{{ t('wfTrimWhitespace') }}</label><label class="pipeline-check"><input v-model="selectedNode.config.collapse_whitespace" type="checkbox" />{{ t('wfCollapseWhitespace') }}</label><p>{{ t('wfNormalizeDerivedOnly') }}</p></template><p v-else>{{ selectedNode.operator === 'source' ? t('wfSourceNodeHint') : t('wfArtifactNodeHint') }}</p></div></aside>
      </div>
      <section class="pipeline-result"><header><strong>{{ t('wfTokenizerPreview') }}</strong><select v-model="segmentId" :aria-label="t('wfSelectTokenizerInput')" :disabled="busy"><option v-for="segment in sourceSegments" :key="segment.id" :value="segment.id">{{ segment.order + 1 }} · {{ segment.text.slice(0, 60) }}</option></select></header><p v-if="!artifact">{{ t('wfSelectRunPreview') }}</p><template v-else><small>{{ t('wfInputRevision', { p0: artifact.input_revision_id }) }} · {{ artifact.tokenizer.implementation }} {{ artifact.tokenizer.version }}</small><div class="pipeline-tokens"><span v-for="(token, index) in artifact.tokens" :key="`${artifact.artifact_id}:${index}`" :title="`UTF-8 ${token.start_utf8}–${token.end_utf8}`">{{ token.text }}</span></div></template></section>
    </template>
    </template>
  </section>
</template>

<style scoped>
.pipeline-surfaces { display: flex; flex: none; gap: 6px; padding: 8px 20px; border-bottom: 1px solid var(--line); background: var(--surface-raised); }.pipeline-surfaces button[aria-pressed="true"] { color: var(--green-900); background: var(--surface-green-soft); border-color: var(--green-700); }.pipeline-workspace > .pipeline-v2 { flex: 1; height: auto; min-height: 0; }
.pipeline-workspace { display: flex; height: 100%; min-height: 0; flex-direction: column; color: var(--ink-900); background: var(--surface-app); }.pipeline-header { display: flex; min-height: 74px; align-items: center; gap: 10px; padding: 12px 22px; border-bottom: 1px solid var(--line); background: var(--surface-raised); }.pipeline-title { display: flex; gap: 12px; align-items: center; margin-right: auto; color: var(--green-900); }.pipeline-title h2 { margin: 0; font-size: 19px; }.pipeline-title p { margin: 3px 0 0; color: var(--ink-500); font-size: 11px; }.method-version { font-size: 11px; color: var(--ink-500); }.pipeline-workspace button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; min-height: 32px; padding: 6px 10px; border: 1px solid var(--line); border-radius: 6px; background: var(--surface-raised); font-size: 12px; cursor: pointer; }.pipeline-workspace button.pipeline-run { color: var(--paper); background: var(--green-700); border-color: var(--green-700); }.pipeline-word-form input { min-width: 0; padding: 7px 9px; border: 1px solid var(--line); border-radius: 6px; background: var(--surface-input); color: var(--ink-900); font-size: 12px; }.pipeline-empty { margin: auto; padding: 30px; text-align: center; color: var(--ink-500); max-width: 480px; }.pipeline-empty h3 { color: var(--ink-900); font-size: 18px; }.pipeline-empty p { font-size: 13px; line-height: 1.8; }.pipeline-method-bar { display: flex; align-items: center; gap: 16px; padding: 12px 20px; font-size: 11px; color: var(--ink-500); border-bottom: 1px solid var(--line); }.pipeline-body { flex: 1; min-height: 220px; display: grid; grid-template-columns: minmax(0, 1fr) 240px; }.pipeline-canvas { position: relative; background-image: radial-gradient(var(--line) 1px, transparent 1px); background-size: 18px 18px; min-width: 0; }.pipeline-fit { position: absolute; left: 14px; bottom: 14px; }.pipeline-node { width: 200px; padding: 18px; border: 1px solid var(--line); border-radius: 12px; background: var(--surface-raised); box-shadow: 0 3px 12px rgb(35 55 38 / 5%); }.pipeline-node--selected { border: 2px solid var(--green-700); padding: 17px; }.pipeline-node__icon { width: 36px; height: 36px; display: grid; place-items: center; border-radius: 10px; color: var(--green-700); background: var(--surface-green-soft); }.pipeline-node strong, .pipeline-node small { display: block; }.pipeline-node strong { margin: 12px 0 5px; font-size: 14px; }.pipeline-node small { color: var(--ink-500); font-size: 11px; }.pipeline-canvas :deep(.vue-flow__edge-path) { stroke: var(--green-700); stroke-width: 1.5; }.pipeline-canvas :deep(.vue-flow__handle) { width: 6px; height: 6px; background: var(--green-700); border: 2px solid var(--surface-raised); }.pipeline-inspector { overflow-y: auto; padding: 20px 16px; border-left: 1px solid var(--line); background: var(--surface-raised); }.pipeline-eyebrow { color: var(--ink-500); font-size: 11px; }.pipeline-inspector h3 { margin: 10px 0 22px; font-size: 16px; }.pipeline-inspector h4 { margin: 24px 0 7px; font-size: 13px; }.pipeline-inspector p { color: var(--ink-500); font-size: 11px; line-height: 1.7; }.pipeline-check { display: flex; align-items: center; gap: 8px; padding: 8px 0; font-size: 12px; }.pipeline-check input { accent-color: var(--green-700); }.pipeline-word-form { display: flex; gap: 6px; }.pipeline-word-form input { width: 100%; }.pipeline-dictionary { padding: 0; list-style: none; }.pipeline-dictionary li { display: flex; justify-content: space-between; align-items: center; padding: 6px 0; border-bottom: 1px solid var(--line); font-size: 12px; }.pipeline-dictionary button { border: 0; }.pipeline-result { max-height: 32%; min-height: 150px; padding: 16px 20px; overflow-y: auto; border-top: 1px solid var(--line); background: var(--surface-raised); }.pipeline-result header { display: flex; gap: 15px; align-items: center; justify-content: space-between; margin-bottom: 14px; }.pipeline-result header strong { font-size: 13px; white-space: nowrap; }.pipeline-result select { max-width: 75%; }.pipeline-result p, .pipeline-result small { color: var(--ink-500); font-size: 11px; }.pipeline-tokens { display: flex; flex-wrap: wrap; gap: 5px; margin-top: 12px; }.pipeline-tokens span { padding: 5px 8px; border: 1px solid var(--line); border-radius: 5px; font-size: 12px; background: var(--surface-green-soft); color: var(--green-900); }.pipeline-notice { display: flex; justify-content: space-between; align-items: center; margin: 0; padding: 8px 16px; background: var(--surface-green-soft); font-size: 12px; line-height: 1.6; }
@media(max-width: 1300px) { .pipeline-body { grid-template-columns: minmax(0, 1fr) 205px; }.pipeline-header { padding-inline: 15px; }.method-version { display: none; } }
</style>
