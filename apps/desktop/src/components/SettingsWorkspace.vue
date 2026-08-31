<script setup lang="ts">
import { t, uiLocale, UI_LOCALES, localeNames, formatDate, formatNumber } from '../i18n';
import { ChevronRight, Database, Keyboard, Minus, Monitor, Palette, Plus, Settings2 } from "@lucide/vue";
import { computed, ref } from "vue";
import type { AppTheme, CacheCleanupPolicy, ShortcutProfile } from "../composables/useAppSettings";

defineProps<{ usesMacShortcuts: boolean; shortcutRows: Array<[string, string]>; cacheCleaning: boolean; lastCacheCleanupAt: number | null }>();
const theme = defineModel<AppTheme>("theme", { required: true });
const fontScale = defineModel<number>("fontScale", { required: true });
const uiScale = defineModel<number>("uiScale", { required: true });
const shortcutProfile = defineModel<ShortcutProfile>("shortcutProfile", { required: true });
const trackpadOptimized = defineModel<boolean>("trackpadOptimized", { required: true });
const autoSaveDelayMs = defineModel<number>("autoSaveDelayMs", { required: true });
const cacheCleanupPolicy = defineModel<CacheCleanupPolicy>("cacheCleanupPolicy", { required: true });

type SettingsSection = "appearance" | "interaction" | "storage";
const activeSection = ref<SettingsSection>("appearance");
const sections = computed(() => [
  { id: "appearance" as const, label: t('settingsAppearance'), description: t('settingsAppearanceHint'), icon: Palette },
  { id: "interaction" as const, label: t('settingsInteraction'), description: t('settingsInteractionHint'), icon: Keyboard },
  { id: "storage" as const, label: t('settingsStorage'), description: t('settingsStorageHint'), icon: Database },
]);
const emit = defineEmits<{ applyUi: []; applyInteraction: []; applyPersistence: []; clearCache: [] }>();
</script>

