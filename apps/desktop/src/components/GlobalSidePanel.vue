<script setup lang="ts">
import { MessageSquareText, Sparkles, X } from "@lucide/vue";
import { nextTick } from "vue";
import { t } from "../i18n";
import type { PanelMessageKey } from "../i18n/panel-messages";

const activeTab = defineModel<"agent" | "annotations">("activeTab", { required: true });
defineProps<{ projectLabel: string; annotationCount: number; pendingCount: number }>();
const emit = defineEmits<{ close: [] }>();
const panelT: (key: PanelMessageKey, params?: Record<string, unknown>) => string = t;
async function navigateTabs(event: KeyboardEvent) {
  if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
  event.preventDefault();
  activeTab.value = event.key === "Home" ? "agent" : event.key === "End" ? "annotations" : activeTab.value === "agent" ? "annotations" : "agent";
  await nextTick();
  document.getElementById(`global-${activeTab.value}-tab`)?.focus();
}
</script>

<template>
  <section class="global-panel" :aria-label="panelT('globalPanelAria')" data-agent-context="exclude">
    <header class="global-panel__header">
      <div class="global-panel__tabs" role="tablist" :aria-label="panelT('globalPanelTablist')" @keydown="navigateTabs">
        <button id="global-agent-tab" type="button" role="tab" :tabindex="activeTab === 'agent' ? 0 : -1" :aria-selected="activeTab === 'agent'" aria-controls="global-agent-content" @click="activeTab = 'agent'"><Sparkles :size="16" />{{ panelT('globalPanelAgent') }}<span v-if="pendingCount" class="count">{{ pendingCount }}</span></button>
        <button id="global-annotations-tab" type="button" role="tab" :tabindex="activeTab === 'annotations' ? 0 : -1" :aria-selected="activeTab === 'annotations'" aria-controls="global-annotations-content" @click="activeTab = 'annotations'"><MessageSquareText :size="16" />{{ panelT('globalPanelAnnotations') }}<span class="count">{{ annotationCount }}</span></button>
      </div>
      <button class="global-panel__close" type="button" :aria-label="panelT('globalPanelCollapse')" @click="emit('close')"><X :size="17" /></button>
    </header>
    <p class="global-panel__scope" :title="projectLabel">{{ projectLabel }}</p>
    <div id="global-agent-content" v-show="activeTab === 'agent'" role="tabpanel" aria-labelledby="global-agent-tab" class="global-panel__content"><slot name="agent" /></div>
    <div id="global-annotations-content" v-show="activeTab === 'annotations'" role="tabpanel" aria-labelledby="global-annotations-tab" class="global-panel__content"><slot name="annotations" /></div>
  </section>
</template>

<style scoped>
.global-panel { height: 100%; min-height: 0; display: flex; flex-direction: column; border-left: 1px solid var(--line); color: var(--ink-900); background: var(--surface-raised); }
.global-panel__header { display: flex; align-items: center; justify-content: space-between; min-height: 53px; padding: 8px 12px; gap: 4px; border-bottom: 1px solid var(--line); }
.global-panel__tabs { display: flex; gap: 4px; min-width: 0; }
.global-panel button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; min-height: 34px; border: 0; border-radius: 7px; padding: 6px 10px; color: var(--ink-700); background: transparent; cursor: pointer; }
.global-panel button[aria-selected="true"] { color: var(--green-900); background: var(--surface-green-soft); font-weight: 600; }
.global-panel button:hover { background: var(--surface-hover); }
.count { font-size: 11px; color: var(--ink-500); }
.global-panel__scope { margin: 0; padding: 9px 15px; border-bottom: 1px solid var(--line); color: var(--ink-500); font-size: 11px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.global-panel__content { flex: 1; min-height: 0; overflow: hidden; background: var(--surface-subtle); }
.global-panel__content :deep(.annotation-heading) { display: none; }
.global-panel__content :deep(.annotation-workspace) { border: 0; box-shadow: none; }
.global-panel__content :deep(.annotation-filter) { padding-top: 13px; }
</style>
