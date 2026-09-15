<script setup lang="ts">
import { computed, ref, watch, type Component } from "vue";
import {
  Accessibility as AccessibilityIcon,
  ArrowLeft,
  ChevronRight,
  Database,
  Gauge,
  HardDrive,
  Info,
  Keyboard,
  Minus,
  Monitor,
  Palette,
  Plus,
  RotateCcw,
  Search,
  Settings2,
  ShieldCheck,
  SlidersHorizontal,
  X,
} from "@lucide/vue";
import type { AppSettingsEnvelope } from "../settings/schema";
import type { FeatureSnapshot } from "../domain/research-types";
import ResearchFeatureSettings, { type ResearchFeaturePreferences } from "./ResearchFeatureSettings.vue";
import { formatDate, t } from "../i18n";
import {
  isSettingsCapabilityAvailable,
  type SettingsCapabilities,
  type SettingsCapabilityId,
} from "../settings/capabilities";

type SettingsSectionId = SettingsCapabilityId | "research";
type SettingsSectionDefinition = {
  id: SettingsSectionId;
  group: "preferences" | "data" | "system";
  label: string;
  description: string;
  icon: Component;
};
type DetailId = "motion-global" | "interface-motion" | "motion-performance";
type SearchEntry = {
  label: string;
  path: string;
  section: SettingsSectionId;
  detail?: DetailId;
  keywords: string;
};

export interface SettingsWorkspaceResearchProps {
  researchFeature?: FeatureSnapshot | null;
  researchAvailable?: boolean;
  researchBusy?: boolean;
}

const settings = defineModel<AppSettingsEnvelope>("settings", { required: true });
const props = defineProps<{
  capabilities: SettingsCapabilities;
  usesMacShortcuts: boolean;
  shortcutRows: Array<[string, string]>;
  cacheCleaning: boolean;
  lastCacheCleanupAt: number | null;
  settingsSaving: boolean;
  settingsSaveError: string | null;
  effectiveMotionMode: "standard" | "reduced" | "off";
  motionSummary: string;
  systemReducedMotion: boolean;
  projectOpen: boolean;
  appVersion: string;
} & SettingsWorkspaceResearchProps>();
const emit = defineEmits<{
  applyGeneral: [];
  applyUi: [];
  applyAccessibility: [];
  applyInteraction: [];
  applyMotion: [];
  applyPersistence: [];
  clearCache: [];
  rememberSection: [section: string];
  resetAll: [];
  "research-enable": [];
  "research-disable": [];
  "research-cancel": [];
  "research-retry": [];
  "research-open": [];
  "research-preferences": [preferences: ResearchFeaturePreferences];
}>();

const sectionDefinitions = computed<SettingsSectionDefinition[]>(() => [
  { id: "account", group: "preferences", label: t("settingsAccount"), description: t("settingsAccountHint"), icon: Settings2 },
  { id: "general", group: "preferences", label: t("settingsGeneral"), description: t("settingsGeneralHint"), icon: SlidersHorizontal },
  { id: "appearance", group: "preferences", label: t("settingsAppearanceReading"), description: t("settingsAppearanceReadingHint"), icon: Palette },
  { id: "accessibility", group: "preferences", label: t("settingsAccessibility"), description: t("settingsAccessibilityHint"), icon: AccessibilityIcon },
  { id: "input", group: "preferences", label: t("settingsInput"), description: t("settingsInputHint"), icon: Keyboard },
  { id: "pet", group: "preferences", label: t("settingsPet"), description: t("settingsPetHint"), icon: Monitor },
  { id: "corpus", group: "data", label: t("settingsCorpus"), description: t("settingsCorpusHint"), icon: Database },
  { id: "upload", group: "data", label: t("settingsUpload"), description: t("settingsUploadHint"), icon: Database },
  { id: "community", group: "data", label: t("settingsCommunity"), description: t("settingsCommunityHint"), icon: Database },
  { id: "agent", group: "data", label: t("settingsAgent"), description: t("settingsAgentHint"), icon: Settings2 },
  { id: "research", group: "data", label: t("settingsResearch"), description: t("settingsResearchHint"), icon: Search },
  { id: "plugins", group: "data", label: t("settingsPlugins"), description: t("settingsPluginsHint"), icon: Settings2 },
  { id: "persistence", group: "data", label: t("settingsPersistence"), description: t("settingsPersistenceHint"), icon: Database },
  { id: "storage", group: "data", label: t("settingsStorageSpace"), description: t("settingsStorageSpaceHint"), icon: HardDrive },
  { id: "privacy", group: "data", label: t("settingsPrivacy"), description: t("settingsPrivacyHint"), icon: ShieldCheck },
  { id: "notifications", group: "data", label: t("settingsNotifications"), description: t("settingsNotificationsHint"), icon: Monitor },
  { id: "about", group: "system", label: t("settingsAbout"), description: t("settingsAboutHint"), icon: Info },
]);

const visibleSections = computed(() => sectionDefinitions.value.filter((section) =>
  section.id === "research"
    ? props.researchAvailable === true
    : isSettingsCapabilityAvailable(props.capabilities, section.id)));
const sectionGroups = computed(() => ([
  { id: "preferences" as const, label: t("settingsGroupPreferences") },
  { id: "data" as const, label: t("settingsGroupData") },
  { id: "system" as const, label: t("settingsGroupSystem") },
].map((group) => ({
  label: group.label,
  sections: visibleSections.value.filter((section) => section.group === group.id),
})).filter((group) => group.sections.length)));

const shortcutGroups = (shortcut: string) => shortcut.split(/\s+[／/·]\s+/).map((group) =>
  group.split(/\+|(?<=[⌃⌥⇧⌘])|(?=[⌃⌥⇧⌘])/u).filter(Boolean));
const shortcutKeyLabel = (key: string) => ({
  "⌘": "Command", "⇧": "Shift", "⌥": "Option", "⌃": "Control", "↵": "Return", Esc: "Escape",
}[key] ?? key);

const activeSection = ref<SettingsSectionId>("appearance");
const detail = ref<DetailId | null>(null);
const searchQuery = ref("");

watch([
  () => settings.value.device.navigation.lastSection,
  () => visibleSections.value.map((section) => section.id).join("|"),
], () => {
  const lastSection = settings.value.device.navigation.lastSection as SettingsSectionId;
  if (visibleSections.value.some((section) => section.id === lastSection)) activeSection.value = lastSection;
  if (!visibleSections.value.some((section) => section.id === activeSection.value)) {
    activeSection.value = visibleSections.value[0]?.id ?? "appearance";
  }
}, { immediate: true });

