export const APP_SETTINGS_SCHEMA_VERSION = 1 as const;

export type AppTheme = "light" | "eye";
export type CacheCleanupPolicy = "startup" | "weekly" | "monthly" | "never";
export type ShortcutProfile = "auto" | "macos" | "windows";
export type InterfaceLanguage = "system" | "zh-CN" | "en";
export type StartupDestination = "welcome" | "last-project" | "project-picker";
export type DefaultWorkspace = "review" | "edit" | "order";
export type ReadingLineSpacing = "compact" | "standard" | "relaxed";
export type ListDensity = "compact" | "standard" | "comfortable";
export type SidebarSize = "auto" | "compact" | "wide";
export type ColumnBalance = "equal" | "source-wide" | "target-wide";
export type MotionMode = "system" | "standard" | "reduced" | "off";
export type ContrastMode = "system" | "standard" | "increased";
export type ScreenReaderVerbosity = "concise" | "standard" | "detailed";
export type PetCharacter = "orange-cat" | "golden-retriever" | "yellow-dog";
export type PetActivity = "quiet" | "standard" | "lively";
export type PetLocation = "bottom-garden" | "side-rest";

export interface AppSettingsEnvelope {
  schemaVersion: typeof APP_SETTINGS_SCHEMA_VERSION;
  device: {
    general: {
      interfaceLanguage: InterfaceLanguage;
      startupDestination: StartupDestination;
      defaultWorkspace: DefaultWorkspace;
      rememberSettingsLocation: boolean;
      restoreWindowBounds: boolean;
    };
    appearance: {
      theme: AppTheme;
      fontScale: number;
      uiScale: number;
      lineSpacing: ReadingLineSpacing;
      listDensity: ListDensity;
      sidebarSize: SidebarSize;
      columnBalance: ColumnBalance;
      alignmentHints: boolean;
    };
    accessibility: {
      contrast: ContrastMode;
      enhancedFocus: boolean;
      largerTargets: boolean;
      reduceTransparency: boolean;
      screenReaderVerbosity: ScreenReaderVerbosity;
    };
    input: {
      shortcutProfile: ShortcutProfile;
      trackpadOptimized: boolean;
    };
    motion: {
      mode: MotionMode;
      interfaceTransitions: boolean;
      smoothNavigation: boolean;
      highlightFade: boolean;
      dragFeedback: boolean;
      successMotion: boolean;
      pauseWhenUnfocused: boolean;
    };
    pet: {
      enabled: boolean;
      character: PetCharacter;
      activity: PetActivity;
      location: PetLocation;
      plantMotion: boolean;
      butterflyMotion: boolean;
      chaseMotion: boolean;
      quietWhileEditing: boolean;
      hideForSearchAndAnnotations: boolean;
    };
    persistence: {
      autoSaveDelayMs: number;
      cacheCleanupPolicy: CacheCleanupPolicy;
      lastCacheCleanupAt: number | null;
    };
    privacy: {
      localOnlyMode: boolean;
      crashReports: boolean;
      anonymousUsage: boolean;
    };
    notifications: {
      systemNotifications: boolean;
      cacheResults: boolean;
      petStatus: boolean;
    };
    navigation: {
      lastSection: string;
    };
  };
}

export const createDefaultAppSettings = (detectedMacOS: boolean): AppSettingsEnvelope => ({
  schemaVersion: APP_SETTINGS_SCHEMA_VERSION,
  device: {
    general: {
      interfaceLanguage: "system",
      startupDestination: "welcome",
      defaultWorkspace: "review",
      rememberSettingsLocation: true,
      restoreWindowBounds: true,
    },
    appearance: {
      theme: "light",
      fontScale: 100,
      uiScale: 100,
      lineSpacing: "standard",
      listDensity: "standard",
      sidebarSize: "auto",
      columnBalance: "equal",
      alignmentHints: true,
    },
    accessibility: {
      contrast: "system",
      enhancedFocus: true,
      largerTargets: false,
      reduceTransparency: false,
      screenReaderVerbosity: "standard",
    },
    input: {
      shortcutProfile: "auto",
      trackpadOptimized: detectedMacOS,
    },
    motion: {
      mode: "system",
      interfaceTransitions: true,
      smoothNavigation: true,
      highlightFade: true,
      dragFeedback: true,
      successMotion: true,
      pauseWhenUnfocused: true,
    },
    pet: {
      enabled: false,
      character: "orange-cat",
      activity: "standard",
      location: "bottom-garden",
      plantMotion: true,
      butterflyMotion: true,
      chaseMotion: true,
      quietWhileEditing: true,
      hideForSearchAndAnnotations: true,
    },
    persistence: {
      autoSaveDelayMs: 3000,
      cacheCleanupPolicy: "weekly",
      lastCacheCleanupAt: null,
    },
    privacy: {
      localOnlyMode: true,
      crashReports: false,
      anonymousUsage: false,
    },
    notifications: {
      systemNotifications: false,
      cacheResults: true,
      petStatus: false,
    },
    navigation: {
      lastSection: "appearance",
    },
  },
});

type UnknownRecord = Record<string, unknown>;

