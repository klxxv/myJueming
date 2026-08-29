import { computed, ref } from "vue";

export type AppTheme = "light" | "eye";
export type CacheCleanupPolicy = "startup" | "weekly" | "monthly" | "never";
export type ShortcutProfile = "auto" | "macos" | "windows";

type UseAppSettingsOptions = {
  detectedMacOS: boolean;
  clearCache: () => Promise<number>;
  onStatus: (message: string) => void;
};

const AUTOSAVE_DELAYS = [1000, 3000, 5000, 10000, 30000];
const CACHE_POLICIES: CacheCleanupPolicy[] = ["startup", "weekly", "monthly", "never"];
const SHORTCUT_PROFILES: ShortcutProfile[] = ["auto", "macos", "windows"];

export function useAppSettings(options: UseAppSettingsOptions) {
  const theme = ref<AppTheme>("light");
  const fontScale = ref(100);
  const uiScale = ref(100);
  const autoSaveDelayMs = ref(3000);
  const cacheCleanupPolicy = ref<CacheCleanupPolicy>("weekly");
  const cacheCleaning = ref(false);
  const lastCacheCleanupAt = ref<number | null>(null);
  const shortcutProfile = ref<ShortcutProfile>("auto");
  const trackpadOptimized = ref(options.detectedMacOS);

  const usesMacShortcuts = computed(() => shortcutProfile.value === "macos"
    || (shortcutProfile.value === "auto" && options.detectedMacOS));
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

  function applyUiSettings() {
    document.documentElement.dataset.theme = theme.value;
    document.documentElement.style.setProperty("--reading-font-scale", String(fontScale.value / 100));
    const appRoot = document.querySelector<HTMLElement>("#app");
    if (appRoot) {
      appRoot.style.zoom = String(uiScale.value / 100);
      appRoot.style.width = `${10000 / uiScale.value}%`;
      appRoot.style.height = `${10000 / uiScale.value}%`;
    }
    localStorage.setItem("jueming-theme", theme.value);
    localStorage.setItem("jueming-font-scale", String(fontScale.value));
    localStorage.setItem("jueming-ui-scale", String(uiScale.value));
  }

  function applyInteractionSettings(notifyUser = true) {
    document.documentElement.dataset.platform = options.detectedMacOS ? "macos" : "windows";
    document.documentElement.dataset.trackpad = trackpadOptimized.value ? "optimized" : "standard";
    localStorage.setItem("jueming-shortcut-profile", shortcutProfile.value);
    localStorage.setItem("jueming-trackpad-optimized", String(trackpadOptimized.value));
    if (notifyUser) {
      options.onStatus(`快捷键已切换为${usesMacShortcuts.value ? "macOS" : "Windows / Linux"}布局`);
    }
  }

  function applyPersistenceSettings() {
    localStorage.setItem("jueming-autosave-delay-ms", String(autoSaveDelayMs.value));
    localStorage.setItem("jueming-cache-cleanup-policy", cacheCleanupPolicy.value);
    options.onStatus("自动保存与缓存策略已更新");
  }

  function initializeSettings() {
    const savedTheme = localStorage.getItem("jueming-theme");
    const savedScale = Number(localStorage.getItem("jueming-font-scale"));
    const savedUiScale = Number(localStorage.getItem("jueming-ui-scale"));
    const savedAutosaveDelay = Number(localStorage.getItem("jueming-autosave-delay-ms"));
    const savedCleanupPolicy = localStorage.getItem("jueming-cache-cleanup-policy") as CacheCleanupPolicy | null;
    const savedCleanupAt = Number(localStorage.getItem("jueming-cache-cleanup-at"));
    const savedShortcutProfile = localStorage.getItem("jueming-shortcut-profile") as ShortcutProfile | null;
    const savedTrackpadOptimized = localStorage.getItem("jueming-trackpad-optimized");

    if (savedTheme === "eye") theme.value = savedTheme;
    if (savedScale >= 85 && savedScale <= 130) fontScale.value = savedScale;
    if (savedUiScale >= 85 && savedUiScale <= 115) uiScale.value = savedUiScale;
    if (AUTOSAVE_DELAYS.includes(savedAutosaveDelay)) autoSaveDelayMs.value = savedAutosaveDelay;
    if (savedCleanupPolicy && CACHE_POLICIES.includes(savedCleanupPolicy)) cacheCleanupPolicy.value = savedCleanupPolicy;
    if (Number.isFinite(savedCleanupAt) && savedCleanupAt > 0) lastCacheCleanupAt.value = savedCleanupAt;
    if (savedShortcutProfile && SHORTCUT_PROFILES.includes(savedShortcutProfile)) shortcutProfile.value = savedShortcutProfile;
    if (savedTrackpadOptimized === "true" || savedTrackpadOptimized === "false") {
      trackpadOptimized.value = savedTrackpadOptimized === "true";
    }

    applyUiSettings();
    applyInteractionSettings(false);
  }

  async function clearProjectCache(manual = true) {
    cacheCleaning.value = true;
    try {
      const removedBytes = await options.clearCache();
      lastCacheCleanupAt.value = Date.now();
      localStorage.setItem("jueming-cache-cleanup-at", String(lastCacheCleanupAt.value));
      if (manual) {
        options.onStatus(`已清理 ${(removedBytes / 1024).toFixed(1)} KB 可重建缓存；Revision 历史未删除`);
      }
    } catch (error) {
      if (manual) options.onStatus(`缓存清理失败：${error instanceof Error ? error.message : String(error)}`);
    } finally {
      cacheCleaning.value = false;
    }
  }

  async function maybeCleanupCache() {
    if (cacheCleanupPolicy.value === "never") return;
    const elapsed = Date.now() - (lastCacheCleanupAt.value ?? 0);
    const due = cacheCleanupPolicy.value === "startup"
      || (cacheCleanupPolicy.value === "weekly" && elapsed >= 7 * 86400000)
      || (cacheCleanupPolicy.value === "monthly" && elapsed >= 30 * 86400000);
    if (due) await clearProjectCache(false);
  }

  return {
    theme,
    fontScale,
    uiScale,
    autoSaveDelayMs,
    cacheCleanupPolicy,
    cacheCleaning,
    lastCacheCleanupAt,
    shortcutProfile,
    trackpadOptimized,
    usesMacShortcuts,
    shortcutLabels,
    shortcutRows,
    initializeSettings,
    applyUiSettings,
    applyInteractionSettings,
    applyPersistenceSettings,
    clearProjectCache,
    maybeCleanupCache,
  };
}