const activeDefinition = computed(() => visibleSections.value.find((section) => section.id === activeSection.value));
const searchEntries = computed<SearchEntry[]>(() => [
  { label: t("swInterfaceLanguage"), path: `${t("settingsGeneral")} > ${t("swLanguage")}`, section: "general", keywords: t("swSearchLanguage") },
  { label: t("swOpenOnStartup"), path: `${t("settingsGeneral")} > ${t("swStartupBehavior")}`, section: "general", keywords: t("swSearchStartup") },
  { label: t("swDefaultWorkspace"), path: `${t("settingsGeneral")} > ${t("swStartupBehavior")}`, section: "general", keywords: t("swSearchStartup") },
  { label: t("swMotionPerformance"), path: `${t("settingsGeneral")} > ${t("swPerformanceEnergy")}`, section: "general", detail: "motion-performance", keywords: t("swSearchMotion") },
  { label: t("swDisplayTheme"), path: `${t("settingsAppearanceReading")} > ${t("swDisplayTheme")}`, section: "appearance", keywords: t("swSearchAppearance") },
  { label: t("swFontSize"), path: `${t("settingsAppearanceReading")} > ${t("swReadingText")}`, section: "appearance", keywords: t("swSearchAppearance") },
  { label: t("swUiSpace"), path: `${t("settingsAppearanceReading")} > ${t("swInterfaceSize")}`, section: "appearance", keywords: t("swSearchAppearance") },
  { label: t("swLineSpacing"), path: `${t("settingsAppearanceReading")} > ${t("swReadingText")}`, section: "appearance", keywords: t("swSearchAppearance") },
  { label: t("swInterfaceAnimation"), path: `${t("settingsAppearanceReading")} > ${t("swInterfaceMotion")}`, section: "appearance", detail: "interface-motion", keywords: t("swSearchMotion") },
  { label: t("swMotionEffects"), path: `${t("settingsAccessibility")} > ${t("swMotionPerception")}`, section: "accessibility", detail: "motion-global", keywords: t("swSearchAccessibility") },
  { label: t("swEnhancedContrast"), path: `${t("settingsAccessibility")} > ${t("swVisualAssistance")}`, section: "accessibility", keywords: t("swSearchAccessibility") },
  { label: t("swEnhancedFocus"), path: `${t("settingsAccessibility")} > ${t("swVisualAssistance")}`, section: "accessibility", keywords: t("swSearchAccessibility") },
  { label: t("swLargerTargets"), path: `${t("settingsAccessibility")} > ${t("swVisualAssistance")}`, section: "accessibility", keywords: t("swSearchAccessibility") },
  { label: t("swShortcutLayout"), path: `${t("settingsInput")} > ${t("swInputDevices")}`, section: "input", keywords: t("swSearchInput") },
  { label: t("swTrackpadOptimized"), path: `${t("settingsInput")} > ${t("swInputDevices")}`, section: "input", keywords: t("swSearchInput") },
  { label: t("swAttentionPreference"), path: `${t("settingsPet")} > ${t("swPresentation")}`, section: "pet", keywords: t("swSearchGarden") },
  { label: t("settingsAgent"), path: `${t("settingsAgent")} > ${t("swGlobalAssistant")}`, section: "agent", keywords: t("swSearchAgent") },
  { label: t("swShareSelection"), path: `${t("settingsAgent")} > ${t("swGlobalAssistant")}`, section: "agent", keywords: t("swSearchAgent") },
  { label: t("settingsResearch"), path: t("settingsResearch"), section: "research", keywords: t("swSearchResearch") },
  { label: t("swSaveDelay"), path: `${t("settingsPersistence")} > ${t("swAutosave")}`, section: "persistence", keywords: t("swSearchPersistence") },
  { label: t("swAutomaticCleanup"), path: `${t("settingsStorageSpace")} > ${t("swCachePolicy")}`, section: "storage", keywords: t("swSearchStorage") },
  { label: t("swLocalMode"), path: `${t("settingsPrivacy")} > ${t("swLocalMode")}`, section: "privacy", keywords: t("swSearchPrivacy") },
  { label: t("swVersionInfo"), path: `${t("settingsAbout")} > ${t("swVersionInfo")}`, section: "about", keywords: t("swSearchAbout") },
]);
const searchResults = computed(() => {
  const query = searchQuery.value.trim().toLocaleLowerCase();
  if (!query) return [];
  const visibleIds = new Set(visibleSections.value.map((section) => section.id));
  return searchEntries.value.filter((entry) => visibleIds.has(entry.section)
    && `${entry.label} ${entry.path} ${entry.keywords}`.toLocaleLowerCase().includes(query));
});

function chooseSection(section: SettingsSectionId) {
  activeSection.value = section;
  detail.value = null;
  if (settings.value.device.general.rememberSettingsLocation) emit("rememberSection", section);
}

function openDetail(nextDetail: DetailId, sourceSection = activeSection.value) {
  activeSection.value = sourceSection;
  detail.value = nextDetail;
  if (settings.value.device.general.rememberSettingsLocation) emit("rememberSection", sourceSection);
}

function chooseSearchResult(entry: SearchEntry) {
  searchQuery.value = "";
  activeSection.value = entry.section;
  detail.value = entry.detail ?? null;
  if (settings.value.device.general.rememberSettingsLocation) emit("rememberSection", entry.section);
}

function requestReset() {
  if (window.confirm(t("swResetConfirm"))) emit("resetAll");
}

const formatCleanupTime = computed(() => props.lastCacheCleanupAt
  ? formatDate(props.lastCacheCleanupAt)
  : t("swNeverCleaned"));

const FONT_SCALE_MIN = 90;
const FONT_SCALE_MAX = 130;
const FONT_SCALE_STEP = 10;

function adjustFontScale(direction: -1 | 1) {
  const nextScale = Math.min(
    FONT_SCALE_MAX,
    Math.max(FONT_SCALE_MIN, settings.value.device.appearance.fontScale + direction * FONT_SCALE_STEP),
  );
  if (nextScale === settings.value.device.appearance.fontScale) return;
  settings.value.device.appearance.fontScale = nextScale;
  emit("applyUi");
}
</script>

