import { computed, watch } from "vue";
import { createI18n } from "vue-i18n";
import de from "./locales/de.json";
import en from "./locales/en.json";
import fr from "./locales/fr.json";
import ja from "./locales/ja.json";
import zh from "./locales/zh.json";
import { settingsRuntimeMessages } from "./settings-runtime-messages";
import { panelMessages } from "./panel-messages";
import { embeddedSettingsMessages } from "./embedded-settings-messages";
import { settingsMessages } from "./settings-messages";
import { parallelStatusMessages } from "./parallel-status-messages";
import { operationMessages } from "./operation-messages";
import { newProjectMessages } from "./new-project-messages";
import { shellMessages } from "./shell-messages";
import { companionMessages } from "./companion-messages";
import { parallelUiMessages } from "./parallel-ui-messages";
import { reviewMessages } from "./review-messages";
import { workflowMessages } from "./workflow-messages";

export const messages = {
  zh: { ...zh, ...settingsRuntimeMessages.zh, ...settingsMessages.zh, ...panelMessages.zh, ...embeddedSettingsMessages.zh, ...parallelStatusMessages.zh, ...operationMessages.zh, ...newProjectMessages.zh, ...shellMessages.zh, ...companionMessages.zh, ...parallelUiMessages.zh, ...reviewMessages.zh, ...workflowMessages.zh },
  en: { ...en, ...settingsRuntimeMessages.en, ...settingsMessages.en, ...panelMessages.en, ...embeddedSettingsMessages.en, ...parallelStatusMessages.en, ...operationMessages.en, ...newProjectMessages.en, ...shellMessages.en, ...companionMessages.en, ...parallelUiMessages.en, ...reviewMessages.en, ...workflowMessages.en },
  ja: { ...ja, ...settingsRuntimeMessages.ja, ...settingsMessages.ja, ...panelMessages.ja, ...embeddedSettingsMessages.ja, ...parallelStatusMessages.ja, ...operationMessages.ja, ...newProjectMessages.ja, ...shellMessages.ja, ...companionMessages.ja, ...parallelUiMessages.ja, ...reviewMessages.ja, ...workflowMessages.ja },
  fr: { ...fr, ...settingsRuntimeMessages.fr, ...settingsMessages.fr, ...panelMessages.fr, ...embeddedSettingsMessages.fr, ...parallelStatusMessages.fr, ...operationMessages.fr, ...newProjectMessages.fr, ...shellMessages.fr, ...companionMessages.fr, ...parallelUiMessages.fr, ...reviewMessages.fr, ...workflowMessages.fr },
  de: { ...de, ...settingsRuntimeMessages.de, ...settingsMessages.de, ...panelMessages.de, ...embeddedSettingsMessages.de, ...parallelStatusMessages.de, ...operationMessages.de, ...newProjectMessages.de, ...shellMessages.de, ...companionMessages.de, ...parallelUiMessages.de, ...reviewMessages.de, ...workflowMessages.de },
};

export const UI_LOCALES = ["zh", "en", "ja", "fr", "de"] as const;
export type UiLocale = typeof UI_LOCALES[number];
export type MessageKey = keyof typeof messages.zh;
export type LocalizedMessage = string | (() => string);
export type InterfaceLanguage = UiLocale | "zh-CN" | "system";

export const LOCALE_STORAGE_KEY = "jueming-ui-locale";
export const localeNames: Record<UiLocale, string> = {
  zh: "中文（简体）",
  en: "English",
  ja: "日本語",
  fr: "Français",
  de: "Deutsch",
};
export const formatLocales: Record<UiLocale, string> = {
  zh: "zh-CN",
  en: "en-US",
  ja: "ja-JP",
  fr: "fr-FR",
  de: "de-DE",
};

function matchLocale(value: string | null | undefined): UiLocale | undefined {
  if (!value) return undefined;
  const normalized = value.toLowerCase().replace("_", "-");
  if (normalized === "zh-cn" || normalized === "zh-hans") return "zh";
  const base = normalized.split("-")[0];
  return UI_LOCALES.find((locale) => locale === base);
}

// UI language is a device preference, never a project LanguageId or a text transform.
export function resolveLocale(saved: string | null, preferred: readonly string[]): UiLocale {
  const savedLocale = matchLocale(saved);
  if (savedLocale) return savedLocale;
  for (const language of preferred) {
    const preferredLocale = matchLocale(language);
    if (preferredLocale) return preferredLocale;
  }
  return "zh";
}

function preferredDeviceLanguages(): readonly string[] {
  if (typeof navigator === "undefined") return [];
  return navigator.languages?.length ? navigator.languages : [navigator.language];
}

function initialLocale(): UiLocale {
  let saved: string | null = null;
  try {
    saved = localStorage.getItem(LOCALE_STORAGE_KEY);
  } catch {
    // Storage can be unavailable while browser privacy controls are active.
  }
  return resolveLocale(saved, preferredDeviceLanguages());
}

// Every catalogue is bundled with the desktop app; switching never requires network access.
export const i18n = createI18n<[Record<MessageKey, string>], UiLocale, false>({
  legacy: false,
  globalInjection: false,
  locale: initialLocale(),
  fallbackLocale: "zh",
  pluralRules: {
    fr: (choice, choicesLength) =>
      choicesLength === 2 ? (Math.abs(choice) > 1 ? 1 : 0) : Math.min(Math.abs(choice), choicesLength - 1),
  },
  messages,
});

export const uiLocale = computed<UiLocale>({
  get: () => i18n.global.locale.value,
  set: (value) => {
    if (UI_LOCALES.includes(value)) i18n.global.locale.value = value;
  },
});
export const formatLocale = computed(() => formatLocales[uiLocale.value]);

export function setUiLocale(language: InterfaceLanguage): UiLocale {
  const locale = language === "system"
    ? resolveLocale(null, preferredDeviceLanguages())
    : resolveLocale(language, []);
  uiLocale.value = locale;
  return locale;
}

export function formatNumber(value: number, options?: Intl.NumberFormatOptions): string {
  return new Intl.NumberFormat(formatLocale.value, options).format(value);
}

export function t(key: MessageKey, params: Record<string, unknown> = {}): string {
  const values = Object.fromEntries(
    Object.entries(params).map(([name, value]) => [
      name,
      typeof value === "number" ? formatNumber(value) : value,
    ]),
  );
  return i18n.global.t(key, values, {
    plural: typeof params.count === "number"
      ? params.count
      : typeof params.p0 === "number"
        ? params.p0
        : undefined,
  });
}

export function formatDate(value: string | number, timeOnly = false): string {
  const date = new Date(value);
  if (!Number.isFinite(date.getTime())) return "—";
  return new Intl.DateTimeFormat(
    formatLocale.value,
    timeOnly
      ? { hour: "2-digit", minute: "2-digit" }
      : {
          year: "numeric",
          month: "2-digit",
          day: "2-digit",
          hour: "2-digit",
          minute: "2-digit",
          second: "2-digit",
        },
  ).format(date);
}

export function initializeLocale(): () => void {
  return watch(
    uiLocale,
    (locale) => {
      if (typeof document !== "undefined") {
        document.documentElement.lang = formatLocales[locale];
        document.documentElement.dir = "ltr";
        document.title = t("appName");
      }
      try {
        localStorage.setItem(LOCALE_STORAGE_KEY, locale);
      } catch {
        // Keep in-memory switching usable when storage is unavailable.
      }
    },
    { immediate: true, flush: "sync" },
  );
}
