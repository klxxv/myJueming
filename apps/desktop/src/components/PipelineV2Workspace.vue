<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { Check, Plus, Save, Square, Trash2 } from "@lucide/vue";
import { pipelineClient } from "../domain/pipeline-client";
import type { OperatorDescriptor, PortDescriptor, SchemaRef, SlotState } from "../domain/research-types";
import type { GraphRunV2, PipelineMethodV2, PlanNodeV2, PlanV2, PortRefV2, SchemaDefinitionV2 } from "../domain/pipeline-v2-types";
import { t, type LocalizedMessage } from "../i18n";

const props = defineProps<{ bindingId: string | null; projectId: string | null; revisionId: string; available: boolean; active: boolean; refreshKey: number }>();
const emit = defineEmits<{ dirty: [value: boolean]; status: [message: LocalizedMessage] }>();
const operators = ref<OperatorDescriptor[]>([]);
const slots = ref<SlotState[]>([]);
const methods = ref<PipelineMethodV2[]>([]);
const method = ref<PipelineMethodV2 | null>(null);
const editing = ref(false);
const name = ref("");
const plan = ref<PlanV2>({ format_version: 2, nodes: [], outputs: [] });
const configs = ref<Record<string, string>>({});
const baseline = ref("");
const operatorToAdd = ref("");
const working = ref(false);
const error = ref<string | null>(null);
const remoteChanged = ref(false);
const topology = ref<string[]>([]);
const validatedSignature = ref("");
const schemaDetail = ref<SchemaDefinitionV2 | null>(null);
const schemaLoading = ref(false);
const run = ref<GraphRunV2 | null>(null);
const root = ref<HTMLElement | null>(null);
let generation = 0;
let refreshGeneration = 0;
let schemaGeneration = 0;
let pollCount = 0;
let pollTimer: ReturnType<typeof setTimeout> | null = null;
const copy = <T,>(value: T): T => JSON.parse(JSON.stringify(value)) as T;
const readable = (cause: unknown) => cause instanceof Error ? cause.message : cause && typeof cause === "object" && "message" in cause ? String(cause.message) : String(cause);
const signature = () => JSON.stringify({ name: name.value, plan: plan.value, configs: configs.value });
const dirty = computed(() => editing.value && signature() !== baseline.value);
const running = computed(() => run.value?.status === "queued" || run.value?.status === "running");
const writable = computed(() => props.available && Boolean(props.projectId && props.bindingId));
const operatorMap = computed(() => new Map(operators.value.map(operator => [operator.operator_id, operator])));
const boundSlots = computed(() => slots.value.filter(slot => slot.state === "bound").length);
const validated = computed(() => Boolean(validatedSignature.value && validatedSignature.value === signature()));
const allOutputs = computed(() => plan.value.nodes.flatMap((node, index) => (operatorMap.value.get(node.operator_id)?.outputs ?? []).map(port => ({ ref: { node_id: node.node_id, port: port.name }, port, label: `${index + 1}. ${operatorMap.value.get(node.operator_id)?.name ?? node.operator_id} / ${port.name}` }))));
const sameRef = (left: PortRefV2, right: PortRefV2) => left.node_id === right.node_id && left.port === right.port;
const encodeRef = (ref: PortRefV2) => JSON.stringify([ref.node_id, ref.port]);
const schemaLabel = (schemas: SchemaRef[]) => schemas.map(schema => `${schema.name} v${schema.version}`).join(" | ");
const nodeLabel = (id: string) => { const index = plan.value.nodes.findIndex(node => node.node_id === id); const node = plan.value.nodes[index]; return node ? `${index + 1}. ${operatorMap.value.get(node.operator_id)?.name ?? node.operator_id}` : id; };
const statusLabels = computed<Record<string, string>>(() => ({ queued: t("wfQueued"), running: t("wfRunning"), completed: t("wfRunCompleted"), cancelled: t("wfStopped"), failed: t("wfRunFailed"), interrupted: t("wfRunInterrupted") }));
const stopPolling = () => { if (pollTimer !== null) clearTimeout(pollTimer); pollTimer = null; };
const sameScope = (epoch: number, binding: string | null) => generation === epoch && binding === props.bindingId;
watch([dirty, working], ([draftDirty, busy]) => emit("dirty", draftDirty || (busy && editing.value)));

