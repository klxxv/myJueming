<script setup lang="ts">
import { onMounted, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { agentConnectionClient, type AgentConnectionInfo } from "../domain/agent-connection-client";
const info = ref<AgentConnectionInfo | null>(null);
const command = ref("");
const busy = ref(false);
const error = ref<string | null>(null);
const notice = ref("");
const readableError = (cause: unknown) => cause instanceof Error ? cause.message : String(cause);
onMounted(async () => {
  if (!agentConnectionClient.available) return;
  try { info.value = await agentConnectionClient.status(); command.value = info.value.mcp_command ?? ""; }
  catch (cause) { error.value = readableError(cause); }
});
async function toggle() {
  if (!info.value || busy.value) return;
  busy.value = true; error.value = null; notice.value = "";
  try { info.value = await agentConnectionClient.setEnabled(!info.value.status.enabled); command.value ||= info.value.mcp_command ?? ""; }
  catch (cause) { error.value = readableError(cause); }
  finally { busy.value = false; }
}
async function chooseBinary() {
  const selected = await open({ title: "选择 jueming-mcp 可执行文件", multiple: false, directory: false });
  if (typeof selected === "string") command.value = selected;
}
async function copyConfig() {
  error.value = null;
  try { const config = await agentConnectionClient.configuration(command.value); await navigator.clipboard.writeText(config); notice.value = "连接配置已复制，仅在本次应用会话内有效"; }
  catch (cause) { error.value = readableError(cause); }
}
</script>

<template>
  <section class="connection-settings" aria-label="外部 AI 连接" data-agent-context="exclude">
    <h3>外部 AI 工具</h3>
    <p>Codex 等工具可以通过 MCP 读取当前页面、搜索文本和提出修改。</p>
    <div class="connection-row"><span><strong>允许本次会话连接</strong><small>{{ info?.status.enabled ? `已开启 · ${info.status.boundAddr}` : '当前关闭' }}</small></span><button type="button" :disabled="busy || !agentConnectionClient.available || !info" :aria-pressed="info?.status.enabled ?? false" @click="toggle">{{ busy ? '处理中…' : info?.status.enabled ? '关闭连接' : '启用连接' }}</button></div>
    <template v-if="info?.status.enabled">
      <label>MCP 程序位置<div class="connection-path"><input v-model="command" aria-label="MCP 可执行文件路径" spellcheck="false" /><button type="button" @click="chooseBinary">选择</button></div></label>
      <button type="button" :disabled="!command" @click="copyConfig">复制 MCP JSON 配置</button>
      <small class="connection-note">将配置添加到支持 MCP 的客户端。重启决明后需要重新复制；关闭连接会断开现有访问。</small>
    </template>
    <p v-if="!agentConnectionClient.available">浏览器预览无法接受外部连接，请在桌面应用中配置。</p>
    <p v-if="error" role="alert">{{ error }}</p><p v-if="notice" role="status">{{ notice }}</p>
  </section>
</template>

<style scoped>
.connection-settings { padding-bottom: 18px; color: var(--ink-900); }.connection-settings h3 { font-size: 14px; }.connection-settings p, .connection-note { font-size: 12px; color: var(--ink-500); line-height: 1.7; }.connection-row { display: flex; align-items: center; justify-content: space-between; gap: 14px; margin: 12px 0; padding: 14px; border: 1px solid var(--line); border-radius: 9px; background: var(--surface-raised); }.connection-row strong, .connection-row small { display: block; }.connection-row strong { font-size: 13px; }.connection-row small { margin-top: 5px; color: var(--ink-500); font-size: 11px; }.connection-settings button { min-height: 34px; padding: 6px 12px; border: 1px solid var(--line); border-radius: 6px; background: var(--surface-green-soft); color: var(--green-900); font-size: 12px; cursor: pointer; }.connection-settings label { display: block; font-size: 12px; margin: 14px 0; }.connection-path { display: flex; gap: 8px; margin-top: 6px; }.connection-path input { min-width: 0; flex: 1; padding: 8px; border: 1px solid var(--line); border-radius: 6px; background: var(--surface-input); color: var(--ink-900); }.connection-note { display: block; margin-top: 8px; }
</style>
