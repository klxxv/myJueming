<script setup lang="ts">
import { onMounted, ref } from "vue";
import { agentRuntimeClient, type AgentRuntimeProviderKind, type AgentRuntimeStatus } from "../domain/agent-runtime-client";
const emit = defineEmits<{ changed: [status: AgentRuntimeStatus] }>();
const available = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const providerKind = ref<AgentRuntimeProviderKind>("loopback");
const endpoint = ref("http://127.0.0.1:11434/v1/");
const model = ref("");
const apiKey = ref("");
const secretConfigured = ref(false);
const busy = ref(false);
const notice = ref("");
const error = ref<string | null>(null);
onMounted(async () => {
  if (!available) return;
  try { const status = await agentRuntimeClient.status(); providerKind.value = status.provider_kind; endpoint.value = status.endpoint; model.value = status.model; secretConfigured.value = status.api_key_configured; }
  catch (cause) { error.value = cause instanceof Error ? cause.message : String(cause); }
});
async function configure() {
  if (!available || busy.value || !model.value.trim()) return;
  busy.value = true; error.value = null;
  try {
    const config = await agentRuntimeClient.configure({ provider_kind: providerKind.value, endpoint: endpoint.value.trim(), model: model.value.trim(), api_key: apiKey.value || undefined });
    apiKey.value = ""; secretConfigured.value = config.api_key_configured;
    notice.value = config.secret_storage === "session_memory" ? "配置已保存；此设备的密钥仅在本次会话保留" : "模型配置已保存";
    emit("changed", await agentRuntimeClient.status());
  } catch (cause) { error.value = cause instanceof Error ? cause.message : String(cause); }
  finally { busy.value = false; }
}
</script>

<template>
  <form class="model-settings" aria-label="内置助手模型" data-agent-context="exclude" @submit.prevent="configure">
    <h3>内置助手模型</h3>
    <p>连接你自己的模型服务。每次发送携带当前上下文，工程修改仍需审核。</p>
    <label>服务类型<select v-model="providerKind" :disabled="!available"><option value="loopback">本机模型 · OpenAI 兼容接口</option><option value="https">远程 HTTPS · OpenAI 兼容接口</option></select></label>
    <p v-if="providerKind === 'https'">发送的消息、选中内容和工具返回内容会交给你配置的服务。</p>
    <label>服务地址<input v-model="endpoint" type="url" :disabled="!available" placeholder="http://127.0.0.1:11434/v1/" required /></label>
    <label>模型名称<input v-model="model" :disabled="!available" placeholder="填写服务中实际可用的模型名称" required /></label>
    <label>API 密钥<input v-model="apiKey" type="password" autocomplete="new-password" :disabled="!available" :placeholder="secretConfigured ? '已设置 · 留空保留当前服务的密钥' : '本机服务可留空'" /></label>
    <button type="submit" :disabled="busy || !available || !model.trim()">{{ busy ? '保存中…' : '保存模型配置' }}</button>
    <p v-if="error" role="alert">{{ error }}</p><p v-if="notice" role="status">{{ notice }}</p>
  </form>
</template>

<style scoped>
.model-settings { display: grid; gap: 12px; padding: 16px 0 24px; border-top: 1px solid var(--line); }.model-settings h3 { margin: 0; font-size: 14px; }.model-settings p { margin: 0; color: var(--ink-500); font-size: 12px; line-height: 1.7; }.model-settings label { display: grid; grid-template-columns: 105px minmax(0, 1fr); align-items: center; gap: 12px; font-size: 12px; }.model-settings input, .model-settings select { min-width: 0; min-height: 35px; padding: 7px 9px; border: 1px solid var(--line); border-radius: 6px; color: var(--ink-900); background: var(--surface-input); }.model-settings button { justify-self: end; min-height: 34px; padding: 6px 12px; border: 1px solid var(--line); border-radius: 6px; color: var(--green-900); background: var(--surface-green-soft); cursor: pointer; font-size: 12px; }
</style>
