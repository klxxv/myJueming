<script setup lang="ts">
import { computed } from "vue";
import { ArrowRight, Check, ChevronDown, CircleAlert, Download, HardDrive } from "@lucide/vue";
import type { FeatureSnapshot } from "../domain/research-types";
import { t } from "../i18n";
import type { EmbeddedSettingsMessageKey } from "../i18n/embedded-settings-messages";

export interface ResearchFeatureSettingsProps {
  feature: FeatureSnapshot | null;
  available: boolean;
  busy?: boolean;
}

export interface ResearchFeaturePreferences {
  default_similarity?: string;
  auto_locate?: boolean;
}

const props = defineProps<ResearchFeatureSettingsProps>();
const emit = defineEmits<{
  enable: [];
  disable: [];
  cancel: [];
  retry: [];
  open: [];
  preferences: [preferences: ResearchFeaturePreferences];
}>();

const msg = (key: EmbeddedSettingsMessageKey, params?: Record<string, string | number>) =>
  t(key, params);
const algorithmIds = ["fuzzy.edit_distance", "fuzzy.char_ngram"] as const;
const algorithms = computed(() => [
  { id: algorithmIds[0], label: msg("researchEditDistance") },
  { id: algorithmIds[1], label: msg("researchCharNgram") },
]);

const closing = computed(() => props.feature?.status === "cancelling" || props.feature?.status === "disabling");
const preparing = computed(() => props.feature?.status === "preparing");
const ready = computed(() => props.feature?.status === "ready" && props.feature.desired_enabled);
const controlsDisabled = computed(() => !props.available || !props.feature || props.busy === true || closing.value);
const preferencesDisabled = computed(() => controlsDisabled.value || preparing.value);
const unknownAlgorithm = computed(() => props.feature
  && !algorithmIds.some((algorithm) => algorithm === props.feature?.default_similarity));
const canRetry = computed(() => props.feature?.reason?.retryable === true
  && (props.feature.status === "failed" || props.feature.status === "blocked"));

const stageLabelKeys: Record<string, EmbeddedSettingsMessageKey> = {
  resolving: "researchStageResolving",
  checking: "researchStageChecking",
  checking_resources: "researchStageChecking",
  downloading: "researchStageDownloading",
  verifying: "researchStageVerifying",
  installing: "researchStageInstalling",
  extracting: "researchStageExtracting",
  probing: "researchStageProbing",
  loading: "researchStageLoading",
  loading_model: "researchStageLoading",
  starting: "researchStageStarting",
};

const statusLabel = computed(() => {
  if (!props.available) return msg("researchUnavailable");
  const feature = props.feature;
  if (!feature) return msg("researchReadingStatus");
  switch (feature.status) {
    case "disabled": return msg(feature.resources_ready ? "researchDisabledReady" : "researchNotEnabled");
    case "preparing": return msg((feature.stage && stageLabelKeys[feature.stage]) || "researchPreparing");
    case "ready": return msg(feature.desired_enabled ? "researchReady" : "researchDisabled");
    case "cancelling": return msg("researchCancelling");
    case "disabling": return msg("researchDisabling");
    case "failed": return msg("researchFailed");
    case "blocked": return msg("researchBlocked");
  }
});

const statusDescription = computed(() => {
  const feature = props.feature;
  if (!feature) return msg("researchStatusAfterConnect");
  if (feature.reason) return feature.reason.message;
  if (ready.value) return msg("researchReadyDescription");
  if (preparing.value) return msg("researchPreparingDescription");
  if (closing.value) return msg("researchClosingDescription");
  if (feature.resources_ready) return msg("researchResourcesReadyDescription");
  return msg("researchFirstEnableDescription");
});

const workerLabel = computed(() => {
  const labelKeys: Record<string, EmbeddedSettingsMessageKey> = {
    stopped: "researchWorkerStopped",
    ready: "researchWorkerReady",
    idle: "researchWorkerIdle",
    starting: "researchWorkerStarting",
    busy: "researchWorkerRunning",
    running: "researchWorkerRunning",
    unloading: "researchWorkerUnloading",
    crashed: "researchWorkerCrashed",
    failed: "researchWorkerFailed",
  };
  return props.feature
    ? msg(labelKeys[props.feature.worker_state] ?? "researchWorkerUnknown")
    : msg("researchWaitingForApp");
});

function parseBytes(value: string | null | undefined): bigint | null {
  return value != null && /^\d+$/.test(value) ? BigInt(value) : null;
}

function formatBytes(bytes: bigint): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let divisor = 1n;
  let unit = 0;
  while (bytes >= divisor * 1024n && unit < units.length - 1) {
    divisor *= 1024n;
    unit += 1;
  }
  const whole = bytes / divisor;
  const decimal = (bytes % divisor) * 10n / divisor;
  return `${whole}${unit > 0 && decimal > 0n ? `.${decimal}` : ""} ${units[unit]}`;
}