const asRecord = (value: unknown): UnknownRecord => value !== null && typeof value === "object" && !Array.isArray(value)
  ? value as UnknownRecord
  : {};
const booleanValue = (value: unknown, fallback: boolean) => typeof value === "boolean" ? value : fallback;
const numberValue = (value: unknown, fallback: number, min: number, max: number) =>
  typeof value === "number" && Number.isFinite(value) ? Math.min(max, Math.max(min, value)) : fallback;
const numberEnumValue = (value: unknown, allowed: readonly number[], fallback: number) =>
  typeof value === "number" && allowed.includes(value) ? value : fallback;
const nullableTimestamp = (value: unknown, fallback: number | null) =>
  value === null ? null : typeof value === "number" && Number.isFinite(value) && value > 0 ? value : fallback;
const enumValue = <T extends string>(value: unknown, allowed: readonly T[], fallback: T): T =>
  typeof value === "string" && allowed.includes(value as T) ? value as T : fallback;

export function parseAppSettings(value: unknown, detectedMacOS: boolean): AppSettingsEnvelope {
  const defaults = createDefaultAppSettings(detectedMacOS);
  const root = asRecord(value);
  const device = asRecord(root.device);
  const general = asRecord(device.general);
  const appearance = asRecord(device.appearance);
  const accessibility = asRecord(device.accessibility);
  const input = asRecord(device.input);
  const motion = asRecord(device.motion);
  const pet = asRecord(device.pet);
  const persistence = asRecord(device.persistence);
  const privacy = asRecord(device.privacy);
  const notifications = asRecord(device.notifications);
  const navigation = asRecord(device.navigation);

  return {
    schemaVersion: APP_SETTINGS_SCHEMA_VERSION,
    device: {
      general: {
        interfaceLanguage: enumValue(general.interfaceLanguage, ["system", "zh-CN", "en"], defaults.device.general.interfaceLanguage),
        startupDestination: enumValue(general.startupDestination, ["welcome", "last-project", "project-picker"], defaults.device.general.startupDestination),
        defaultWorkspace: enumValue(general.defaultWorkspace, ["review", "edit", "order"], defaults.device.general.defaultWorkspace),
        rememberSettingsLocation: booleanValue(general.rememberSettingsLocation, defaults.device.general.rememberSettingsLocation),
        restoreWindowBounds: booleanValue(general.restoreWindowBounds, defaults.device.general.restoreWindowBounds),
      },
      appearance: {
        theme: enumValue(appearance.theme, ["light", "eye"], defaults.device.appearance.theme),
        fontScale: numberValue(appearance.fontScale, defaults.device.appearance.fontScale, 90, 130),
        uiScale: numberValue(appearance.uiScale, defaults.device.appearance.uiScale, 85, 115),
        lineSpacing: enumValue(appearance.lineSpacing, ["compact", "standard", "relaxed"], defaults.device.appearance.lineSpacing),
        listDensity: enumValue(appearance.listDensity, ["compact", "standard", "comfortable"], defaults.device.appearance.listDensity),
        sidebarSize: enumValue(appearance.sidebarSize, ["auto", "compact", "wide"], defaults.device.appearance.sidebarSize),
        columnBalance: enumValue(appearance.columnBalance, ["equal", "source-wide", "target-wide"], defaults.device.appearance.columnBalance),
        alignmentHints: booleanValue(appearance.alignmentHints, defaults.device.appearance.alignmentHints),
      },
      accessibility: {
        contrast: enumValue(accessibility.contrast, ["system", "standard", "increased"], defaults.device.accessibility.contrast),
        enhancedFocus: booleanValue(accessibility.enhancedFocus, defaults.device.accessibility.enhancedFocus),
        largerTargets: booleanValue(accessibility.largerTargets, defaults.device.accessibility.largerTargets),
        reduceTransparency: booleanValue(accessibility.reduceTransparency, defaults.device.accessibility.reduceTransparency),
        screenReaderVerbosity: enumValue(accessibility.screenReaderVerbosity, ["concise", "standard", "detailed"], defaults.device.accessibility.screenReaderVerbosity),
      },
      input: {
        shortcutProfile: enumValue(input.shortcutProfile, ["auto", "macos", "windows"], defaults.device.input.shortcutProfile),
        trackpadOptimized: booleanValue(input.trackpadOptimized, defaults.device.input.trackpadOptimized),
      },
      motion: {
        mode: enumValue(motion.mode, ["system", "standard", "reduced", "off"], defaults.device.motion.mode),
        interfaceTransitions: booleanValue(motion.interfaceTransitions, defaults.device.motion.interfaceTransitions),
        smoothNavigation: booleanValue(motion.smoothNavigation, defaults.device.motion.smoothNavigation),
        highlightFade: booleanValue(motion.highlightFade, defaults.device.motion.highlightFade),
        dragFeedback: booleanValue(motion.dragFeedback, defaults.device.motion.dragFeedback),
        successMotion: booleanValue(motion.successMotion, defaults.device.motion.successMotion),
        pauseWhenUnfocused: booleanValue(motion.pauseWhenUnfocused, defaults.device.motion.pauseWhenUnfocused),
      },
      pet: {
        enabled: booleanValue(pet.enabled, defaults.device.pet.enabled),
        character: enumValue(pet.character, ["orange-cat", "golden-retriever", "yellow-dog"], defaults.device.pet.character),
        activity: enumValue(pet.activity, ["quiet", "standard", "lively"], defaults.device.pet.activity),
        location: enumValue(pet.location, ["bottom-garden", "side-rest"], defaults.device.pet.location),
        plantMotion: booleanValue(pet.plantMotion, defaults.device.pet.plantMotion),
        butterflyMotion: booleanValue(pet.butterflyMotion, defaults.device.pet.butterflyMotion),
        chaseMotion: booleanValue(pet.chaseMotion, defaults.device.pet.chaseMotion),
        quietWhileEditing: booleanValue(pet.quietWhileEditing, defaults.device.pet.quietWhileEditing),
        hideForSearchAndAnnotations: booleanValue(pet.hideForSearchAndAnnotations, defaults.device.pet.hideForSearchAndAnnotations),
      },
      persistence: {
        autoSaveDelayMs: numberEnumValue(persistence.autoSaveDelayMs, [1000, 3000, 5000, 10000, 30000], defaults.device.persistence.autoSaveDelayMs),
        cacheCleanupPolicy: enumValue(persistence.cacheCleanupPolicy, ["startup", "weekly", "monthly", "never"], defaults.device.persistence.cacheCleanupPolicy),
        lastCacheCleanupAt: nullableTimestamp(persistence.lastCacheCleanupAt, defaults.device.persistence.lastCacheCleanupAt),
      },
      privacy: {
        localOnlyMode: booleanValue(privacy.localOnlyMode, defaults.device.privacy.localOnlyMode),
        crashReports: booleanValue(privacy.crashReports, defaults.device.privacy.crashReports),
        anonymousUsage: booleanValue(privacy.anonymousUsage, defaults.device.privacy.anonymousUsage),
      },
      notifications: {
        systemNotifications: booleanValue(notifications.systemNotifications, defaults.device.notifications.systemNotifications),
        cacheResults: booleanValue(notifications.cacheResults, defaults.device.notifications.cacheResults),
        petStatus: booleanValue(notifications.petStatus, defaults.device.notifications.petStatus),
      },
      navigation: {
        lastSection: typeof navigation.lastSection === "string" ? navigation.lastSection : defaults.device.navigation.lastSection,
      },
    },
  };
}