<template>
  <section class="settings-view">
    <header class="settings-header"><span class="settings-header__icon"><Settings2 :size="20" /></span><div><h2>{{ t('navSettings') }}</h2><p>{{ t('settingsLocalOnly') }}</p></div></header>
    <div class="settings-split">
      <nav class="settings-sidebar" :aria-label="t('settingsCategories')">
        <button v-for="section in sections" :key="section.id" type="button" :class="{ active: activeSection === section.id }" @click="activeSection = section.id">
          <span class="settings-sidebar__icon"><component :is="section.icon" :size="18" /></span><span><strong>{{ section.label }}</strong><small>{{ section.description }}</small></span><ChevronRight :size="15" class="settings-sidebar__chevron" />
        </button>
        <p><Monitor :size="15" />{{ t('offlinePrivacy') }}</p>
      </nav>
      <div class="settings-detail">
        <section v-if="activeSection === 'appearance'" class="settings-pane">
          <header><p>{{ t('appearance') }}</p><h3>{{ t('settingsAppearance') }}</h3><span>{{ t('scaleDescription') }}</span></header>
          <div class="settings-group"><label class="setting-row"><span><strong>{{ t('interfaceLanguage') }}</strong><small>{{ t('interfaceLanguageHint') }}</small></span><select v-model="uiLocale" :aria-label="t('interfaceLanguage')"><option v-for="code in UI_LOCALES" :key="code" :value="code" :lang="code">{{ localeNames[code] }}</option></select></label></div>
          <div class="settings-group"><label class="setting-row"><span><strong>{{ t('theme') }}</strong><small>{{ t('themeDescription') }}</small></span><select v-model="theme" @change="emit('applyUi')"><option value="light">{{ t('themeLight') }}</option><option value="eye">{{ t('themeEye') }}</option></select></label></div>
          <div class="settings-group scale-group">
            <div class="scale-heading"><span><strong>{{ t('fontSize') }}</strong><small>{{ t('fontSizeHint') }}</small></span><output>{{ formatNumber(fontScale / 100, { style: 'percent' }) }}</output></div>
            <div class="stepped-slider"><Minus :size="17" /><input v-model.number="fontScale" :aria-label="t('fontSize')" type="range" min="90" max="130" step="10" @input="emit('applyUi')" /><Plus :size="17" /></div><div class="scale-labels"><span>{{ t('smaller') }}</span><span>{{ t('standard') }}</span><span>{{ t('larger') }}</span></div>
          </div>
          <div class="settings-group scale-group">
            <div class="scale-heading"><span><strong>{{ t('uiScale') }}</strong><small>{{ t('uiScaleHint') }}</small></span><output>{{ formatNumber(uiScale / 100, { style: 'percent' }) }}</output></div>
            <div class="stepped-slider"><Minus :size="17" /><input v-model.number="uiScale" :aria-label="t('uiScale')" type="range" min="85" max="115" step="5" @input="emit('applyUi')" /><Plus :size="17" /></div><div class="scale-labels"><span>{{ t('compact') }}</span><span>{{ t('default') }}</span><span>{{ t('spacious') }}</span></div>
          </div>
        </section>
        <section v-else-if="activeSection === 'interaction'" class="settings-pane">
          <header><p>{{ t('input') }}</p><h3>{{ t('settingsInteraction') }}</h3><span>{{ t('interactionDescription') }}</span></header>
          <div class="settings-group">
            <label class="setting-row"><span><strong>{{ t('shortcutLayout') }}</strong><small>{{ t('primaryModifierLabel') }}{{ usesMacShortcuts ? 'Command' : 'Ctrl' }}</small></span><select v-model="shortcutProfile" @change="emit('applyInteraction')"><option value="auto">{{ t('shortcutAuto') }}</option><option value="macos">{{ t('shortcutMac') }}</option><option value="windows">{{ t('shortcutWindows') }}</option></select></label>
            <label class="setting-row"><span><strong>{{ t('trackpad') }}</strong><small>{{ t('trackpadHint') }}</small></span><input v-model="trackpadOptimized" class="switch" type="checkbox" @change="emit('applyInteraction')" /></label>
          </div>
          <div class="settings-group shortcut-grid" :aria-label="t('shortcutHelp')"><template v-for="row in shortcutRows" :key="row[0]"><span>{{ row[0] }}</span><kbd>{{ row[1] }}</kbd></template></div>
        </section>
        <section v-else class="settings-pane">
          <header><p>{{ t('project') }}</p><h3>{{ t('settingsStorage') }}</h3><span>{{ t('historyPreservedHint') }}</span></header>
          <div class="settings-group">
            <label class="setting-row"><span><strong>{{ t('saveDelay') }}</strong><small>{{ t('saveDelayHint') }}</small></span><select v-model.number="autoSaveDelayMs" @change="emit('applyPersistence')"><option :value="1000">{{ t('delayOne') }}</option><option :value="3000">{{ t('delayThree') }}</option><option :value="5000">{{ t('delayFive') }}</option><option :value="10000">{{ t('delayTen') }}</option><option :value="30000">{{ t('delayThirty') }}</option></select></label>
            <label class="setting-row"><span><strong>{{ t('cacheCleanup') }}</strong><small>{{ t('cacheCleanupHint') }}</small></span><select v-model="cacheCleanupPolicy" @change="emit('applyPersistence')"><option value="startup">{{ t('cleanupStartup') }}</option><option value="weekly">{{ t('cleanupWeekly') }}</option><option value="monthly">{{ t('cleanupMonthly') }}</option><option value="never">{{ t('cleanupNever') }}</option></select></label>
          </div>
          <div class="settings-group cache-row"><span><strong>{{ t('clearCacheNow') }}</strong><small>{{ t('cacheHistorySafe') }}</small></span><div><button class="secondary-button" type="button" :disabled="cacheCleaning" @click="emit('clearCache')">{{ cacheCleaning ? t('clearingCache') : t('clearNow') }}</button><small v-if="lastCacheCleanupAt">{{ t('lastCleanupLabel') }}{{ formatDate(lastCacheCleanupAt) }}</small></div></div>
        </section>
      </div>
    </div>
  </section>
</template>