function accept(next: PipelineMethodV2) {
  method.value = next; editing.value = true; name.value = next.name; plan.value = copy(next.plan);
  configs.value = Object.fromEntries(next.plan.nodes.map(node => [node.node_id, JSON.stringify(node.config, null, 2)]));
  baseline.value = signature(); remoteChanged.value = false; topology.value = []; validatedSignature.value = "";
}
function discardDraft() {
  if (working.value) { emit("status", () => t("wfWaitBeforeDraft")); return; }
  if (method.value) accept(method.value);
  else { editing.value = false; name.value = ""; plan.value = { format_version: 2, nodes: [], outputs: [] }; configs.value = {}; baseline.value = signature(); }
}
function newMethod() {
  if (!writable.value || dirty.value || working.value) return;
  method.value = null; name.value = t("wfNewDataFlowName"); plan.value = { format_version: 2, nodes: [], outputs: [] }; configs.value = {};
  baseline.value = ""; editing.value = true; run.value = null; topology.value = []; validatedSignature.value = ""; error.value = null;
}
async function load() {
  if (!writable.value || !props.bindingId) return;
  const epoch = generation; const request = ++refreshGeneration; const binding = props.bindingId;
  try {
    const [nextOperators, nextSlots, nextMethods] = await Promise.all([pipelineClient.v2.operators(), pipelineClient.v2.slots(), pipelineClient.v2.listMethods(binding)]);
    if (!sameScope(epoch, binding) || request !== refreshGeneration) return;
    operators.value = nextOperators; slots.value = nextSlots; methods.value = nextMethods;
    if (!nextOperators.some(operator => operator.operator_id === operatorToAdd.value)) operatorToAdd.value = nextOperators[0]?.operator_id ?? "";
    const next = nextMethods.find(item => item.method_id === method.value?.method_id) ?? nextMethods[0];
    if (dirty.value || working.value) { remoteChanged.value = Boolean(method.value && next && next.method_revision_id !== method.value.method_revision_id); return; }
    if (next) accept(next);
    error.value = null;
  } catch (cause) { if (sameScope(epoch, binding) && request === refreshGeneration) error.value = readable(cause); }
}
function selectMethod(event: Event) {
  if (dirty.value || working.value) return;
  const next = methods.value.find(item => item.method_id === (event.target as HTMLSelectElement).value);
  if (next) { stopPolling(); run.value = null; accept(next); }
}
function defaultConfig(operatorId: string): Record<string, unknown> {
  if (operatorId === "host.input.SegmentView" || operatorId === "host.input.TextView") return { view: "source" };
  return operatorId.startsWith("host.input.") ? { value: {} } : {};
}
function addNode() {
  const operator = operatorMap.value.get(operatorToAdd.value);
  if (!operator || !editing.value || working.value) return;
  const node: PlanNodeV2 = { node_id: crypto.randomUUID(), slot_id: operator.slots[0] ?? "", operator_id: operator.operator_id, inputs: {}, config: defaultConfig(operator.operator_id) };
  plan.value.nodes.push(node); configs.value[node.node_id] = JSON.stringify(node.config, null, 2);
  if (!plan.value.outputs.length && operator.outputs[0]) plan.value.outputs.push({ node_id: node.node_id, port: operator.outputs[0].name });
}
function changeOperator(node: PlanNodeV2, operatorId: string) {
  const operator = operatorMap.value.get(operatorId); if (!operator || working.value) return;
  node.operator_id = operatorId; node.slot_id = operator.slots.includes(node.slot_id) ? node.slot_id : operator.slots[0] ?? "";
  node.inputs = Object.fromEntries(Object.entries(node.inputs).filter(([port]) => operator.inputs.some(input => input.name === port)));
  node.config = defaultConfig(operatorId); configs.value[node.node_id] = JSON.stringify(node.config, null, 2);
}
function removeNode(nodeId: string) {
  plan.value.nodes = plan.value.nodes.filter(node => node.node_id !== nodeId);
  plan.value.outputs = plan.value.outputs.filter(output => output.node_id !== nodeId);
  delete configs.value[nodeId];
  for (const node of plan.value.nodes) for (const [port, refs] of Object.entries(node.inputs)) {
    const retained = refs.filter(ref => ref.node_id !== nodeId);
    if (retained.length) node.inputs[port] = retained; else delete node.inputs[port];
  }
}
function compatibleOutputs(node: PlanNodeV2, port: PortDescriptor) {
  return allOutputs.value.filter(output => output.ref.node_id !== node.node_id && output.port.schemas.every(schema => port.schemas.some(input => input.name === schema.name && input.version === schema.version)));
}
function setInput(node: PlanNodeV2, port: PortDescriptor, index: number, value: string) {
  const refs = [...(node.inputs[port.name] ?? [])];
  if (!value) refs.splice(index, 1);
  else { const [node_id, outputPort] = JSON.parse(value) as [string, string]; refs[index] = { node_id, port: outputPort }; }
  if (refs.length) node.inputs[port.name] = refs; else delete node.inputs[port.name];
}
function addInput(node: PlanNodeV2, port: PortDescriptor) {
  const existing = node.inputs[port.name] ?? [];
  const output = compatibleOutputs(node, port).find(candidate => !existing.some(ref => sameRef(ref, candidate.ref)));
  if (output) node.inputs[port.name] = [...existing, { ...output.ref }];
}
function toggleOutput(ref: PortRefV2, enabled: boolean) {
  plan.value.outputs = enabled ? [...plan.value.outputs.filter(output => !sameRef(output, ref)), { ...ref }] : plan.value.outputs.filter(output => !sameRef(output, ref));
}
function parsedPlan(): PlanV2 {
  return { ...copy(plan.value), nodes: plan.value.nodes.map(node => {
    let config: unknown;
    try { config = JSON.parse(configs.value[node.node_id] ?? "{}"); } catch { throw new Error(t("wfInvalidJson", { p0: nodeLabel(node.node_id) })); }
    if (!config || typeof config !== "object" || Array.isArray(config)) throw new Error(t("wfJsonObjectRequired", { p0: nodeLabel(node.node_id) }));
    return { ...copy(node), config: config as Record<string, unknown> };
  }) };
}
async function validate() {
  if (!writable.value || !props.bindingId || working.value) return false;
  const epoch = generation; const binding = props.bindingId; const sentSignature = signature();
  working.value = true; error.value = null;
  try {
    const order = await pipelineClient.v2.validatePlan(parsedPlan(), binding);
    if (!sameScope(epoch, binding) || sentSignature !== signature()) return false;
    topology.value = order; validatedSignature.value = sentSignature; emit("status", () => t("wfHostValidated")); return true;
  } catch (cause) { if (sameScope(epoch, binding)) { error.value = readable(cause); validatedSignature.value = ""; } return false; }
  finally { if (sameScope(epoch, binding)) working.value = false; }
}
async function saveDraft(): Promise<boolean> {
  if (!writable.value || !props.bindingId || working.value || !editing.value) return false;
  const epoch = generation; const binding = props.bindingId; const sentSignature = signature();
  working.value = true; error.value = null;
  try {
    if (!name.value.trim()) throw new Error(t("wfMethodNameRequired"));
    const nextPlan = parsedPlan();
    const order = await pipelineClient.v2.validatePlan(nextPlan, binding);
    if (!sameScope(epoch, binding)) return false;
    const next = await pipelineClient.v2.saveMethod({ ...(method.value ? { method_id: method.value.method_id, base_method_revision_id: method.value.method_revision_id } : {}), name: name.value.trim(), plan: nextPlan }, binding);
    if (!sameScope(epoch, binding)) return false;
    methods.value = [next, ...methods.value.filter(item => item.method_id !== next.method_id)];
    if (sentSignature === signature()) { accept(next); topology.value = order; validatedSignature.value = signature(); }
    else method.value = next;
    emit("status", () => t("wfDataFlowSaved")); return !dirty.value;
  } catch (cause) { if (sameScope(epoch, binding)) error.value = readable(cause); return false; }
  finally { if (sameScope(epoch, binding)) working.value = false; }
}
function schedulePoll() {
  stopPolling(); if (!props.active || !running.value || pollCount >= 600) return;
  pollTimer = setTimeout(() => { pollTimer = null; void refreshRun(); }, 1000);
}
async function refreshRun() {
  if (!run.value || !props.bindingId) return;
  const epoch = generation; const binding = props.bindingId; const runId = run.value.run_id;
  try {
    const next = await pipelineClient.v2.getRun(runId, binding);
    if (!sameScope(epoch, binding) || run.value?.run_id !== runId) return;
    run.value = next; pollCount++;
    if (next.error) error.value = next.error;
    if (pollCount >= 600 && running.value) error.value = t("wfPollingPausedManual");
    schedulePoll();
  } catch (cause) { if (sameScope(epoch, binding)) { error.value = readable(cause); stopPolling(); } }
}
async function start() {
  if (!writable.value || !props.bindingId || !method.value || dirty.value || working.value || running.value) return;
  const epoch = generation; const binding = props.bindingId; working.value = true; error.value = null; pollCount = 0;
  try {
    const next = await pipelineClient.v2.start({ method_id: method.value.method_id, method_revision_id: method.value.method_revision_id }, binding);
    if (!sameScope(epoch, binding)) return;
    run.value = { ...next, artifacts: [] }; await refreshRun();
  } catch (cause) { if (sameScope(epoch, binding)) error.value = readable(cause); }
  finally { if (sameScope(epoch, binding)) working.value = false; }
}
async function cancelRun() {
  if (!run.value || !props.bindingId) return;
  const epoch = generation; const binding = props.bindingId;
  try { await pipelineClient.v2.cancelRun(run.value.run_id, binding); if (sameScope(epoch, binding)) await refreshRun(); }
  catch (cause) { if (sameScope(epoch, binding)) error.value = readable(cause); }
}
async function showSchema(schema: SchemaRef) {
  const request = ++schemaGeneration; schemaLoading.value = true;
  try { const value = await pipelineClient.v2.schema(schema); if (request === schemaGeneration) schemaDetail.value = value; }
  catch (cause) { if (request === schemaGeneration) error.value = readable(cause); }
  finally { if (request === schemaGeneration) schemaLoading.value = false; }
}
async function revealNode(nodeId: string) {
  if (!plan.value.nodes.some(node => node.node_id === nodeId)) {
    if (dirty.value || working.value) return false;
    await load(); const target = methods.value.find(item => item.plan.nodes.some(node => node.node_id === nodeId));
    if (!target) return false; accept(target);
  }
  await nextTick(); const element = root.value?.querySelector<HTMLElement>(`[data-pipeline-node-id="${CSS.escape(nodeId)}"]`);
  element?.scrollIntoView({ block: "center", behavior: "instant" }); element?.focus(); return Boolean(element);
}
watch(() => [props.projectId, props.bindingId] as const, () => {
  generation++; refreshGeneration++; schemaGeneration++; stopPolling(); method.value = null; editing.value = false; name.value = ""; plan.value = { format_version: 2, nodes: [], outputs: [] }; configs.value = {}; baseline.value = ""; methods.value = []; operators.value = []; slots.value = []; run.value = null; error.value = null; working.value = false; schemaDetail.value = null; topology.value = []; validatedSignature.value = "";
  void load();
}, { immediate: true });
watch(() => props.refreshKey, () => { void load(); });
watch(() => props.active, active => { if (active) { void load(); if (running.value) { pollCount = 0; void refreshRun(); } } else stopPolling(); });
onBeforeUnmount(() => { generation++; schemaGeneration++; stopPolling(); emit("dirty", false); });
defineExpose({ hasDirtyDraft: () => dirty.value || working.value, saveDraft, discardDraft, revealNode, reload: load });
</script>

