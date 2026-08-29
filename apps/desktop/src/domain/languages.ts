import type { SupportedLanguageDto } from "./kernel-client";

/**
 * The Phase 0 LTR language directory used when the desktop runtime is not
 * available yet. The Kernel remains authoritative when a project is open.
 */
export const fallbackLanguages: SupportedLanguageDto[] = [
  ["en", "English"], ["zh", "中文（简体）"], ["hi", "हिन्दी"], ["es", "Español"], ["fr", "Français"],
  ["bn", "বাংলা"], ["pt", "Português"], ["ru", "Русский"], ["id", "Bahasa Indonesia"], ["de", "Deutsch"],
].map(([language_id, native_name]) => ({ language_id, native_name, english_name: native_name }));

export const languageLabel = (languages: readonly SupportedLanguageDto[], languageId: string) =>
  languages.find((language) => language.language_id === languageId)?.native_name ?? languageId;
