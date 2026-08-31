import { t, formatNumber, type LocalizedMessage } from '../i18n';
import { formatError } from '../i18n/kernel-messages';
import { computed, ref } from "vue";

export type AppTheme = "light" | "eye";
export type CacheCleanupPolicy = "startup" | "weekly" | "monthly" | "never";
export type ShortcutProfile = "auto" | "macos" | "windows";

type UseAppSettingsOptions = {
  detectedMacOS: boolean;
  clearCache: () => Promise<number>;
  onStatus: (message: LocalizedMessage) => void;
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
    [t('shortcutNewOpen'), `${shortcutLabels.value.newProject} / ${shortcutLabels.value.openProject}`],
    [t('saveProject'), shortcutLabels.value.save],
    [t('shortcutFind'), shortcutLabels.value.find],
    [t('shortcutNextPrevious'), `${shortcutLabels.value.nextFind} / ${shortcutLabels.value.previousFind}`],
    [t('shortcutUndoRedo'), `${shortcutLabels.value.undo} / ${shortcutLabels.value.redo}`],
    [t('shortcutSaveExit'), `${shortcutLabels.value.commitEdit} · Esc`],
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
      options.onStatus(() => t('shortcutChanged', { p0: usesMacShortcuts.value ? "macOS" : "Windows / Linux" }));
    }
  }

  function applyPersistenceSettings() {
    localStorage.setItem("jueming-autosave-delay-ms", String(autoSaveDelayMs.value));
    localStorage.setItem("jueming-cache-cleanup-policy", cacheCleanupPolicy.value);
    options.onStatus(() => t('persistenceUpdated'));
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
        options.onStatus(() => t('cacheCleared', { p0: formatNumber(removedBytes / 1024, { minimumFractionDigits: 1, maximumFractionDigits: 1 }) }));
      }
    } catch (error) {
      if (manual) options.onStatus(() => t('cacheFailed', { p0: formatError(error) }));
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