const LEGACY_KEYS = [
  "jueming-theme",
  "jueming-font-scale",
  "jueming-ui-scale",
  "jueming-autosave-delay-ms",
  "jueming-cache-cleanup-policy",
  "jueming-cache-cleanup-at",
  "jueming-shortcut-profile",
  "jueming-trackpad-optimized",
] as const;

export function migrateLegacySettings(storage: Storage, detectedMacOS: boolean): AppSettingsEnvelope | null {
  if (!LEGACY_KEYS.some((key) => storage.getItem(key) !== null)) return null;
  const migrated = createDefaultAppSettings(detectedMacOS);
  const theme = storage.getItem("jueming-theme");
  const fontScale = Number(storage.getItem("jueming-font-scale"));
  const uiScale = Number(storage.getItem("jueming-ui-scale"));
  const autoSaveDelayMs = Number(storage.getItem("jueming-autosave-delay-ms"));
  const cacheCleanupPolicy = storage.getItem("jueming-cache-cleanup-policy");
  const lastCacheCleanupAt = Number(storage.getItem("jueming-cache-cleanup-at"));
  const shortcutProfile = storage.getItem("jueming-shortcut-profile");
  const trackpadOptimized = storage.getItem("jueming-trackpad-optimized");

  if (theme === "light" || theme === "eye") migrated.device.appearance.theme = theme;
  if (Number.isFinite(fontScale) && fontScale >= 90 && fontScale <= 130) migrated.device.appearance.fontScale = fontScale;
  if (Number.isFinite(uiScale) && uiScale >= 85 && uiScale <= 115) migrated.device.appearance.uiScale = uiScale;
  if ([1000, 3000, 5000, 10000, 30000].includes(autoSaveDelayMs)) migrated.device.persistence.autoSaveDelayMs = autoSaveDelayMs;
  if (["startup", "weekly", "monthly", "never"].includes(cacheCleanupPolicy ?? "")) migrated.device.persistence.cacheCleanupPolicy = cacheCleanupPolicy as CacheCleanupPolicy;
  if (Number.isFinite(lastCacheCleanupAt) && lastCacheCleanupAt > 0) migrated.device.persistence.lastCacheCleanupAt = lastCacheCleanupAt;
  if (["auto", "macos", "windows"].includes(shortcutProfile ?? "")) migrated.device.input.shortcutProfile = shortcutProfile as ShortcutProfile;
  if (trackpadOptimized === "true" || trackpadOptimized === "false") migrated.device.input.trackpadOptimized = trackpadOptimized === "true";
  return migrated;
}

export function clearLegacySettings(storage: Storage) {
  for (const key of LEGACY_KEYS) storage.removeItem(key);
}