<style scoped>
.setting-row select { flex-shrink: 0; max-width: 55%; }
.settings-view { display: flex; flex-direction: column; width: 100%; height: 100%; min-height: 0; color: var(--ink-900); background: var(--surface-app); }
.settings-header { display: flex; flex: 0 0 74px; align-items: center; gap: 12px; padding: 0 24px; border-bottom: 1px solid var(--line); background: var(--surface-raised); }.settings-header__icon { display: grid; width: 36px; height: 36px; place-items: center; border-radius: 9px; color: #fff; background: var(--green-700); }.settings-header h2, .settings-header p { margin: 0; }.settings-header h2 { font-size: 18px; }.settings-header p { margin-top: 2px; color: var(--ink-500); font-size: 11px; }
.settings-split { display: grid; flex: 1; min-height: 0; grid-template-columns: 260px minmax(0, 1fr); }.settings-sidebar { display: flex; min-height: 0; flex-direction: column; gap: 6px; padding: 18px 12px; border-right: 1px solid var(--line); background: var(--surface-subtle); }.settings-sidebar button { display: grid; grid-template-columns: 34px minmax(0, 1fr) 16px; align-items: center; gap: 9px; min-height: 58px; padding: 8px 10px; border: 0; border-radius: 10px; color: var(--ink-700); background: transparent; text-align: left; cursor: pointer; }.settings-sidebar button:hover { background: var(--surface-hover); }.settings-sidebar button.active { color: var(--green-900); background: var(--surface-green-selected); }.settings-sidebar__icon { display: grid; width: 32px; height: 32px; place-items: center; border-radius: 8px; color: var(--green-700); background: var(--surface-green-soft); }.settings-sidebar strong, .settings-sidebar small { display: block; }.settings-sidebar strong { font-size: 13px; }.settings-sidebar small { margin-top: 3px; overflow: hidden; color: var(--ink-500); font-size: 10px; text-overflow: ellipsis; white-space: nowrap; }.settings-sidebar__chevron { color: var(--ink-500); }.settings-sidebar > p { display: flex; align-items: center; gap: 7px; margin: auto 8px 0; padding-top: 14px; border-top: 1px solid var(--line); color: var(--ink-500); font-size: 10px; }
.settings-detail { min-height: 0; overflow-y: auto; scrollbar-width: none; }.settings-detail::-webkit-scrollbar { display: none; }.settings-pane { width: min(720px, calc(100% - 64px)); margin: 0 auto; padding: 36px 0 48px; }.settings-pane > header { margin-bottom: 24px; }.settings-pane > header p { margin: 0 0 5px; color: var(--green-700); font-size: 10px; font-weight: 750; letter-spacing: .1em; }.settings-pane > header h3 { margin: 0; font-size: 23px; }.settings-pane > header span { display: block; margin-top: 7px; color: var(--ink-500); font-size: 12px; }
.settings-group { overflow: hidden; margin-bottom: 18px; border: 1px solid var(--line); border-radius: 12px; background: var(--surface-raised); box-shadow: 0 1px 2px rgb(31 53 34 / 3%); }.setting-row, .cache-row { display: flex; min-height: 68px; align-items: center; justify-content: space-between; gap: 24px; padding: 12px 16px; }.setting-row + .setting-row { border-top: 1px solid var(--line); }.setting-row > span, .cache-row > span, .scale-heading > span { min-width: 0; }.setting-row strong, .setting-row small, .cache-row strong, .cache-row small, .scale-heading strong, .scale-heading small { display: block; }.setting-row strong, .cache-row strong, .scale-heading strong { font-size: 13px; }.setting-row small, .cache-row small, .scale-heading small { margin-top: 4px; color: var(--ink-500); font-size: 10px; line-height: 1.4; }.setting-row select { min-width: 190px; height: 34px; padding: 0 30px 0 10px; border: 1px solid #c8d3c9; border-radius: 7px; color: var(--ink-900); background: var(--surface-input); }.switch { width: 38px; height: 22px; accent-color: var(--green-700); }
.scale-group { padding: 17px 20px 14px; }.scale-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 20px; }.scale-heading output { padding: 4px 8px; border-radius: 7px; color: var(--green-900); background: var(--surface-green-soft); font-size: 12px; font-weight: 650; }.stepped-slider { display: grid; grid-template-columns: 22px 1fr 22px; align-items: center; gap: 12px; margin-top: 18px; color: var(--ink-500); }.stepped-slider input { width: 100%; accent-color: var(--green-700); cursor: pointer; }.scale-labels { display: flex; justify-content: space-between; padding: 3px 34px 0; color: var(--ink-500); font-size: 9px; }
.shortcut-grid { display: grid; grid-template-columns: 1fr auto; }.shortcut-grid > span, .shortcut-grid kbd { padding: 10px 14px; border-bottom: 1px solid var(--line); font-size: 11px; }.shortcut-grid > :nth-last-child(-n + 2) { border-bottom: 0; }.shortcut-grid kbd { min-width: 180px; border-left: 1px solid var(--line); color: var(--green-900); background: var(--surface-subtle); font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; text-align: center; }.cache-row > div { display: grid; justify-items: end; gap: 6px; }.cache-row > div small { color: var(--ink-500); font-size: 9px; }.secondary-button { height: 34px; padding: 0 14px; border: 1px solid #bdcabf; border-radius: 7px; color: var(--green-900); background: var(--surface-raised); cursor: pointer; }.secondary-button:disabled { opacity: .55; cursor: wait; }
@media (max-width: 980px) { .settings-split { grid-template-columns: 220px minmax(0, 1fr); }.settings-pane { width: calc(100% - 40px); }.setting-row { gap: 14px; }.setting-row select { min-width: 160px; } }
</style>