<template>
  <section class="settings-view flex h-full min-h-0 w-full flex-col text-ink-900 bg-app" data-agent-context="exclude">
    <header class="settings-header flex basis-[70px] shrink-0 grow-0 items-center gap-[12px] px-[22px] py-0 border-0 border-b border-solid border-line bg-raised">
      <span class="settings-header__icon"><Settings2 :size="20" /></span>
      <div><h2>{{ t('navSettings') }}</h2><p>{{ t('settingsLocalOnly') }}</p></div>
      <span v-if="settingsSaving" class="settings-save-state">{{ t('settingsSaving') }}</span>
      <span v-else-if="settingsSaveError" class="settings-save-state settings-save-state--error">{{ t('saveFailed') }}</span>
      <span v-else class="settings-save-state">{{ t('settingsSavedDevice') }}</span>
    </header>

    <div class="settings-split grid flex-1 min-h-0 grid-cols-[248px_minmax(0,1fr)]">
      <nav class="settings-sidebar" :aria-label="t('settingsCategories')">
        <label class="settings-search">
          <Search :size="15" />
          <input v-model="searchQuery" type="search" :placeholder="t('searchSettings')" :aria-label="t('searchSettings')" />
          <button v-if="searchQuery" type="button" :title="t('clearSearch')" @click="searchQuery = ''"><X :size="14" /></button>
        </label>

        <div v-if="searchQuery" class="settings-search-results" aria-live="polite">
          <button v-for="entry in searchResults" :key="`${entry.section}-${entry.label}`" type="button" @click="chooseSearchResult(entry)">
            <span><strong>{{ entry.label }}</strong><small>{{ entry.path }}</small></span><ChevronRight :size="14" />
          </button>
          <p v-if="!searchResults.length">{{ t('noMatchingSettings') }}</p>
        </div>

        <template v-else v-for="group in sectionGroups" :key="group.label">
          <p class="settings-sidebar__group">{{ group.label }}</p>
          <button v-for="section in group.sections" :key="section.id" type="button" class="settings-sidebar__item" :class="{ active: activeSection === section.id }" @click="chooseSection(section.id)">
            <span class="settings-sidebar__icon"><component :is="section.icon" :size="17" /></span>
            <strong>{{ section.label }}</strong><ChevronRight :size="14" />
          </button>
        </template>

        <p class="settings-local-status"><Monitor :size="14" />{{ t('projectsStoredLocally') }}</p>
      </nav>

      <div class="settings-detail">
        <section v-if="detail" class="settings-pane settings-pane--detail">
          <button class="settings-back" type="button" @click="detail = null"><ArrowLeft :size="16" />{{ activeDefinition?.label }}</button>

          <template v-if="detail === 'motion-global'">
            <header><p>{{ t('swMotionGlobalPath') }}</p><h2>{{ t('swMotionEffects') }}</h2><span>{{ t('swMotionGlobalDescription') }}</span></header>
            <div class="settings-group">
              <label class="setting-row"><span><strong>{{ t('swGlobalMotionMode') }}</strong><small>{{ t('swCurrentEffectiveState', { state: effectiveMotionMode === 'standard' ? t('swStandard') : effectiveMotionMode === 'reduced' ? t('swReduced') : t('swOff') }) }}</small></span><select v-model="settings.device.motion.mode" @change="emit('applyMotion')"><option value="system">{{ t('swFollowSystemRecommended') }}</option><option value="standard">{{ t('swStandard') }}</option><option value="reduced">{{ t('swReduce') }}</option><option value="off">{{ t('swTurnOffNonessential') }}</option></select></label>
              <div class="setting-row setting-row--readonly"><span><strong>{{ t('swSystemPreference') }}</strong><small>{{ t('swSystemPreferenceHint') }}</small></span><output>{{ systemReducedMotion ? t('swSystemReduced') : t('swSystemNotReduced') }}</output></div>
            </div>
            <p class="settings-note">{{ t('swMotionStaticFeedbackNote') }}</p>
          </template>

          <template v-else-if="detail === 'interface-motion'">
            <header><p>{{ t('swInterfaceMotionPath') }}</p><h2>{{ t('swInterfaceAnimation') }}</h2><span>{{ t('swInterfaceMotionDescription') }}</span></header>
            <div class="settings-group">
              <label class="setting-row"><span><strong>{{ t('swPanelTransitions') }}</strong><small>{{ t('swPanelTransitionsHint') }}</small></span><input v-model="settings.device.motion.interfaceTransitions" class="switch" type="checkbox" @change="emit('applyMotion')" /></label>
              <label class="setting-row"><span><strong>{{ t('swSmoothNavigation') }}</strong><small>{{ t('swSmoothNavigationHint') }}</small></span><input v-model="settings.device.motion.smoothNavigation" class="switch" type="checkbox" @change="emit('applyMotion')" /></label>
              <label class="setting-row"><span><strong>{{ t('swHighlightFade') }}</strong><small>{{ t('swHighlightFadeHint') }}</small></span><input v-model="settings.device.motion.highlightFade" class="switch" type="checkbox" @change="emit('applyMotion')" /></label>
              <label class="setting-row"><span><strong>{{ t('swDragFeedback') }}</strong><small>{{ t('swDragFeedbackHint') }}</small></span><input v-model="settings.device.motion.dragFeedback" class="switch" type="checkbox" @change="emit('applyMotion')" /></label>
              <label class="setting-row"><span><strong>{{ t('swSuccessMotion') }}</strong><small>{{ t('swSuccessMotionHint') }}</small></span><input v-model="settings.device.motion.successMotion" class="switch" type="checkbox" @change="emit('applyMotion')" /></label>
            </div>
          </template>

          <template v-else>
            <header><p>{{ t('swMotionPerformancePath') }}</p><h2>{{ t('swMotionPerformance') }}</h2><span>{{ t('swMotionPerformanceDescription') }}</span></header>
            <div class="settings-group">
              <label class="setting-row"><span><strong>{{ t('swPauseWhenUnfocused') }}</strong><small>{{ t('swPauseWhenUnfocusedHint') }}</small></span><input v-model="settings.device.motion.pauseWhenUnfocused" class="switch" type="checkbox" @change="emit('applyMotion')" /></label>
              <div class="setting-row setting-row--readonly"><span><strong>{{ t('swCurrentMotionState') }}</strong><small>{{ motionSummary }}</small></span><output>{{ effectiveMotionMode === 'off' ? t('swPaused') : t('swRunningForeground') }}</output></div>
            </div>
          </template>
        </section>

        <section v-else-if="activeSection === 'general'" class="settings-pane">
          <header><p>{{ t('settingsGroupPreferences') }}</p><h2>{{ t('settingsGeneral') }}</h2><span>{{ t('settingsGeneralDescription') }}</span></header>
          <h3>{{ t('swLanguage') }}</h3>
          <div class="settings-group">
            <label class="setting-row">
              <span><strong>{{ t('swInterfaceLanguage') }}</strong><small>{{ t('swInterfaceLanguageHint') }}</small></span>
              <select v-model="settings.device.general.interfaceLanguage" :aria-label="t('swInterfaceLanguage')" @change="emit('applyGeneral')">
                <option value="system">{{ t('swFollowSystemRecommended') }}</option>
                <option value="zh-CN">中文（简体）</option>
                <option value="en">English</option>
                <option value="ja">日本語</option>
                <option value="fr">Français</option>
                <option value="de">Deutsch</option>
              </select>
            </label>
          </div>
          <h3>{{ t('swStartupBehavior') }}</h3>
          <div class="settings-group">
            <label class="setting-row"><span><strong>{{ t('swOpenOnStartup') }}</strong><small>{{ t('swOpenOnStartupHint') }}</small></span><select v-model="settings.device.general.startupDestination" @change="emit('applyGeneral')"><option value="welcome">{{ t('swWelcomePage') }}</option><option value="last-project">{{ t('swLastProject') }}</option><option value="project-picker">{{ t('swProjectPicker') }}</option></select></label>
            <label class="setting-row"><span><strong>{{ t('swDefaultWorkspace') }}</strong><small>{{ t('swDefaultWorkspaceHint') }}</small></span><select v-model="settings.device.general.defaultWorkspace" @change="emit('applyGeneral')"><option value="review">{{ t('swReview') }}</option><option value="edit">{{ t('swEdit') }}</option><option value="order">{{ t('swOrder') }}</option></select></label>
            <label class="setting-row"><span><strong>{{ t('swRememberLocation') }}</strong><small>{{ t('swRememberLocationHint') }}</small></span><input v-model="settings.device.general.rememberSettingsLocation" class="switch" type="checkbox" @change="emit('applyGeneral')" /></label>
          </div>
          <h3>{{ t('swPerformanceEnergy') }}</h3>
          <button class="settings-group settings-link-row" type="button" @click="openDetail('motion-performance', 'general')"><span><Gauge :size="18" /><span><strong>{{ t('swMotionPerformance') }}</strong><small>{{ t('swMotionPerformanceDescription') }}</small></span></span><span>{{ settings.device.motion.pauseWhenUnfocused ? t('swAutomatic') : t('swContinuous') }}<ChevronRight :size="15" /></span></button>
        </section>

        <section v-else-if="activeSection === 'appearance'" class="settings-pane">
          <header><p>{{ t('settingsGroupPreferences') }}</p><h2>{{ t('settingsAppearanceReading') }}</h2><span>{{ t('settingsAppearanceReadingHint') }}</span></header>
          <h3>{{ t('swDisplayTheme') }}</h3>
          <div class="settings-group"><label class="setting-row"><span><strong>{{ t('swDisplayTheme') }}</strong><small>{{ t('swDisplayThemeHint') }}</small></span><select v-model="settings.device.appearance.theme" @change="emit('applyUi')"><option value="system">{{ t('swFollowSystem') }}</option><option value="light">{{ t('swLight') }}</option><option value="dark">{{ t('swDark') }}</option><option value="eye">{{ t('swEyeCare') }}</option></select></label>
            <label v-if="settings.device.appearance.theme === 'system'" class="setting-row"><span><strong>{{ t('swSystemThemePair') }}</strong><small>{{ t('swSystemThemePairHint') }}</small></span><select v-model="settings.device.appearance.systemLightTheme" @change="emit('applyUi')"><option value="light">{{ t('swDarkLight') }}</option><option value="eye">{{ t('swDarkEye') }}</option></select></label>
          </div>
          <h3>{{ t('swReadingText') }}</h3>
          <div class="settings-group scale-group">
            <div class="scale-heading"><span><strong>{{ t('swFontSize') }}</strong><small>{{ t('swFontSizeHint') }}</small></span><output aria-live="polite">{{ settings.device.appearance.fontScale }}%</output></div>
            <div class="font-size-control">
              <button type="button" :aria-label="t('swDecreaseFont')" :title="t('swDecreaseFont')" :disabled="settings.device.appearance.fontScale <= FONT_SCALE_MIN" @click="adjustFontScale(-1)"><Minus :size="16" /></button>
              <input v-model.number="settings.device.appearance.fontScale" :aria-label="t('swFontSize')" :aria-valuetext="`${settings.device.appearance.fontScale}%`" type="range" :min="FONT_SCALE_MIN" :max="FONT_SCALE_MAX" :step="FONT_SCALE_STEP" @input="emit('applyUi')" />
              <button type="button" :aria-label="t('swIncreaseFont')" :title="t('swIncreaseFont')" :disabled="settings.device.appearance.fontScale >= FONT_SCALE_MAX" @click="adjustFontScale(1)"><Plus :size="16" /></button>
            </div>
          </div>
          <figure class="settings-group reading-preview" aria-labelledby="reading-preview-label">
            <figcaption id="reading-preview-label">{{ t('swPreview') }}</figcaption>
            <blockquote lang="ca">L’arquitecte és l’home sintètic, el que és capaç de veure les coses en conjunt abans que estiguin fetes.</blockquote>
            <cite>— Antoni Gaudí</cite>
          </figure>
          <div class="settings-group"><label class="setting-row"><span><strong>{{ t('swLineSpacing') }}</strong><small>{{ t('swLineSpacingHint') }}</small></span><select v-model="settings.device.appearance.lineSpacing" @change="emit('applyUi')"><option value="compact">{{ t('swCompact') }}</option><option value="standard">{{ t('swStandard') }}</option><option value="relaxed">{{ t('swRelaxed') }}</option></select></label></div>
          <h3>{{ t('swInterfaceSize') }}</h3>
          <div class="settings-group scale-group"><div class="scale-heading"><span><strong>{{ t('swUiSpace') }}</strong><small>{{ t('swUiSpaceHint') }}</small></span><output>{{ settings.device.appearance.uiScale }}%</output></div><div class="stepped-slider"><span>{{ t('swTight') }}</span><input v-model.number="settings.device.appearance.uiScale" :aria-label="t('swUiSpace')" type="range" min="85" max="115" step="5" @input="emit('applyUi')" /><span>{{ t('swLoose') }}</span></div></div>
          <div class="settings-group">
            <label class="setting-row"><span><strong>{{ t('swListDensity') }}</strong><small>{{ t('swListDensityHint') }}</small></span><select v-model="settings.device.appearance.listDensity" @change="emit('applyUi')"><option value="compact">{{ t('swCompact') }}</option><option value="standard">{{ t('swStandard') }}</option><option value="comfortable">{{ t('swRelaxed') }}</option></select></label>
            <label class="setting-row"><span><strong>{{ t('swSidebarSize') }}</strong><small>{{ t('swSidebarSizeHint') }}</small></span><select v-model="settings.device.appearance.sidebarSize" @change="emit('applyUi')"><option value="auto">{{ t('swAuto') }}</option><option value="compact">{{ t('swCompact') }}</option><option value="wide">{{ t('swWide') }}</option></select></label>
            <label class="setting-row"><span><strong>{{ t('swColumnBalance') }}</strong><small>{{ t('swColumnBalanceHint') }}</small></span><select v-model="settings.device.appearance.columnBalance" @change="emit('applyUi')"><option value="equal">{{ t('swEqual') }}</option><option value="source-wide">{{ t('swSourceWide') }}</option><option value="target-wide">{{ t('swTargetWide') }}</option></select></label>
            <label class="setting-row"><span><strong>{{ t('swAlignmentHints') }}</strong><small>{{ t('swAlignmentHintsHint') }}</small></span><input v-model="settings.device.appearance.alignmentHints" class="switch" type="checkbox" @change="emit('applyUi')" /></label>
          </div>
          <h3>{{ t('swInterfaceMotion') }}</h3>
          <button class="settings-group settings-link-row" type="button" @click="openDetail('interface-motion', 'appearance')"><span><Gauge :size="18" /><span><strong>{{ t('swInterfaceAnimation') }}</strong><small>{{ t('swInterfaceAnimationHint') }}</small></span></span><span>{{ motionSummary }}<ChevronRight :size="15" /></span></button>
        </section>

        <section v-else-if="activeSection === 'accessibility'" class="settings-pane">
          <header><p>{{ t('settingsGroupPreferences') }}</p><h2>{{ t('settingsAccessibility') }}</h2><span>{{ t('settingsAccessibilityHint') }}</span></header>
          <h3>{{ t('swMotionPerception') }}</h3>
          <button class="settings-group settings-link-row" type="button" @click="openDetail('motion-global', 'accessibility')"><span><AccessibilityIcon :size="18" /><span><strong>{{ t('swMotionEffects') }}</strong><small>{{ t('swMotionGlobalDescription') }}</small></span></span><span>{{ motionSummary }}<ChevronRight :size="15" /></span></button>
          <h3>{{ t('swVisualAssistance') }}</h3>
          <div class="settings-group">
            <label class="setting-row"><span><strong>{{ t('swEnhancedContrast') }}</strong><small>{{ t('swEnhancedContrastHint') }}</small></span><select v-model="settings.device.accessibility.contrast" @change="emit('applyAccessibility')"><option value="system">{{ t('swFollowSystem') }}</option><option value="standard">{{ t('swStandard') }}</option><option value="increased">{{ t('swIncreased') }}</option></select></label>
            <label class="setting-row"><span><strong>{{ t('swEnhancedFocus') }}</strong><small>{{ t('swEnhancedFocusHint') }}</small></span><input v-model="settings.device.accessibility.enhancedFocus" class="switch" type="checkbox" @change="emit('applyAccessibility')" /></label>
            <label class="setting-row"><span><strong>{{ t('swLargerTargets') }}</strong><small>{{ t('swLargerTargetsHint') }}</small></span><input v-model="settings.device.accessibility.largerTargets" class="switch" type="checkbox" @change="emit('applyAccessibility')" /></label>
            <label class="setting-row"><span><strong>{{ t('swReduceTransparency') }}</strong><small>{{ t('swReduceTransparencyHint') }}</small></span><input v-model="settings.device.accessibility.reduceTransparency" class="switch" type="checkbox" @change="emit('applyAccessibility')" /></label>
          </div>
        </section>

        <section v-else-if="activeSection === 'input'" class="settings-pane">
          <header><p>{{ t('settingsGroupPreferences') }}</p><h2>{{ t('settingsInput') }}</h2><span>{{ t('settingsInputHint') }}</span></header>
          <h3>{{ t('swInputDevices') }}</h3>
          <div class="settings-group">
            <label class="setting-row"><span><strong>{{ t('swShortcutLayout') }}</strong><small>{{ t('swPrimaryModifier', { modifier: usesMacShortcuts ? 'Command' : 'Ctrl' }) }}</small></span><select v-model="settings.device.input.shortcutProfile" @change="emit('applyInteraction')"><option value="auto">{{ t('swAutoDetectRecommended') }}</option><option value="macos">macOS · Command</option><option value="windows">Windows / Linux · Ctrl</option></select></label>
            <label class="setting-row"><span><strong>{{ t('swTrackpadOptimized') }}</strong><small>{{ t('swTrackpadOptimizedHint') }}</small></span><input v-model="settings.device.input.trackpadOptimized" class="switch" type="checkbox" @change="emit('applyInteraction')" /></label>
          </div>
          <h3>{{ t('swCurrentShortcuts') }}</h3>
          <div class="settings-group shortcut-grid" :aria-label="t('swCurrentShortcutsAria')">
            <div v-for="row in shortcutRows" :key="row[0]" class="shortcut-row">
              <span class="shortcut-action">{{ row[0] }}</span>
              <div class="shortcut-combinations">
                <template v-for="(keys, groupIndex) in shortcutGroups(row[1])" :key="groupIndex">
                  <span v-if="groupIndex" class="shortcut-separator" aria-hidden="true">/</span>
                  <kbd class="shortcut-combination" :aria-label="keys.map(shortcutKeyLabel).join(' + ')" :title="keys.map(shortcutKeyLabel).join(' + ')">
                    <span v-for="(key, keyIndex) in keys" :key="keyIndex" class="shortcut-key" :class="{ 'shortcut-key--symbol': /^[⌃⌥⇧⌘↵]$/u.test(key) }" aria-hidden="true">{{ key }}</span>
                  </kbd>
                </template>
              </div>
            </div>
          </div>
        </section>

        <section v-else-if="activeSection === 'pet'" class="settings-pane">
          <header><p>{{ t('settingsGroupPreferences') }}</p><h2>{{ t('swGardenTitle') }}</h2><span>{{ t('swGardenDescription') }}</span></header>
          <h3>{{ t('swPresentation') }}</h3>
          <div class="settings-group">
            <label class="setting-row"><span><strong>{{ t('swShowGarden') }}</strong><small>{{ t('swShowGardenHint') }}</small></span><input v-model="settings.device.pet.enabled" class="switch" type="checkbox" @change="emit('applyUi')" /></label>
            <label class="setting-row"><span><strong>{{ t('swAttentionPreference') }}</strong><small>{{ t('swAttentionPreferenceHint') }}</small></span><select v-model="settings.device.pet.presentation" @change="emit('applyUi')"><option value="static">{{ t('swGardenStatic') }}</option><option value="quiet">{{ t('swGardenQuiet') }}</option><option value="animated">{{ t('swGardenAnimated') }}</option><option value="hidden">{{ t('swGardenHidden') }}</option></select></label>
          </div>
          <h3>{{ t('swGardenMembers') }}</h3>
          <div class="settings-group">
            <label class="setting-row"><span><strong>{{ t('swOrangeCat') }}</strong><small>{{ t('swOrangeCatHint') }}</small></span><input v-model="settings.device.pet.catEnabled" class="switch" type="checkbox" @change="emit('applyUi')" /></label>
            <label class="setting-row"><span><strong>{{ t('swGoldenDog') }}</strong><small>{{ t('swGoldenDogHint') }}</small></span><input v-model="settings.device.pet.dogEnabled" class="switch" type="checkbox" @change="emit('applyUi')" /></label>
            <label class="setting-row"><span><strong>{{ t('swButterflyGuide') }}</strong><small>{{ t('swButterflyGuideHint') }}</small></span><input v-model="settings.device.pet.butterflyMotion" class="switch" type="checkbox" @change="emit('applyUi')" /></label>
            <label class="setting-row"><span><strong>{{ t('swQuietWhileEditing') }}</strong><small>{{ t('swQuietWhileEditingHint') }}</small></span><input v-model="settings.device.pet.quietWhileEditing" class="switch" type="checkbox" @change="emit('applyUi')" /></label>
          </div>
          <p class="settings-note">{{ t('swGardenNote') }}</p>
        </section>

        <section v-else-if="activeSection === 'agent'" class="settings-pane">
          <header><p>{{ t('settingsGroupData') }}</p><h2>{{ t('settingsAgent') }}</h2><span>{{ t('swAgentDescription') }}</span></header>
          <slot name="agent-settings" />
          <h3>{{ t('swGlobalAssistant') }}</h3>
          <div class="settings-group">
            <label class="setting-row"><span><strong>{{ t('swShowContext') }}</strong><small>{{ t('swShowContextHint') }}</small></span><input v-model="settings.device.agent.showContext" class="switch" type="checkbox" @change="emit('applyUi')" /></label>
            <label class="setting-row"><span><strong>{{ t('swShareSelection') }}</strong><small>{{ t('swShareSelectionHint') }}</small></span><input v-model="settings.device.agent.shareSelection" class="switch" type="checkbox" @change="emit('applyUi')" /></label>
            <label class="setting-row"><span><strong>{{ t('swOpenAssistantOnRequest') }}</strong><small>{{ t('swOpenAssistantOnRequestHint') }}</small></span><input v-model="settings.device.agent.openOnRequest" class="switch" type="checkbox" @change="emit('applyUi')" /></label>
          </div>
        </section>

        <section v-else-if="activeSection === 'research'" class="settings-pane">
          <header><p>{{ t('settingsGroupData') }}</p><h2>{{ t('settingsResearch') }}</h2><span>{{ t('swResearchDescription') }}</span></header>
          <ResearchFeatureSettings
            :feature="researchFeature ?? null"
            :available="researchAvailable === true"
            :busy="researchBusy === true"
            @enable="emit('research-enable')"
            @disable="emit('research-disable')"
            @cancel="emit('research-cancel')"
            @retry="emit('research-retry')"
            @open="emit('research-open')"
            @preferences="emit('research-preferences', $event)"
          />
        </section>

        <section v-else-if="activeSection === 'persistence'" class="settings-pane">
          <header><p>{{ t('settingsGroupData') }}</p><h2>{{ t('settingsPersistence') }}</h2><span>{{ t('swPersistenceDescription') }}</span></header>
          <h3>{{ t('swAutosave') }}</h3>
          <div class="settings-group"><label class="setting-row"><span><strong>{{ t('swSaveDelay') }}</strong><small>{{ t('swSaveDelayHint') }}</small></span><select v-model.number="settings.device.persistence.autoSaveDelayMs" @change="emit('applyPersistence')"><option :value="1000">{{ t('swSeconds', { count: 1 }) }}</option><option :value="3000">{{ t('swSecondsRecommended', { count: 3 }) }}</option><option :value="5000">{{ t('swSeconds', { count: 5 }) }}</option><option :value="10000">{{ t('swSeconds', { count: 10 }) }}</option><option :value="30000">{{ t('swSeconds', { count: 30 }) }}</option></select></label></div>
          <h3>{{ t('swRevisionHistory') }}</h3>
          <div class="settings-group">
            <div class="setting-row setting-row--readonly"><span><strong>{{ t('swFullHistory') }}</strong><small>{{ t('swFullHistoryHint') }}</small></span><output>{{ t('swAlwaysKeep') }}</output></div>
            <div class="setting-row setting-row--readonly"><span><strong>{{ t('swAutoDeleteHistory') }}</strong><small>{{ t('swRevisionNotCache') }}</small></span><output>{{ t('swUnavailable') }}</output></div>
          </div>
        </section>

        <section v-else-if="activeSection === 'storage'" class="settings-pane">
          <header><p>{{ t('settingsGroupData') }}</p><h2>{{ t('settingsStorageSpace') }}</h2><span>{{ t('swStorageDescription') }}</span></header>
          <h3>{{ t('swCachePolicy') }}</h3>
          <div class="settings-group"><label class="setting-row"><span><strong>{{ t('swAutomaticCleanup') }}</strong><small>{{ t('swAutomaticCleanupHint') }}</small></span><select v-model="settings.device.persistence.cacheCleanupPolicy" @change="emit('applyPersistence')"><option value="startup">{{ t('swEveryStartup') }}</option><option value="weekly">{{ t('swEvery7DaysRecommended') }}</option><option value="monthly">{{ t('swEvery30Days') }}</option><option value="never">{{ t('swNeverCleanup') }}</option></select></label></div>
          <h3>{{ t('swManualCleanup') }}</h3>
          <div class="settings-group cache-row"><span><strong>{{ t('swCleanDerivedCache') }}</strong><small>{{ projectOpen ? t('swLastCleanup', { date: formatCleanupTime }) : t('swOpenProjectToClean') }}</small></span><div><button class="secondary-button" type="button" :disabled="cacheCleaning || !projectOpen" @click="emit('clearCache')">{{ cacheCleaning ? t('swCleaning') : t('swCleanNow') }}</button><small>{{ t('swRevisionsPreserved') }}</small></div></div>
        </section>

        <section v-else-if="activeSection === 'privacy'" class="settings-pane">
          <header><p>{{ t('settingsGroupData') }}</p><h2>{{ t('settingsPrivacy') }}</h2><span>{{ t('swPrivacyDescription') }}</span></header>
          <h3>{{ t('swLocalMode') }}</h3>
          <div class="settings-group">
            <div class="setting-row setting-row--readonly"><span><strong>{{ t('swProjectText') }}</strong><small>{{ t('swProjectTextHint') }}</small></span><output>{{ t('swStoredLocally') }}</output></div>
            <div class="setting-row setting-row--readonly"><span><strong>{{ t('swExternalAiConnections') }}</strong><small>{{ t('swExternalAiConnectionsHint') }}</small></span><button class="secondary-button" type="button" @click="chooseSection('agent')">{{ t('swViewConnections') }}</button></div>
            <div class="setting-row setting-row--readonly"><span><strong>{{ t('swDiagnosticsUsage') }}</strong><small>{{ t('swDiagnosticsUsageHint') }}</small></span><output>{{ t('swNotSent') }}</output></div>
          </div>
        </section>

        <section v-else class="settings-pane">
          <header><p>{{ t('settingsGroupSystem') }}</p><h2>{{ t('settingsAbout') }}</h2><span>{{ t('swAboutDescription') }}</span></header>
          <h3>{{ t('swVersionInfo') }}</h3>
          <div class="settings-group">
            <div class="setting-row setting-row--readonly"><span><strong>{{ t('appName') }}</strong><small>Jueming Aligner</small></span><output>v{{ appVersion }}</output></div>
            <div class="setting-row setting-row--readonly"><span><strong>{{ t('swSettingsFormat') }}</strong><small>{{ t('swSettingsFormatHint') }}</small></span><output>Schema v{{ settings.schemaVersion }}</output></div>
            <div class="setting-row setting-row--readonly"><span><strong>{{ t('swDataMode') }}</strong><small>{{ t('swDataModeHint') }}</small></span><output>{{ t('swLocalFirst') }}</output></div>
          </div>
          <h3>{{ t('swRestore') }}</h3>
          <div class="settings-group reset-row"><span><strong>{{ t('swResetLocalSettings') }}</strong><small>{{ t('swResetLocalSettingsHint') }}</small></span><button class="secondary-button" type="button" @click="requestReset"><RotateCcw :size="15" />{{ t('swResetDefaults') }}</button></div>
        </section>
      </div>
    </div>
  </section>