const transfer = computed(() => {
  const completed = parseBytes(props.feature?.completed_bytes);
  const total = parseBytes(props.feature?.total_bytes);
  if (completed === null) return null;
  if (total === null || total === 0n) return { label: msg("researchPreparedBytes", { size: formatBytes(completed) }), percent: null };
  return {
    label: `${formatBytes(completed)} / ${formatBytes(total)}`,
    percent: Number((completed > total ? total : completed) * 100n / total),
  };
});

function toggleFeature(event: Event) {
  const input = event.target as HTMLInputElement;
  const requested = input.checked;
  input.checked = props.feature?.desired_enabled === true;
  if (controlsDisabled.value) return;
  if (requested) emit("enable");
  else if (preparing.value) emit("cancel");
  else emit("disable");
}

function chooseAlgorithm(event: Event) {
  const select = event.target as HTMLSelectElement;
  const selected = select.value;
  select.value = props.feature?.default_similarity ?? "";
  if (preferencesDisabled.value || !algorithmIds.some((algorithm) => algorithm === selected)) return;
  emit("preferences", { default_similarity: selected });
}

function toggleAutomaticLocation(event: Event) {
  const input = event.target as HTMLInputElement;
  const checked = input.checked;
  input.checked = props.feature?.auto_locate === true;
  if (!preferencesDisabled.value) emit("preferences", { auto_locate: checked });
}
</script>

<template>
  <div class="research-settings">
    <div class="research-card">
      <label class="feature-toggle">
        <span><strong>{{ msg("researchTitle") }}</strong><small>{{ msg("researchDescription") }}</small></span>
        <input
          class="research-switch"
          type="checkbox"
          :checked="feature?.desired_enabled === true"
          :disabled="controlsDisabled"
          aria-describedby="research-feature-status research-feature-description"
          @change="toggleFeature"
        />
      </label>
      <div class="feature-status" :class="{ 'feature-status--ready': ready, 'feature-status--error': feature?.status === 'failed' || feature?.status === 'blocked' }">
        <div class="feature-status-heading" role="status" aria-live="polite" aria-atomic="true">
          <Check v-if="ready" :size="17" aria-hidden="true" />
          <CircleAlert v-else-if="feature?.reason" :size="17" aria-hidden="true" />
          <Download v-else-if="preparing" :size="17" aria-hidden="true" />
          <HardDrive v-else :size="17" aria-hidden="true" />
          <strong id="research-feature-status">{{ statusLabel }}</strong>
        </div>
        <p id="research-feature-description">{{ statusDescription }}</p>
        <div v-if="preparing && transfer" class="feature-progress">
          <progress v-if="transfer.percent !== null" :value="transfer.percent" :max="100" :aria-label="msg('researchProgressAria')" :aria-valuetext="transfer.label" />
          <span>{{ transfer.label }}</span>
        </div>
        <div v-if="preparing || canRetry || ready" class="feature-actions">
          <button v-if="preparing" type="button" :disabled="busy || closing" @click="emit('cancel')">{{ msg("researchCancel") }}</button>
          <button v-if="canRetry" type="button" :disabled="controlsDisabled" @click="emit('retry')">{{ msg("researchRetry") }}</button>
          <button v-if="ready" class="feature-open" type="button" :disabled="busy" @click="emit('open')">{{ msg("researchOpen") }}<ArrowRight :size="16" aria-hidden="true" /></button>
        </div>
      </div>
    </div>

    <p class="feature-note">{{ msg("researchRetentionNote") }}</p>

    <details class="research-card feature-details">
      <summary><span>{{ msg("researchDefaultsAndResources") }}</span><ChevronDown :size="16" aria-hidden="true" /></summary>
      <div class="feature-details-body">
        <h3>{{ msg("researchNewTaskDefaults") }}</h3>
        <label class="preference-row">
          <span><strong>{{ msg("researchSimilarityAlgorithm") }}</strong><small>{{ msg("researchSimilarityDescription") }}</small></span>
          <select :value="feature?.default_similarity ?? ''" :disabled="preferencesDisabled" @change="chooseAlgorithm">
            <option v-if="!feature" value="" disabled>{{ msg("researchWaitingForFeature") }}</option>
            <option v-else-if="unknownAlgorithm" :value="feature.default_similarity" disabled>{{ msg("researchCurrentAlgorithm") }}</option>
            <option v-for="algorithm in algorithms" :key="algorithm.id" :value="algorithm.id">{{ algorithm.label }}</option>
          </select>
        </label>
        <label class="preference-row">
          <span><strong>{{ msg("researchAutoLocate") }}</strong><small>{{ msg("researchAutoLocateDescription") }}</small></span>
          <input class="research-switch" type="checkbox" :checked="feature?.auto_locate === true" :disabled="preferencesDisabled" @change="toggleAutomaticLocation" />
        </label>
        <p class="details-note">{{ msg("researchDefaultsNote") }}</p>
        <h3>{{ msg("researchLocalResources") }}</h3>
        <dl class="resource-summary">
          <div><dt>{{ msg("researchResourceStatus") }}</dt><dd>{{ !feature ? msg("researchWaitingForApp") : feature.resources_ready ? msg("researchResourcesReady") : msg("researchResourcesNeeded") }}</dd></div>
          <div><dt>{{ msg("researchWorker") }}</dt><dd>{{ workerLabel }}</dd></div>
        </dl>
        <p class="details-note">{{ msg("researchResourcesNote") }}</p>
      </div>
    </details>
  </div>