<template>
  <section ref="root" class="pipeline-v2" :aria-label="t('wfGeneralDataFlowEditor')">
    <div class="v2-toolbar"><div><h2>{{ t('wfDataFlow') }}</h2><p>{{ t('wfNamedPortsMethods') }}</p></div><button type="button" :disabled="!writable || dirty || working" @click="newMethod"><Plus :size="15" />{{ t('wfNewDataFlow') }}</button><button v-if="editing" type="button" :disabled="working || !dirty" @click="saveDraft"><Save :size="15" />{{ t('wfSaveMethod') }}</button><button v-if="editing" type="button" :disabled="working" @click="validate"><Check :size="15" />{{ t('wfValidateConnections') }}</button><button v-if="running" type="button" @click="cancelRun"><Square :size="14" />{{ t('wfStop') }}</button><button v-else-if="method" class="primary" type="button" :disabled="working || dirty || !writable" @click="start">{{ t('wfRunSavedVersion') }}</button></div>
    <div v-if="!writable" class="v2-empty">{{ t('wfDesktopRealProject') }}</div>
    <template v-else>
      <div class="v2-methods"><label v-if="methods.length">{{ t('wfSavedMethods') }} <select :value="method?.method_id ?? ''" :disabled="dirty || working" :aria-label="t('wfSavedDataFlows')" @change="selectMethod"><option v-if="!method" value="" disabled>{{ t('wfCreatingNewMethod') }}</option><option v-for="item in methods" :key="item.method_id" :value="item.method_id">{{ item.name }}</option></select></label><span>{{ t('wfBoundSlots', { p0: boundSlots, p1: slots.length }) }}</span><button type="button" :disabled="working" @click="load">{{ t('wfRefreshCapabilities') }}</button></div>
      <p v-if="error" class="v2-notice" role="alert">{{ error }}</p><p v-if="remoteChanged" class="v2-notice" role="status">{{ t('wfV2RemoteChanged') }}</p>
      <div class="v2-content"><div class="v2-editor">
        <div v-if="!editing" class="v2-empty">{{ t('wfAddNodesHint') }}</div>
        <template v-else><label class="v2-name">{{ t('wfMethodName') }}<input v-model="name" :disabled="working" :aria-label="t('wfDataFlowMethodName')" /></label><div class="v2-add"><select v-model="operatorToAdd" :disabled="working" :aria-label="t('wfAddOperator')"><option v-for="operator in operators" :key="operator.operator_id" :value="operator.operator_id">{{ operator.name }} · {{ operator.operator_id }}</option></select><button type="button" :disabled="working || !operatorToAdd" @click="addNode"><Plus :size="14" />{{ t('wfAddNode') }}</button></div>
          <article v-for="(node, index) in plan.nodes" :key="node.node_id" :data-pipeline-node-id="node.node_id" class="v2-node" tabindex="-1"><header><strong>{{ t('wfNodeNumber', { p0: index + 1 }) }}</strong><button type="button" :disabled="working" :aria-label="t('wfDeleteNodeNumber', { p0: index + 1 })" @click="removeNode(node.node_id)"><Trash2 :size="14" />{{ t('wfDelete') }}</button></header><label>{{ t('wfAlgorithmImplementation') }}<select :value="node.operator_id" :disabled="working" :aria-label="t('wfNodeAlgorithmAria', { p0: index + 1 })" @change="changeOperator(node, ($event.target as HTMLSelectElement).value)"><option v-if="!operatorMap.has(node.operator_id)" :value="node.operator_id">{{ node.operator_id }} · {{ t('wfCurrentlyUnavailable') }}</option><option v-for="operator in operators" :key="operator.operator_id" :value="operator.operator_id">{{ operator.name }} · {{ operator.operator_id }}</option></select></label><label>Slot<select v-model="node.slot_id" :disabled="working" :aria-label="t('wfNodeSlotAria', { p0: index + 1 })"><option v-for="slotId in operatorMap.get(node.operator_id)?.slots ?? [node.slot_id]" :key="slotId" :value="slotId">{{ slotId }}</option></select></label>
            <div v-for="port in operatorMap.get(node.operator_id)?.inputs ?? []" :key="port.name" class="v2-input"><div><strong>{{ port.name }}</strong><span>{{ t(port.required ? 'wfRequired' : 'wfOptional') }}{{ port.multiple ? ` · ${t('wfMultipleInputs')}` : '' }}</span><button v-for="schema in port.schemas" :key="`${schema.name}:${schema.version}`" type="button" class="schema-link" @click="showSchema(schema)">{{ schema.name }} v{{ schema.version }}</button></div><select v-for="(_, edgeIndex) in Math.max(1, (node.inputs[port.name] ?? []).length)" :key="edgeIndex" :value="node.inputs[port.name]?.[edgeIndex] ? encodeRef(node.inputs[port.name]![edgeIndex]!) : ''" :disabled="working" :aria-label="t('wfNodeInputAria', { p0: index + 1, p1: port.name, p2: edgeIndex + 1 })" @change="setInput(node, port, edgeIndex, ($event.target as HTMLSelectElement).value)"><option value="">{{ t(port.required ? 'wfChooseUpstream' : 'wfNoConnection') }}</option><option v-if="node.inputs[port.name]?.[edgeIndex] && !compatibleOutputs(node, port).some(output => sameRef(output.ref, node.inputs[port.name]![edgeIndex]!))" :value="encodeRef(node.inputs[port.name]![edgeIndex]!)">{{ t('wfIncompatibleConnection') }}</option><option v-for="output in compatibleOutputs(node, port)" :key="encodeRef(output.ref)" :value="encodeRef(output.ref)">{{ output.label }}</option></select><button v-if="port.multiple" type="button" :disabled="working || compatibleOutputs(node, port).length <= (node.inputs[port.name]?.length ?? 0)" @click="addInput(node, port)">{{ t('wfAddConnection') }}</button></div>
            <label>{{ t('wfNodeParamsJson') }}<textarea v-model="configs[node.node_id]" :disabled="working" :aria-label="t('wfNodeParamsJsonAria', { p0: index + 1 })" spellcheck="false" rows="4"></textarea></label><p v-if="node.operator_id.startsWith('host.input.')" class="v2-help">{{ t('wfHostInputHelp') }}</p><div class="v2-ports"><span v-for="port in operatorMap.get(node.operator_id)?.outputs ?? []" :key="port.name">{{ t('wfOutput') }} {{ port.name }} · {{ schemaLabel(port.schemas) }}<button v-for="schema in port.schemas" :key="`${schema.name}:${schema.version}`" class="schema-link" type="button" @click="showSchema(schema)">{{ t('wfViewContract') }}</button></span></div>
          </article>
          <fieldset class="v2-outputs"><legend>{{ t('wfPublishedOutputs') }}</legend><p v-if="!allOutputs.length">{{ t('wfSelectOutputsHint') }}</p><label v-for="output in allOutputs" :key="encodeRef(output.ref)"><input type="checkbox" :checked="plan.outputs.some(item => sameRef(item, output.ref))" :disabled="working" @change="toggleOutput(output.ref, ($event.target as HTMLInputElement).checked)" />{{ output.label }} · {{ schemaLabel(output.port.schemas) }}</label></fieldset><p v-if="validated" class="v2-validation" role="status">{{ t('wfHostValidationResult', { p0: topology.map(nodeLabel).join(' → ') }) }}</p><p v-else-if="topology.length" class="v2-help">{{ t('wfNeedsRevalidation') }}</p>
        </template>
      </div><aside class="v2-catalog" :aria-label="t('wfSlotSchemaContracts')"><details open><summary>{{ t('wfSlotRegistration') }}</summary><ul><li v-for="slot in slots" :key="slot.slot_id"><strong>{{ slot.slot_id }}</strong><span>{{ t(slot.state === 'bound' ? 'wfBound' : slot.state === 'unbound' ? 'wfUnbound' : slot.state === 'disabled' ? 'wfDisabled' : 'wfIncompatible') }}</span><p v-if="slot.reason">{{ slot.reason }}</p><button v-for="schema in [...slot.inputs, ...slot.outputs].flatMap(port => port.schemas).filter((value, index, values) => values.findIndex(other => other.name === value.name && other.version === value.version) === index)" :key="`${schema.name}:${schema.version}`" class="schema-link" type="button" @click="showSchema(schema)">{{ schema.name }} v{{ schema.version }}</button></li></ul></details><p v-if="schemaLoading" role="status">{{ t('wfReadingSchema') }}</p><section v-if="schemaDetail" class="v2-schema"><h3>{{ schemaDetail.name }} v{{ schemaDetail.version }}</h3><pre>{{ JSON.stringify(schemaDetail.schema, null, 2) }}</pre></section></aside></div>
      <section v-if="run" class="v2-run" :aria-label="t('wfDataFlowResults')"><header><strong>{{ statusLabels[run.status] ?? run.status }}</strong><span>{{ t('wfInputRevision', { p0: run.input_revision_id }) }} · {{ run.completed }} / {{ run.total }}</span><button type="button" @click="pollCount = 0; refreshRun()">{{ t('wfRefreshStatus') }}</button></header><p v-if="!run.artifacts.length">{{ t(running ? 'wfArtifactsAfterRun' : 'wfNoPublishedResults') }}</p><ul><li v-for="artifact in run.artifacts" :key="artifact.handle"><strong>{{ artifact.schema.name }} v{{ artifact.schema.version }}</strong><code>{{ artifact.handle }}</code><span>{{ artifact.bytes }} bytes · {{ t('wfInputRevision', { p0: artifact.input_revision_id }) }}</span><small>{{ artifact.provider_id }} · {{ artifact.provider_release }}</small></li></ul></section>
    </template>
  </section>
