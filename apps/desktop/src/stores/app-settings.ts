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

type AppSettingsStoreOptions = {
  detectedMacOS: boolean;
  kernelClient: Pick<KernelClient, "loadAppSettings" | "saveAppSettings" | "resetAppSettings" | "clearCache">;
  onStatus: (message: string) => void;
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
  let listenersAttached = false;

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
    ["新建 / 打开工程", `${shortcutLabels.value.newProject} / ${shortcutLabels.value.openProject}`],
    ["保存工程", shortcutLabels.value.save],
    ["审阅内查找", shortcutLabels.value.find],
    ["下一处 / 上一处", `${shortcutLabels.value.nextFind} / ${shortcutLabels.value.previousFind}`],
    ["撤销 / 重做", `${shortcutLabels.value.undo} / ${shortcutLabels.value.redo}`],
    ["保存句段并退出", `${shortcutLabels.value.commitEdit} · Esc`],
  ]);

  const motionSummary = computed(() => {
    if (settings.value.device.motion.mode === "system") {
      return systemReducedMotion.value ? "跟随系统 · 已减少" : "跟随系统";
    }
    return settings.value.device.motion.mode === "standard"
      ? "标准"
      : settings.value.device.motion.mode === "reduced" ? "减少" : "关闭非必要动画";
  });

  function configure(options: AppSettingsStoreOptions) {
    client = options.kernelClient;
    onStatus = options.onStatus;
    detectedMacOS.value = options.detectedMacOS;
    if (!hydrated.value) settings.value = createDefaultAppSettings(options.detectedMacOS);
  }

  function applySettingsToDocument() {
    const root = document.documentElement;
    const device = settings.value.device;
    root.dataset.theme = device.appearance.theme;
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
    reducedMotionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    handleSystemMotionChange(reducedMotionQuery);
    reducedMotionQuery.addEventListener("change", handleSystemMotionChange);
    window.addEventListener("focus", handleFocus);
    window.addEventListener("blur", handleBlur);
    listenersAttached = true;
  }

  function disposeRuntimeListeners() {
    if (!listenersAttached) return;
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
      saveError.value = error instanceof Error ? error.message : String(error);
      onStatus(`设置保存失败：${saveError.value}`);
    } finally {
      saving.value = false;
    }
  }

  function schedulePersist(status?: string) {
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
      saveError.value = error instanceof Error ? error.message : String(error);
      onStatus(`读取设置失败，已使用默认值：${saveError.value}`);
    }
    attachRuntimeListeners();
    hydrated.value = true;
    applySettingsToDocument();
  }

  function applyGeneralSettings() {
    schedulePersist("通用偏好已更新");
  }

  function applyUiSettings() {
    schedulePersist();
  }

  function applyAccessibilitySettings() {
    schedulePersist("辅助功能偏好已更新");
  }

  function applyInteractionSettings(notifyUser = true) {
    schedulePersist(notifyUser
      ? `快捷键已切换为${usesMacShortcuts.value ? "macOS" : "Windows / Linux"}布局`
      : undefined);
  }

  function applyMotionSettings() {
    schedulePersist(`动态效果：${motionSummary.value}`);
  }

  function applyPersistenceSettings() {
    schedulePersist("自动保存与缓存策略已更新");
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
    onStatus("已恢复本机默认设置");
  }

  async function clearProjectCache(manual = true) {
    if (!client) return;
    cacheCleaning.value = true;
    try {
      const removedBytes = await client.clearCache();
      settings.value.device.persistence.lastCacheCleanupAt = Date.now();
      await persistNow();
      if (manual) onStatus(`已清理 ${(removedBytes / 1024).toFixed(1)} KB 可重建缓存；Revision 历史未删除`);
    } catch (error) {
      if (manual) onStatus(`缓存清理失败：${error instanceof Error ? error.message : String(error)}`);
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
