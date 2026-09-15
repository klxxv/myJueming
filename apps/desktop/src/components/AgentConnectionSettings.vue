<script setup lang="ts">
import { onMounted, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { agentConnectionClient, type AgentConnectionInfo } from "../domain/agent-connection-client";
import { t } from "../i18n";
import type { EmbeddedSettingsMessageKey } from "../i18n/embedded-settings-messages";
const info = ref<AgentConnectionInfo | null>(null);
const command = ref("");
const busy = ref(false);
const error = ref<string | null>(null);
const noticeKey = ref<EmbeddedSettingsMessageKey | null>(null);
const msg = (key: EmbeddedSettingsMessageKey, params?: Record<string, string | number>) =>
  t(key, params);
const readableError = (cause: unknown) => cause instanceof Error ? cause.message : String(cause);
onMounted(async () => {
  if (!agentConnectionClient.available) return;
  try { info.value = await agentConnectionClient.status(); command.value = info.value.mcp_command ?? ""; }
  catch (cause) { error.value = readableError(cause); }
});
async function toggle() {
  if (!info.value || busy.value) return;
  busy.value = true; error.value = null; noticeKey.value = null;
  try { info.value = await agentConnectionClient.setEnabled(!info.value.status.enabled); command.value ||= info.value.mcp_command ?? ""; }
  catch (cause) { error.value = readableError(cause); }
  finally { busy.value = false; }
}
async function chooseBinary() {
  const selected = await open({ title: msg("connectionChooseDialog"), multiple: false, directory: false });
  if (typeof selected === "string") command.value = selected;
}
async function copyConfig() {
  error.value = null;
  try { const config = await agentConnectionClient.configuration(command.value); await navigator.clipboard.writeText(config); noticeKey.value = "connectionConfigCopied"; }
  catch (cause) { error.value = readableError(cause); }
}
</script>

<template>
  <section class="connection-settings" :aria-label="msg('connectionAria')" data-agent-context="exclude">
    <h3>{{ msg("connectionTitle") }}</h3>
    <p>{{ msg("connectionDescription") }}</p>
    <div class="connection-row"><span><strong>{{ msg("connectionAllowSession") }}</strong><small>{{ info?.status.enabled ? msg("connectionEnabledAt", { address: info.status.boundAddr ?? "—" }) : msg("connectionDisabled") }}</small></span><button type="button" :disabled="busy || !agentConnectionClient.available || !info" :aria-pressed="info?.status.enabled ?? false" @click="toggle">{{ busy ? msg("connectionProcessing") : info?.status.enabled ? msg("connectionDisable") : msg("connectionEnable") }}</button></div>
    <template v-if="info?.status.enabled">
      <label>{{ msg("connectionProgramLocation") }}<div class="connection-path"><input v-model="command" :aria-label="msg('connectionPathAria')" spellcheck="false" /><button type="button" @click="chooseBinary">{{ msg("connectionChoose") }}</button></div></label>
      <button type="button" :disabled="!command" @click="copyConfig">{{ msg("connectionCopyConfig") }}</button>
      <small class="connection-note">{{ msg("connectionNote") }}</small>
    </template>
    <p v-if="!agentConnectionClient.available">{{ msg("connectionBrowserUnavailable") }}</p>
    <p v-if="error" role="alert">{{ error }}</p><p v-if="noticeKey" role="status">{{ msg(noticeKey) }}</p>
  </section>
</template>

<style scoped>
.connection-settings { padding-bottom: 18px; @apply text-ink-900; }.connection-settings h3 { font-size: 14px; }.connection-settings p, .connection-note { font-size: 12px; @apply text-ink-500; line-height: 1.7; }.connection-row { display: flex; align-items: center; justify-content: space-between; gap: 14px; margin: 12px 0; padding: 14px; border: 1px solid var(--line); border-radius: 9px; @apply bg-raised; }.connection-row strong, .connection-row small { display: block; }.connection-row strong { font-size: 13px; }.connection-row small { margin-top: 5px; @apply text-ink-500; font-size: 11px; }.connection-settings button { min-height: 34px; padding: 6px 12px; border: 1px solid var(--line); border-radius: 6px; @apply bg-green-soft text-accent-strong; font-size: 12px; cursor: pointer; }.connection-settings label { display: block; font-size: 12px; margin: 14px 0; }.connection-path { display: flex; gap: 8px; margin-top: 6px; }.connection-path input { min-width: 0; flex: 1; padding: 8px; border: 1px solid var(--line); border-radius: 6px; @apply bg-input text-ink-900; }.connection-note { display: block; margin-top: 8px; }
</style>