</template>

<style scoped>
.pipeline-v2 { display: flex; flex-direction: column; height: 100%; min-height: 0; @apply text-ink-900 bg-app; }.v2-toolbar, .v2-methods, .v2-add, .v2-node header, .v2-run header { display: flex; align-items: center; gap: 10px; }.v2-toolbar { flex-wrap: wrap; padding: 15px 20px; border-bottom: 1px solid var(--line); @apply bg-raised; }.v2-toolbar > div { margin-right: auto; }.v2-toolbar h2 { margin: 0; font-size: 18px; }.v2-toolbar p { margin: 4px 0 0; @apply text-ink-500; font-size: 11px; }button, input, textarea { font: inherit; @apply text-ink-900; font-size: 12px; }button { display: inline-flex; align-items: center; justify-content: center; gap: 5px; min-height: 32px; padding: 6px 10px; @apply bg-raised; border: 1px solid var(--line); border-radius: 6px; cursor: pointer; }button:disabled { opacity: .55; cursor: default; }.primary { @apply text-white border-accent bg-accent-solid; }input, textarea { min-width: 0; padding: 7px; border: 1px solid var(--line); border-radius: 5px; @apply bg-input; }textarea { resize: vertical; font-family: ui-monospace, monospace; line-height: 1.7; }.v2-methods { flex-wrap: wrap; padding: 10px 20px; border-bottom: 1px solid var(--line); font-size: 11px; @apply text-ink-500; }.v2-methods select { margin-left: 6px; }.v2-content { flex: 1; min-height: 0; display: grid; grid-template-columns: minmax(360px, 1fr) 270px; }.v2-editor { overflow: auto; padding: 18px 20px; }.v2-name, .v2-node > label { display: flex; flex-direction: column; gap: 7px; margin-bottom: 12px; font-size: 12px; }.v2-add { margin: 16px 0; }.v2-add select { flex: 1; width: 0; }.v2-node { padding: 14px; margin-bottom: 14px; border: 1px solid var(--line); border-radius: 10px; @apply bg-raised; outline-offset: 3px; }.v2-node header { justify-content: space-between; margin-bottom: 12px; font-size: 13px; }.v2-input { padding: 10px 0; border-top: 1px solid var(--line); }.v2-input > div { display: flex; gap: 7px; flex-wrap: wrap; align-items: center; font-size: 11px; margin-bottom: 7px; }.v2-input > select { display: block; width: 100%; margin-bottom: 6px; }.v2-input span, .v2-help, .v2-ports, .v2-empty { @apply text-ink-500; font-size: 11px; line-height: 1.8; }.v2-ports > span { display: block; }.schema-link { display: inline; min-height: 24px; padding: 2px 4px; border: 0; @apply text-accent-strong; font-size: 10px; background: transparent; text-align: left; }.v2-outputs { padding: 12px; border: 1px solid var(--line); border-radius: 8px; font-size: 12px; }.v2-outputs label { display: flex; align-items: center; gap: 8px; margin: 8px 0; overflow-wrap: anywhere; }.v2-outputs input { accent-color: var(--green-700); }.v2-validation { font-size: 12px; line-height: 1.8; @apply text-accent-strong; }.v2-catalog { overflow: auto; border-left: 1px solid var(--line); padding: 14px; @apply bg-raised; }.v2-catalog summary { cursor: pointer; font-size: 12px; }.v2-catalog ul, .v2-run ul { margin: 0; padding: 0; list-style: none; }.v2-catalog li { padding: 10px 0; border-bottom: 1px solid var(--line); }.v2-catalog li > strong { display: block; font-size: 11px; overflow-wrap: anywhere; }.v2-catalog li > span, .v2-catalog li > p { font-size: 11px; @apply text-ink-500; margin: 5px 0; }.v2-schema h3 { font-size: 12px; }.v2-schema pre { font-size: 10px; overflow: auto; max-height: 350px; }.v2-notice { padding: 10px 20px; margin: 0; @apply bg-green-soft; font-size: 12px; }.v2-empty { padding: 30px; }.v2-run { flex: none; max-height: 26%; min-height: 90px; padding: 12px 20px; overflow: auto; border-top: 1px solid var(--line); @apply bg-raised; font-size: 11px; }.v2-run header { flex-wrap: wrap; }.v2-run li { display: grid; grid-template-columns: 160px minmax(130px, 1fr); gap: 4px 12px; padding: 10px 0; border-top: 1px solid var(--line); margin-top: 10px; }.v2-run code { overflow-wrap: anywhere; }.v2-run small { @apply text-ink-500; }@media(max-width:1100px) { .v2-content { grid-template-columns: minmax(320px, 1fr) 220px; }.v2-editor { padding: 12px; }.v2-toolbar { padding: 12px; } }
</style>
