import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { KernelClient } from "../domain/kernel-client";
import {
  clearLegacySettings,
  createDefaultAppSettings,
  migrateLegacySettings,
  parseAppSettings,
  type AppSettingsEnvelope,
} from "../settings/schema";
import { createSettingsCapabilities } from "../settings/capabilities";
import { setUiLocale, t, formatNumber, type LocalizedMessage } from "../i18n";

type AppSettingsStoreOptions = {
  detectedMacOS: boolean;
  kernelClient: Pick<KernelClient, "loadAppSettings" | "saveAppSettings" | "resetAppSettings" | "clearCache"> & Partial<Pick<KernelClient, "observeSystemTheme">>;
  onStatus: (message: LocalizedMessage) => void;
};

const SAVE_DEBOUNCE_MS = 320;

export const useAppSettingsStore = defineStore("app-settings", () => {
  const settings = ref<AppSettingsEnvelope>(createDefaultAppSettings(false));
  const capabilities = ref(createSettingsCapabilities());
  const hydrated = ref(false);
  const saving = ref(false);
  const saveError = ref<string | null>(null);
  const cacheCleaning = ref(false);
  const detectedMacOS = ref(false);
  const systemReducedMotion = ref(false);
  const windowFocused = ref(true);

  let client: AppSettingsStoreOptions["kernelClient"] | null = null;
  let onStatus: AppSettingsStoreOptions["onStatus"] = () => undefined;
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let reducedMotionQuery: MediaQueryList | null = null;
  let colorSchemeQuery: MediaQueryList | null = null;
  let listenersAttached = false;
  let nativeTheme: "light" | "dark" | null = null;
  let unlistenNativeTheme: (() => void) | null = null;
  let themeListenerGeneration = 0;

  const usesMacShortcuts = computed(() => settings.value.device.input.shortcutProfile === "macos"
    || (settings.value.device.input.shortcutProfile === "auto" && detectedMacOS.value));

  const effectiveMotionMode = computed<"standard" | "reduced" | "off">(() => {
    const configured = settings.value.device.motion.mode;
    if (configured === "off") return "off";
    if (!windowFocused.value && settings.value.device.motion.pauseWhenUnfocused) return "off";
    if (systemReducedMotion.value) return "reduced";
    if (configured === "reduced") return "reduced";
    return "standard";
  });

  const smoothNavigationEnabled = computed(() => effectiveMotionMode.value === "standard"
    && settings.value.device.motion.smoothNavigation);

  const shortcutLabels = computed(() => usesMacShortcuts.value ? {
    newProject: "⌘N",
    openProject: "⌘O",
    save: "⌘S",
    find: "⌘F",
    nextFind: "⌘G",
    previousFind: "⇧⌘G",
    undo: "⌘Z",
    redo: "⇧⌘Z",
    commitEdit: "⌘↵",
  } : {
    newProject: "Ctrl+N",
    openProject: "Ctrl+O",
    save: "Ctrl+S",
    find: "Ctrl+F",
    nextFind: "Ctrl+G",
    previousFind: "Ctrl+Shift+G",
    undo: "Ctrl+Z",
    redo: "Ctrl+Y",
    commitEdit: "Ctrl+Enter",
  });

  const shortcutRows = computed<Array<[string, string]>>(() => [
    [t("shortcutNewOpen"), `${shortcutLabels.value.newProject} / ${shortcutLabels.value.openProject}`],
    [t("saveProject"), shortcutLabels.value.save],
    [t("shortcutFind"), shortcutLabels.value.find],
    [t("shortcutNextPrevious"), `${shortcutLabels.value.nextFind} / ${shortcutLabels.value.previousFind}`],
    [t("shortcutUndoRedo"), `${shortcutLabels.value.undo} / ${shortcutLabels.value.redo}`],
    [t("shortcutSaveExit"), `${shortcutLabels.value.commitEdit} · Esc`],
  ]);

  function currentMotionSummaryKey() {
    const mode = settings.value.device.motion.mode;
    if (mode === "system") return systemReducedMotion.value ? "runtimeMotionSystemReduced" : "runtimeMotionSystem";
    if (mode === "standard") return "standard";
    return mode === "reduced" ? "runtimeMotionReduced" : "runtimeMotionOff";
  }

  const motionSummary = computed(() => t(currentMotionSummaryKey()));

  function configure(options: AppSettingsStoreOptions) {
    client = options.kernelClient;
    onStatus = options.onStatus;
    detectedMacOS.value = options.detectedMacOS;
    if (!hydrated.value) settings.value = createDefaultAppSettings(options.detectedMacOS);
  }

  function applyThemeToDocument() {
    const preference = settings.value.device.appearance.theme;
    const systemTheme = nativeTheme ?? ((colorSchemeQuery ?? window.matchMedia("(prefers-color-scheme: dark)")).matches ? "dark" : "light");
    document.documentElement.dataset.theme = preference === "system"
      ? (systemTheme === "dark" ? "dark" : settings.value.device.appearance.systemLightTheme)
      : preference;
  }

  function handleSystemThemeChange(event?: MediaQueryListEvent) {
    // A fresh media event must not be masked by a cached native snapshot.
    if (event) nativeTheme = event.matches ? "dark" : "light";
    if (settings.value.device.appearance.theme === "system") applyThemeToDocument();
  }

  function applySettingsToDocument() {
    const root = document.documentElement;
    const device = settings.value.device;
    setUiLocale(device.general.interfaceLanguage);
    applyThemeToDocument();
    root.dataset.lineSpacing = device.appearance.lineSpacing;
    root.dataset.listDensity = device.appearance.listDensity;
    root.dataset.sidebarSize = device.appearance.sidebarSize;
    root.dataset.columnBalance = device.appearance.columnBalance;
    root.dataset.alignmentHints = device.appearance.alignmentHints ? "on" : "off";
    root.dataset.contrast = device.accessibility.contrast;
    root.dataset.enhancedFocus = device.accessibility.enhancedFocus ? "on" : "off";
    root.dataset.largerTargets = device.accessibility.largerTargets ? "on" : "off";
    root.dataset.reduceTransparency = device.accessibility.reduceTransparency ? "on" : "off";
    root.dataset.screenReaderVerbosity = device.accessibility.screenReaderVerbosity;
    root.dataset.platform = detectedMacOS.value ? "macos" : "windows";
    root.dataset.trackpad = device.input.trackpadOptimized ? "optimized" : "standard";
    root.dataset.motion = effectiveMotionMode.value;
    root.dataset.interfaceTransitions = device.motion.interfaceTransitions ? "on" : "off";
    root.dataset.highlightFade = device.motion.highlightFade ? "on" : "off";
    root.dataset.dragFeedback = device.motion.dragFeedback ? "on" : "off";
    root.dataset.successMotion = device.motion.successMotion ? "on" : "off";
    root.style.setProperty("--reading-font-scale", String(device.appearance.fontScale / 100));
    root.style.setProperty("--jm-reading-line-height", device.appearance.lineSpacing === "compact"
      ? "1.45" : device.appearance.lineSpacing === "relaxed" ? "1.82" : "1.62");
    const columnFractions = device.appearance.columnBalance === "source-wide"
      ? ["1.18fr", ".82fr"]
      : device.appearance.columnBalance === "target-wide" ? [".82fr", "1.18fr"] : ["1fr", "1fr"];
    root.style.setProperty("--source-column-fr", columnFractions[0]);
    root.style.setProperty("--target-column-fr", columnFractions[1]);

    const appRoot = document.querySelector<HTMLElement>("#app");
    if (appRoot) {
      appRoot.style.zoom = String(device.appearance.uiScale / 100);
      // Chromium/WebView2 already adjusts the layout viewport for CSS zoom.
      appRoot.style.removeProperty("width");
      appRoot.style.removeProperty("height");
    }
  }

  function handleSystemMotionChange(event: MediaQueryListEvent | MediaQueryList) {
    systemReducedMotion.value = event.matches;
    applySettingsToDocument();
  }

  function handleFocus() {
    windowFocused.value = true;
    applySettingsToDocument();
  }

  function handleBlur() {
    windowFocused.value = false;
    applySettingsToDocument();
  }

  function attachRuntimeListeners() {
    if (listenersAttached) return;
    colorSchemeQuery = window.matchMedia("(prefers-color-scheme: dark)");
    colorSchemeQuery.addEventListener("change", handleSystemThemeChange);
    reducedMotionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    handleSystemMotionChange(reducedMotionQuery);
    reducedMotionQuery.addEventListener("change", handleSystemMotionChange);
    window.addEventListener("focus", handleFocus);
    window.addEventListener("blur", handleBlur);
    listenersAttached = true;
    const generation = ++themeListenerGeneration;
    void client?.observeSystemTheme?.((theme) => {
      if (!listenersAttached || generation !== themeListenerGeneration) return;
      nativeTheme = theme;
      handleSystemThemeChange();
    }).then((unlisten) => {
      if (!listenersAttached || generation !== themeListenerGeneration) unlisten();
      else unlistenNativeTheme = unlisten;
    }).catch(() => {
      // Native theme may be unavailable; retain the browser media-query fallback.
    });
  }

  function disposeRuntimeListeners() {
    if (!listenersAttached) return;
    themeListenerGeneration++;
    unlistenNativeTheme?.();
    unlistenNativeTheme = null;
    nativeTheme = null;
    colorSchemeQuery?.removeEventListener("change", handleSystemThemeChange);
    colorSchemeQuery = null;
    reducedMotionQuery?.removeEventListener("change", handleSystemMotionChange);
    window.removeEventListener("focus", handleFocus);
    window.removeEventListener("blur", handleBlur);
    reducedMotionQuery = null;
    listenersAttached = false;
  }

  async function persistNow() {
    if (!client) return;
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
    saving.value = true;
    saveError.value = null;
    try {
      await client.saveAppSettings(settings.value);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      saveError.value = message;
      onStatus(() => t("runtimeSettingsSaveFailed", { p0: message }));
    } finally {
      saving.value = false;
    }
  }

  function schedulePersist(status?: LocalizedMessage) {
    applySettingsToDocument();
    if (status) onStatus(status);
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => { void persistNow(); }, SAVE_DEBOUNCE_MS);
  }

  async function initializeSettings() {
    if (!client) throw new Error("App settings store was not configured with a KernelClient.");
    try {
      const stored = await client.loadAppSettings();
      if (stored) {
        settings.value = parseAppSettings(stored, detectedMacOS.value);
      } else {
        const legacy = migrateLegacySettings(localStorage, detectedMacOS.value);
        settings.value = legacy ?? createDefaultAppSettings(detectedMacOS.value);
        await client.saveAppSettings(settings.value);
        if (legacy) clearLegacySettings(localStorage);
      }
    } catch (error) {
      settings.value = createDefaultAppSettings(detectedMacOS.value);
      const message = error instanceof Error ? error.message : String(error);
      saveError.value = message;
      onStatus(() => t("runtimeSettingsLoadFailed", { p0: message }));
    }
    attachRuntimeListeners();
    hydrated.value = true;
    applySettingsToDocument();
  }

  function applyGeneralSettings() {
    schedulePersist(() => t("runtimeGeneralUpdated"));
  }

  function applyUiSettings() {
    schedulePersist();
  }

  function applyAccessibilitySettings() {
    schedulePersist(() => t("runtimeAccessibilityUpdated"));
  }

  function applyInteractionSettings(notifyUser = true) {
    const platform = usesMacShortcuts.value ? "macOS" : "Windows / Linux";
    schedulePersist(notifyUser
      ? () => t("shortcutChanged", { p0: platform })
      : undefined);
  }

  function applyMotionSettings() {
    const summaryKey = currentMotionSummaryKey();
    schedulePersist(() => t("runtimeMotionUpdated", { p0: t(summaryKey) }));
  }

  function applyPersistenceSettings() {
    schedulePersist(() => t("persistenceUpdated"));
  }

  function rememberSettingsSection(section: string) {
    settings.value.device.navigation.lastSection = section;
    schedulePersist();
  }

  async function resetAllSettings() {
    if (!client) return;
    await client.resetAppSettings();
    settings.value = createDefaultAppSettings(detectedMacOS.value);
    applySettingsToDocument();
    await persistNow();
    onStatus(() => t("runtimeSettingsReset"));
  }

  async function clearProjectCache(manual = true) {
    if (!client) return;
    cacheCleaning.value = true;
    try {
      const removedBytes = await client.clearCache();
      settings.value.device.persistence.lastCacheCleanupAt = Date.now();
      await persistNow();
      const removedKilobytes = removedBytes / 1024;
      if (manual) onStatus(() => t("cacheCleared", { p0: formatNumber(removedKilobytes, { minimumFractionDigits: 1, maximumFractionDigits: 1 }) }));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (manual) onStatus(() => t("cacheFailed", { p0: message }));
    } finally {
      cacheCleaning.value = false;
    }
  }

  async function maybeCleanupCache() {
    const policy = settings.value.device.persistence.cacheCleanupPolicy;
    if (policy === "never") return;
    const elapsed = Date.now() - (settings.value.device.persistence.lastCacheCleanupAt ?? 0);
    const due = policy === "startup"
      || (policy === "weekly" && elapsed >= 7 * 86400000)
      || (policy === "monthly" && elapsed >= 30 * 86400000);
    if (due) await clearProjectCache(false);
  }

  return {
    settings,
    capabilities,
    hydrated,
    saving,
    saveError,
    cacheCleaning,
    systemReducedMotion,
    windowFocused,
    usesMacShortcuts,
    effectiveMotionMode,
    smoothNavigationEnabled,
    motionSummary,
    shortcutLabels,
    shortcutRows,
    configure,
    initializeSettings,
    disposeRuntimeListeners,
    persistNow,
    applyGeneralSettings,
    applyUiSettings,
    applyAccessibilitySettings,
    applyInteractionSettings,
    applyMotionSettings,
    applyPersistenceSettings,
    rememberSettingsSection,
    resetAllSettings,
    clearProjectCache,
    maybeCleanupCache,
  };
});
