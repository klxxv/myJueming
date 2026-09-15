import { computed } from "vue";
import { storeToRefs } from "pinia";
import type { KernelClient } from "../domain/kernel-client";
import { useAppSettingsStore } from "../stores/app-settings";
import type { LocalizedMessage } from "../i18n";

export type {
  AppTheme,
  CacheCleanupPolicy,
  ShortcutProfile,
} from "../settings/schema";

type UseAppSettingsOptions = {
  detectedMacOS: boolean;
  kernelClient: Pick<KernelClient, "loadAppSettings" | "saveAppSettings" | "resetAppSettings" | "clearCache"> & Partial<Pick<KernelClient, "observeSystemTheme">>;
  onStatus: (message: LocalizedMessage) => void;
};

export function useAppSettings(options: UseAppSettingsOptions) {
  const store = useAppSettingsStore();
  store.configure(options);
  const refs = storeToRefs(store);

  const appearance = computed(() => refs.settings.value.device.appearance);
  const general = computed(() => refs.settings.value.device.general);
  const accessibility = computed(() => refs.settings.value.device.accessibility);
  const input = computed(() => refs.settings.value.device.input);
  const motion = computed(() => refs.settings.value.device.motion);
  const pet = computed(() => refs.settings.value.device.pet);
  const persistence = computed(() => refs.settings.value.device.persistence);
  const privacy = computed(() => refs.settings.value.device.privacy);
  const notifications = computed(() => refs.settings.value.device.notifications);

  const theme = computed({ get: () => appearance.value.theme, set: (value) => { appearance.value.theme = value; } });
  const fontScale = computed({ get: () => appearance.value.fontScale, set: (value) => { appearance.value.fontScale = value; } });
  const uiScale = computed({ get: () => appearance.value.uiScale, set: (value) => { appearance.value.uiScale = value; } });
  const shortcutProfile = computed({ get: () => input.value.shortcutProfile, set: (value) => { input.value.shortcutProfile = value; } });
  const trackpadOptimized = computed({ get: () => input.value.trackpadOptimized, set: (value) => { input.value.trackpadOptimized = value; } });
  const autoSaveDelayMs = computed({ get: () => persistence.value.autoSaveDelayMs, set: (value) => { persistence.value.autoSaveDelayMs = value; } });
  const cacheCleanupPolicy = computed({ get: () => persistence.value.cacheCleanupPolicy, set: (value) => { persistence.value.cacheCleanupPolicy = value; } });
  const lastCacheCleanupAt = computed(() => persistence.value.lastCacheCleanupAt);

  return {
    store,
    settings: refs.settings,
    capabilities: refs.capabilities,
    hydrated: refs.hydrated,
    settingsSaving: refs.saving,
    settingsSaveError: refs.saveError,
    theme,
    fontScale,
    uiScale,
    general,
    appearance,
    accessibility,
    input,
    motion,
    pet,
    persistence,
    privacy,
    notifications,
    autoSaveDelayMs,
    cacheCleanupPolicy,
    cacheCleaning: refs.cacheCleaning,
    lastCacheCleanupAt,
    shortcutProfile,
    trackpadOptimized,
    usesMacShortcuts: refs.usesMacShortcuts,
    effectiveMotionMode: refs.effectiveMotionMode,
    smoothNavigationEnabled: refs.smoothNavigationEnabled,
    systemReducedMotion: refs.systemReducedMotion,
    motionSummary: refs.motionSummary,
    shortcutLabels: refs.shortcutLabels,
    shortcutRows: refs.shortcutRows,
    initializeSettings: store.initializeSettings,
    disposeSettings: store.disposeRuntimeListeners,
    flushSettings: store.persistNow,
    applyGeneralSettings: store.applyGeneralSettings,
    applyUiSettings: store.applyUiSettings,
    applyAccessibilitySettings: store.applyAccessibilitySettings,
    applyInteractionSettings: store.applyInteractionSettings,
    applyMotionSettings: store.applyMotionSettings,
    applyPersistenceSettings: store.applyPersistenceSettings,
    rememberSettingsSection: store.rememberSettingsSection,
    resetAllSettings: store.resetAllSettings,
    clearProjectCache: store.clearProjectCache,
    maybeCleanupCache: store.maybeCleanupCache,
  };
}
