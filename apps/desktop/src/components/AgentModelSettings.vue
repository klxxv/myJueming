<script setup lang="ts">
import { onMounted, ref } from "vue";
import { agentRuntimeClient, type AgentRuntimeProviderKind, type AgentRuntimeStatus } from "../domain/agent-runtime-client";
import { t } from "../i18n";
import type { EmbeddedSettingsMessageKey } from "../i18n/embedded-settings-messages";
const emit = defineEmits<{ changed: [status: AgentRuntimeStatus] }>();
const available = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const providerKind = ref<AgentRuntimeProviderKind>("loopback");
const endpoint = ref("http://127.0.0.1:11434/v1/");
const model = ref("");
const apiKey = ref("");
const secretConfigured = ref(false);
const busy = ref(false);
const noticeKey = ref<EmbeddedSettingsMessageKey | null>(null);
const error = ref<string | null>(null);
const msg = (key: EmbeddedSettingsMessageKey, params?: Record<string, string | number>) =>
  t(key, params);
onMounted(async () => {
  if (!available) return;
  try { const status = await agentRuntimeClient.status(); providerKind.value = status.provider_kind; endpoint.value = status.endpoint; model.value = status.model; secretConfigured.value = status.api_key_configured; }
  catch (cause) { error.value = cause instanceof Error ? cause.message : String(cause); }
});
async function configure() {
  if (!available || busy.value || !model.value.trim()) return;
  busy.value = true; error.value = null; noticeKey.value = null;
  try {
    const config = await agentRuntimeClient.configure({ provider_kind: providerKind.value, endpoint: endpoint.value.trim(), model: model.value.trim(), api_key: apiKey.value || undefined });
    apiKey.value = ""; secretConfigured.value = config.api_key_configured;
    noticeKey.value = config.secret_storage === "session_memory" ? "modelSavedSessionSecret" : "modelSaved";
    emit("changed", await agentRuntimeClient.status());
  } catch (cause) { error.value = cause instanceof Error ? cause.message : String(cause); }
  finally { busy.value = false; }
}
</script>

<template>
  <form class="model-settings" :aria-label="msg('modelAria')" data-agent-context="exclude" @submit.prevent="configure">
    <h3>{{ msg("modelTitle") }}</h3>
    <p>{{ msg("modelDescription") }}</p>
    <label>{{ msg("modelServiceType") }}<select v-model="providerKind" :disabled="!available"><option value="loopback">{{ msg("modelLocalProvider") }}</option><option value="https">{{ msg("modelRemoteProvider") }}</option></select></label>
    <p v-if="providerKind === 'https'">{{ msg("modelRemoteDisclosure") }}</p>
    <label>{{ msg("modelEndpoint") }}<input v-model="endpoint" type="url" :disabled="!available" placeholder="http://127.0.0.1:11434/v1/" required /></label>
    <label>{{ msg("modelName") }}<input v-model="model" :disabled="!available" :placeholder="msg('modelNamePlaceholder')" required /></label>
    <label>{{ msg("modelApiKey") }}<input v-model="apiKey" type="password" autocomplete="new-password" :disabled="!available" :placeholder="secretConfigured ? msg('modelSecretConfigured') : msg('modelSecretOptional')" /></label>
    <button type="submit" :disabled="busy || !available || !model.trim()">{{ busy ? msg("modelSaving") : msg("modelSave") }}</button>
    <p v-if="error" role="alert">{{ error }}</p><p v-if="noticeKey" role="status">{{ msg(noticeKey) }}</p>
  </form>
</template>

<style scoped>
.model-settings { display: grid; gap: 12px; padding: 16px 0 24px; border-top: 1px solid var(--line); }.model-settings h3 { margin: 0; font-size: 14px; }.model-settings p { margin: 0; color: var(--ink-500); font-size: 12px; line-height: 1.7; }.model-settings label { display: grid; grid-template-columns: 105px minmax(0, 1fr); align-items: center; gap: 12px; font-size: 12px; }.model-settings input, .model-settings select { min-width: 0; min-height: 35px; padding: 7px 9px; border: 1px solid var(--line); border-radius: 6px; color: var(--ink-900); background: var(--surface-input); }.model-settings button { justify-self: end; min-height: 34px; padding: 6px 12px; border: 1px solid var(--line); border-radius: 6px; color: var(--green-900); background: var(--surface-green-soft); cursor: pointer; font-size: 12px; }
</style>