</template>

<style scoped>


.settings-header__icon { display: grid; width: 34px; height: 34px; place-items: center; border-radius: 9px; color: #fff; @apply bg-accent-solid; }
.settings-header h2, .settings-header p { margin: 0; }.settings-header h2 { font-size: var(--jm-font-size-title-2); line-height: var(--jm-line-height-title-2); }.settings-header p { margin-top: 2px; @apply text-ink-500; font-size: var(--jm-font-size-callout); }
.settings-save-state { margin-left: auto; @apply text-accent; font-size: var(--jm-font-size-callout); }.settings-save-state--error { color: var(--text-danger, #a34f4f); }

.settings-sidebar { display: flex; min-height: 0; flex-direction: column; overflow-y: auto; padding: 14px 10px; border-right: 1px solid var(--line); @apply bg-subtle; scrollbar-width: thin; }
.settings-search { display: flex; flex: 0 0 34px; align-items: center; gap: 7px; margin: 0 2px 10px; padding: 0 9px; border: 1px solid var(--line); border-radius: 9px; @apply text-ink-500 bg-input; }
.settings-search:focus-within { border-color: #8fbf95; box-shadow: 0 0 0 3px rgb(47 129 67 / 12%); }.settings-search input { flex: 1; min-width: 0; border: 0; outline: 0; @apply text-ink-900; background: transparent; }.settings-search button { display: grid; padding: 2px; border: 0; place-items: center; @apply text-ink-500; background: transparent; cursor: pointer; }
.settings-sidebar__group { margin: 11px 10px 5px; @apply text-ink-500; font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-semibold); letter-spacing: .06em; }
.settings-sidebar__item { display: grid; grid-template-columns: 30px minmax(0, 1fr) 14px; min-height: 44px; align-items: center; gap: 8px; padding: 5px 9px; border: 0; border-radius: 9px; @apply text-ink-700; background: transparent; text-align: left; cursor: pointer; }.settings-sidebar__item:hover { @apply bg-hover; }.settings-sidebar__item.active { @apply text-accent-strong bg-green-selected; }.settings-sidebar__item strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--jm-font-size-body); }.settings-sidebar__icon { display: grid; width: 28px; height: 28px; place-items: center; border-radius: 7px; @apply text-accent bg-green-soft; }
.settings-local-status { display: flex; align-items: center; gap: 7px; margin: auto 7px 0; padding: 13px 3px 2px; border-top: 1px solid var(--line); @apply text-ink-500; font-size: var(--jm-font-size-subheadline); }
.settings-search-results { display: grid; gap: 3px; }.settings-search-results button { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 48px; padding: 7px 9px; border: 0; border-radius: 8px; background: transparent; text-align: left; cursor: pointer; }.settings-search-results button:hover { @apply bg-hover; }.settings-search-results strong, .settings-search-results small { display: block; }.settings-search-results small { margin-top: 2px; @apply text-ink-500; }.settings-search-results p { padding: 18px 10px; @apply text-ink-500; }
.settings-detail { min-height: 0; overflow-y: auto; scrollbar-gutter: stable both-edges; scrollbar-width: thin; }.settings-pane { width: min(760px, calc(100% - 56px)); margin: 0 auto; padding: 31px 0 52px; }.settings-pane > header { margin-bottom: 23px; }.settings-pane > header p { margin: 0 0 5px; @apply text-accent; font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-semibold); letter-spacing: .08em; }.settings-pane > header h2 { margin: 0; font-size: var(--jm-font-size-title-1); line-height: var(--jm-line-height-title-1); }.settings-pane > header span { display: block; margin-top: 7px; @apply text-ink-500; line-height: 1.5; }.settings-pane > h3 { margin: 22px 4px 8px; @apply text-ink-700; font-size: var(--jm-font-size-callout); font-weight: var(--jm-font-weight-semibold); }
.settings-back { display: inline-flex; align-items: center; gap: 6px; margin-bottom: 20px; padding: 5px 7px 5px 2px; border: 0; @apply text-accent; background: transparent; cursor: pointer; }.settings-back:hover { @apply text-accent-strong; }
.settings-group { overflow: hidden; width: 100%; margin: 0 0 12px; border: 1px solid var(--line); border-radius: 12px; @apply bg-raised; box-shadow: 0 1px 2px rgb(31 53 34 / 3%); }
.setting-row, .cache-row, .reset-row { display: flex; min-height: 64px; align-items: center; justify-content: space-between; gap: 24px; padding: 11px 15px; }.setting-row + .setting-row { border-top: 1px solid var(--line); }.setting-row > span, .cache-row > span, .reset-row > span, .scale-heading > span { min-width: 0; }.setting-row strong, .setting-row small, .cache-row strong, .cache-row small, .reset-row strong, .reset-row small, .scale-heading strong, .scale-heading small { display: block; }.setting-row strong, .cache-row strong, .reset-row strong, .scale-heading strong { font-size: var(--jm-font-size-body); }.setting-row small, .cache-row small, .reset-row small, .scale-heading small { margin-top: 4px; @apply text-ink-500; font-size: var(--jm-font-size-callout); line-height: 1.4; }.setting-row select { min-width: 190px; }.setting-row output { flex: none; @apply text-accent-strong; font-size: var(--jm-font-size-callout); }.setting-row--readonly { @apply bg-raised; }
.switch { flex: none; width: 38px; height: 22px; accent-color: var(--green-700); }.scale-group { padding: 16px 18px 13px; }.scale-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 20px; }.scale-heading output { padding: 4px 8px; border-radius: 7px; @apply text-accent-strong bg-green-soft; font-size: var(--jm-font-size-callout); font-weight: var(--jm-font-weight-semibold); }.stepped-slider, .font-size-control { display: grid; grid-template-columns: 24px 1fr 24px; align-items: center; gap: 11px; margin-top: 16px; @apply text-ink-500; text-align: center; }.stepped-slider input, .font-size-control input { width: 100%; accent-color: var(--green-700); cursor: pointer; }.font-size-control { grid-template-columns: 36px minmax(0, 1fr) 36px; }.font-size-control button { display: grid; width: 36px; height: 36px; padding: 0; place-items: center; border: 1px solid var(--line); border-radius: 9px; @apply text-accent-strong bg-input; cursor: pointer; }.font-size-control button:hover:not(:disabled) { border-color: #9dbca1; @apply bg-hover; }.reading-preview { padding: 15px 18px 17px; }.reading-preview figcaption { margin-bottom: 10px; @apply text-ink-500; font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-semibold); letter-spacing: .06em; }.reading-preview blockquote { margin: 0; @apply text-ink-900; font-family: var(--jm-font-ui); font-size: calc(16px * var(--reading-font-scale)); line-height: var(--jm-reading-line-height); }.reading-preview cite { display: block; margin-top: 8px; @apply text-accent; font-family: var(--jm-font-ui); font-size: calc(13px * var(--reading-font-scale)); font-style: normal; line-height: var(--jm-reading-line-height); text-align: right; }
.settings-link-row { display: flex; min-height: 64px; align-items: center; justify-content: space-between; gap: 20px; padding: 11px 15px; @apply text-ink-900; text-align: left; cursor: pointer; }.settings-link-row:hover { border-color: #b8ccb9; @apply bg-hover; }.settings-link-row > span { display: flex; align-items: center; gap: 10px; }.settings-link-row > span:last-child { flex: none; @apply text-ink-500; font-size: var(--jm-font-size-callout); }.settings-link-row strong, .settings-link-row small { display: block; }.settings-link-row small { margin-top: 4px; @apply text-ink-500; }
.shortcut-grid { display: flex; flex-direction: column; }
.shortcut-row { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 10px 24px; min-height: 50px; padding: 10px 16px; box-sizing: border-box; }
.shortcut-row + .shortcut-row { border-top: 1px solid var(--line); }
.shortcut-action { @apply text-ink-900; font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }
.shortcut-combinations { display: flex; flex-wrap: wrap; align-items: center; justify-content: flex-end; gap: 10px; margin-left: auto; }
.shortcut-combination { display: inline-flex; align-items: center; gap: 4px; margin: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
.shortcut-key { display: inline-flex; align-items: center; justify-content: center; min-width: 28px; height: 28px; padding: 0 6px; box-sizing: border-box; border: 1px solid var(--line); border-radius: 6px; @apply text-ink-900 bg-subtle; font-size: 13px; font-weight: 500; line-height: 1; white-space: nowrap; }
.shortcut-key--symbol { font-size: 18px; font-weight: 400; }
.shortcut-separator { @apply text-ink-500; font-size: 12px; }
.cache-row > div { display: grid; justify-items: end; gap: 6px; }.cache-row > div small { @apply text-ink-500; font-size: var(--jm-font-size-subheadline); }.secondary-button { display: inline-flex; height: 34px; align-items: center; gap: 6px; padding: 0 13px; border: 1px solid #bdcabf; border-radius: 7px; @apply text-accent-strong bg-raised; cursor: pointer; }.secondary-button:disabled { opacity: .5; cursor: not-allowed; }.settings-note { margin: -2px 4px 18px; @apply text-ink-500; font-size: var(--jm-font-size-callout); line-height: 1.55; }
@media (max-width: 1080px) { .settings-split { grid-template-columns: 210px minmax(0, 1fr); }.settings-pane { width: calc(100% - 36px); }.setting-row { gap: 12px; }.setting-row select { min-width: 158px; } }
</style>
