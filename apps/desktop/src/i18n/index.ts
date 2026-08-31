import { createI18n } from "vue-i18n";
import { computed, watch } from "vue";
import zh from "./locales/zh.json";
import en from "./locales/en.json";
import fr from "./locales/fr.json";

export const UI_LOCALES = ["zh", "en", "fr"] as const;
export type UiLocale = typeof UI_LOCALES[number];
export type MessageKey = keyof typeof zh;
export type LocalizedMessage = string | (() => string);
export const LOCALE_STORAGE_KEY = "jueming-ui-locale";
export const localeNames: Record<UiLocale, string> = { zh: "中文（简体）", en: "English", fr: "Français" };
export const formatLocales: Record<UiLocale, string> = { zh: "zh-CN", en: "en-US", fr: "fr-FR" };

// UI language is a device preference, never a project LanguageId or a text transform.
export function resolveLocale(saved: string | null, preferred: readonly string[]): UiLocale {
  if (UI_LOCALES.includes(saved as UiLocale)) return saved as UiLocale;
  for (const language of preferred) {
    const base = language.toLowerCase().split(/[-_]/)[0];
    if (UI_LOCALES.includes(base as UiLocale)) return base as UiLocale;
  }
  return "zh";
}

function initialLocale(): UiLocale {
  let saved: string | null = null;
  try { saved = localStorage.getItem(LOCALE_STORAGE_KEY); } catch { /* Storage can be disabled. */ }
  return resolveLocale(saved, typeof navigator === "undefined" ? [] : navigator.languages ?? [navigator.language]);
}

// All three catalogues are bundled and precompiled by Vite; switching needs no network.
export const i18n = createI18n<[typeof zh], UiLocale, false>({
  legacy: false,
  globalInjection: false,
  locale: initialLocale(),
  fallbackLocale: "zh",
  pluralRules: { fr: (choice, choicesLength) => choicesLength === 2 ? (Math.abs(choice) > 1 ? 1 : 0) : Math.min(Math.abs(choice), choicesLength - 1) },
  messages: { zh, en, fr },
});

export const uiLocale = computed<UiLocale>({
  get: () => i18n.global.locale.value,
  set: (value) => { if (UI_LOCALES.includes(value)) i18n.global.locale.value = value; },
});
export const formatLocale = computed(() => formatLocales[uiLocale.value]);

export function formatNumber(value: number, options?: Intl.NumberFormatOptions): string {
  return new Intl.NumberFormat(formatLocale.value, options).format(value);
}

export function t(key: MessageKey, params: Record<string, unknown> = {}): string {
  const values = Object.fromEntries(Object.entries(params).map(([name, value]) =>
    [name, typeof value === "number" ? formatNumber(value) : value]));
  return i18n.global.t(key, values, { plural: typeof params.count === "number" ? params.count : typeof params.p0 === "number" ? params.p0 : undefined });
}

export function formatDate(value: string | number, timeOnly = false): string {
  const date = new Date(value);
  if (!Number.isFinite(date.getTime())) return "—";
  return new Intl.DateTimeFormat(formatLocale.value, timeOnly
    ? { hour: "2-digit", minute: "2-digit" }
    : { year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit", second: "2-digit" }).format(date);
}

export function initializeLocale(): () => void {
  return watch(uiLocale, (locale) => {
    if (typeof document !== "undefined") {
      document.documentElement.lang = formatLocales[locale];
      document.documentElement.dir = "ltr";
      document.title = t("appName");
    }
    try { localStorage.setItem(LOCALE_STORAGE_KEY, locale); } catch { /* Keep in-memory switching usable. */ }
  }, { immediate: true, flush: "sync" });
}
