<script setup lang="ts">
import { computed } from "vue";
import { ArrowUp, ExternalLink, Focus, Square, Sparkles } from "@lucide/vue";
import AgentActionBar, { type ReviewableProposal } from "./AgentActionBar.vue";
import { t } from "../i18n";
import type { PanelMessageKey } from "../i18n/panel-messages";

export interface AgentMessageView { id: string; role: string; content: string }
export interface AgentActivityView { id: string; title: string; status: string }
const draft = defineModel<string>("draft", { default: "" });
const props = defineProps<{
  tabLabel: string;
  selectedText: string;
  connected: boolean;
  runtimeAvailable: boolean;
  running: boolean;
  reviewing: boolean;
  showContext: boolean;
  messages: AgentMessageView[];
  activity: AgentActivityView[];
  proposals: ReviewableProposal[];
  error: string | null;
}>();
const emit = defineEmits<{ send: [text: string]; cancel: []; approve: [id: string]; reject: [id: string]; settings: []; clearSelection: [] }>();
const panelT: (key: PanelMessageKey, params?: Record<string, unknown>) => string = t;
const canSend = computed(() => props.runtimeAvailable && !props.running && Boolean(draft.value.trim()));
function send() { if (canSend.value) emit("send", draft.value.trim()); }
function keydown(event: KeyboardEvent) {
  if (event.isComposing) return;
  if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) { event.preventDefault(); send(); }
}
</script>

<template>
  <section class="agent-panel" :aria-label="panelT('agentWorkspaceAria')" data-agent-context="exclude">
    <div v-if="showContext" class="agent-context">
      <span><Focus :size="13" />{{ panelT('agentCurrentPage', { p0: tabLabel }) }}</span>
      <button v-if="selectedText" type="button" :title="selectedText" :aria-label="panelT('agentRemoveSelectedText')" @click="emit('clearSelection')">{{ panelT('agentSelectedText', { p0: selectedText.slice(0, 34) }) }}<span aria-hidden="true"> ×</span></button>
      <small>{{ panelT('agentContextFixed') }}</small>
    </div>
    <div class="agent-panel__scroll">
      <div v-if="!messages.length && !activity.length" class="agent-welcome"><span class="agent-welcome__icon"><Sparkles :size="24" /></span><h3>{{ panelT('agentWelcomeTitle') }}</h3><p>{{ panelT('agentWelcomeDescription') }}</p><button type="button" @click="emit('settings')">{{ panelT('agentConfigureConnections') }} <ExternalLink :size="13" /></button></div>
      <article v-for="message in messages" :key="message.id" class="agent-message" :class="{ 'agent-message--user': message.role === 'user' }"><small>{{ message.role === 'user' ? panelT('agentRoleUser') : panelT('agentRoleAssistant') }}</small><p>{{ message.content }}</p></article>
      <ol v-if="activity.length" class="agent-activity" :aria-label="panelT('agentActivityLog')"><li v-for="item in activity.slice(-30)" :key="item.id"><span class="activity-dot" aria-hidden="true"></span><span>{{ item.title }}</span><small>{{ item.status }}</small></li></ol>
      <AgentActionBar v-for="proposal in proposals" :key="proposal.id" :proposal="proposal" :busy="reviewing" @approve="emit('approve', $event)" @reject="emit('reject', $event)" />
      <p v-if="error" class="agent-error" role="alert">{{ error }}</p>
    </div>
    <form class="agent-composer" @submit.prevent="send">
      <textarea v-model="draft" rows="3" :aria-label="panelT('agentMessageAria')" :placeholder="panelT(runtimeAvailable ? 'agentPromptAvailable' : 'agentPromptUnavailable')" :disabled="!runtimeAvailable" @keydown="keydown"></textarea>
      <div><small>{{ panelT(connected ? 'agentConnected' : 'agentConnectionNotReady') }}</small><button v-if="running" type="button" :aria-label="panelT('agentStop')" @click="emit('cancel')"><Square :size="15" /></button><button v-else type="submit" :disabled="!canSend" :aria-label="panelT('agentSend')"><ArrowUp :size="17" /></button></div>
      <p v-if="!runtimeAvailable" class="agent-provider-note">{{ panelT('agentExternalActivityVisible') }}</p>
    </form>
  </section>
</template>

<style scoped>
.agent-panel { display: flex; flex-direction: column; height: 100%; min-height: 0; @apply text-ink-900; }
.agent-context { display: flex; flex-wrap: wrap; align-items: center; gap: 7px; padding: 13px; border-bottom: 1px solid var(--line); }
.agent-context > span, .agent-context button { display: inline-flex; align-items: center; gap: 5px; max-width: 100%; padding: 5px 7px; border: 1px solid var(--line); border-radius: 6px; font-size: 11px; @apply text-accent-strong bg-green-soft; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.agent-context small { @apply text-ink-500; font-size: 11px; width: 100%; }
.agent-panel__scroll { flex: 1; min-height: 0; display: flex; flex-direction: column; gap: 14px; overflow-y: auto; padding: 16px; overscroll-behavior: contain; scrollbar-width: thin; }
.agent-welcome { margin: auto 0; padding: 20px 2px; }.agent-welcome__icon { display: grid; place-items: center; width: 44px; height: 44px; @apply text-accent bg-green-soft; border-radius: 14px; }
.agent-welcome h3 { font-size: 16px; font-weight: 600; margin: 16px 0 10px; }.agent-welcome p { @apply text-ink-500; font-size: 13px; line-height: 1.8; }
.agent-welcome button { display: inline-flex; gap: 6px; align-items: center; border: 0; padding: 8px 0; font-size: 12px; @apply text-accent-strong; background: transparent; cursor: pointer; }
.agent-message { padding: 10px 0; }.agent-message--user { padding: 12px; border-radius: 10px; @apply bg-subtle; }.agent-message small { font-size: 11px; @apply text-ink-500; }.agent-message p { margin: 7px 0 0; font-size: 13px; line-height: 1.8; white-space: pre-wrap; overflow-wrap: anywhere; }
.agent-activity { list-style: none; padding: 0; margin: 0; }.agent-activity li { display: flex; align-items: center; gap: 8px; padding: 9px 0; font-size: 12px; border-bottom: 1px solid var(--line); }.agent-activity small { margin-left: auto; @apply text-ink-500; font-size: 11px; }.activity-dot { width: 5px; height: 5px; flex: 0 0 5px; border-radius: 50%; @apply bg-accent; }
.agent-composer { margin: 12px; padding: 10px; border: 1px solid var(--line); border-radius: 11px; @apply bg-input; }.agent-composer textarea { display: block; width: 100%; resize: vertical; min-height: 64px; max-height: 180px; border: 0; outline: none; @apply text-ink-900; background: transparent; font: inherit; font-size: 13px; line-height: 1.65; }.agent-composer textarea:disabled { opacity: .65; }.agent-composer > div { display: flex; justify-content: space-between; align-items: center; gap: 6px; }.agent-composer small { @apply text-ink-500; font-size: 11px; }.agent-composer button { width: 32px; height: 32px; display: grid; place-items: center; border: 0; border-radius: 7px; @apply bg-accent-solid text-white; cursor: pointer; }.agent-composer button:disabled { opacity: .4; cursor: default; }.agent-provider-note { margin: 8px 0 0; font-size: 11px; @apply text-ink-500; }.agent-error { @apply text-ink-900; border-left: 3px solid var(--green-700); padding-left: 10px; font-size: 12px; line-height: 1.6; }
</style>