</template>

<style scoped>
.research-settings { color: var(--ink-900); }
.research-card { overflow: hidden; border: 1px solid var(--line); border-radius: 12px; background: var(--surface-raised); }
.feature-toggle, .preference-row { display: flex; min-height: 72px; align-items: center; justify-content: space-between; gap: 24px; padding: 15px 17px; }
.feature-toggle > span, .preference-row > span { min-width: 0; }
.feature-toggle strong, .preference-row strong { display: block; font-size: var(--jm-font-size-body); }
.feature-toggle small, .preference-row small { display: block; margin-top: 5px; color: var(--ink-500); font-size: var(--jm-font-size-callout); line-height: 1.5; }
.research-switch { flex: none; width: 38px; height: 22px; accent-color: var(--green-700); }
.feature-status { padding: 15px 17px; border-top: 1px solid var(--line); background: var(--surface-subtle); }
.feature-status-heading { display: flex; align-items: center; gap: 8px; color: var(--ink-700); font-size: var(--jm-font-size-callout); }
.feature-status--ready .feature-status-heading { color: var(--green-900); }
.feature-status--error { border-inline-start: 3px solid var(--ink-700); background: var(--surface-danger-soft); }
.feature-status > p { margin: 7px 0 0; color: var(--ink-500); font-size: var(--jm-font-size-callout); line-height: 1.5; overflow-wrap: anywhere; }
.feature-progress { display: grid; gap: 6px; margin-top: 14px; }
.feature-progress progress { display: block; width: 100%; height: 10px; accent-color: var(--green-700); }
.feature-progress > span { color: var(--ink-500); font-size: var(--jm-font-size-subheadline); font-variant-numeric: tabular-nums; }
.feature-actions { display: flex; flex-wrap: wrap; gap: 9px; margin-top: 14px; }
.feature-actions button { display: inline-flex; min-height: 36px; align-items: center; justify-content: center; gap: 8px; padding: 7px 12px; border: 1px solid var(--line); border-radius: 8px; background: var(--surface-raised); color: var(--ink-700); cursor: pointer; }
.feature-actions button:hover:enabled { background: var(--surface-hover); }
.feature-actions .feature-open { border-color: var(--green-700); color: var(--green-900); background: var(--surface-green-soft); }
.feature-actions button:disabled, .research-switch:disabled { opacity: .55; cursor: not-allowed; }
.feature-note, .details-note { color: var(--ink-500); font-size: var(--jm-font-size-callout); line-height: 1.5; }
.feature-note { margin: 10px 4px 20px; }
.feature-details summary { display: flex; min-height: 54px; align-items: center; justify-content: space-between; gap: 12px; padding: 12px 17px; color: var(--ink-700); cursor: pointer; font-size: var(--jm-font-size-body); font-weight: var(--jm-font-weight-semibold); list-style: none; }
.feature-details summary::-webkit-details-marker { display: none; }
.feature-details summary:hover { background: var(--surface-hover); }
.feature-details summary:focus-visible { outline: 2px solid var(--green-700); outline-offset: -3px; }
:global(html[data-enhanced-focus="on"]) .feature-details summary:focus-visible { outline-width: 3px; }
:global(html[data-larger-targets="on"]) .feature-actions button { min-height: 42px; }
.feature-details[open] summary { border-bottom: 1px solid var(--line); }
.feature-details[open] summary svg { transform: rotate(180deg); }
.feature-details-body h3 { margin: 18px 17px 3px; color: var(--ink-700); font-size: var(--jm-font-size-callout); }
.preference-row { gap: 18px; }
.preference-row + .preference-row { border-top: 1px solid var(--line); }
.preference-row select { flex: none; }
.details-note { margin: 2px 17px 17px; }
.resource-summary { margin: 0 17px 14px; }
.resource-summary > div { display: flex; justify-content: space-between; gap: 18px; padding: 10px 0; border-bottom: 1px solid var(--line); font-size: var(--jm-font-size-callout); }
.resource-summary dt { color: var(--ink-500); }
.resource-summary dd { margin: 0; color: var(--ink-700); text-align: right; }
@media (max-width: 760px) { .preference-row { flex-wrap: wrap; gap: 12px; } .preference-row > span { flex: 1 1 180px; } }
</style>
