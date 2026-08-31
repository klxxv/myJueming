import { afterEach, describe, expect, it, vi } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { computed, defineComponent, nextTick, ref } from "vue";
import { mount } from "@vue/test-utils";
import { parse } from "vue/compiler-sfc";
import { i18n, initializeLocale, uiLocale, resolveLocale, formatDate, formatNumber, t, LOCALE_STORAGE_KEY, UI_LOCALES, type MessageKey } from "../src/i18n";
import { formatError, rawErrorMessage, revisionAction, revisionSummary, LocalizedError } from "../src/i18n/kernel-messages";
import { fallbackLanguages, languageLabel } from "../src/domain/languages";
import { useViewModeController } from "../src/composables/useViewModeController";
import SettingsWorkspace from "../src/components/SettingsWorkspace.vue";
import AnnotationPanel from "../src/components/AnnotationPanel.vue";
import SegmentContentDialog from "../src/components/SegmentContentDialog.vue";
import SearchReplaceWorkspace from "../src/components/SearchReplaceWorkspace.vue";

afterEach(() => { uiLocale.value = "zh"; localStorage.clear(); vi.useRealTimers(); });

describe("locale preference", () => {
  it("prefers a valid saved language, normalizes system languages, and has a safe fallback", () => {
    expect(resolveLocale("fr", ["en-US"])).toBe("fr");
    expect(resolveLocale(null, ["de-DE", "fr-CA"])).toBe("fr");
    expect(resolveLocale("corrupt", ["EN_us"])).toBe("en");
    expect(resolveLocale(null, ["zh-Hant-TW"])).toBe("zh");
    expect(resolveLocale(null, ["de", "ar"])).toBe("zh");
    expect(resolveLocale(null, [])).toBe("zh");
  });

  it("updates document language, title and persisted preference synchronously", () => {
    const stop = initializeLocale();
    try {
      uiLocale.value = "fr";
      expect(document.documentElement.lang).toBe("fr-FR");
      expect(document.documentElement.dir).toBe("ltr");
      expect(document.title).toBe("Jueming Aligner");
      expect(resolveLocale(localStorage.getItem(LOCALE_STORAGE_KEY), ["en"])).toBe("fr");
      uiLocale.value = "zh";
      expect(document.title).toBe("决明对齐器");
    } finally { stop(); }
  });

  it("still switches when storage is disabled", () => {
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => { throw new Error("denied"); });
    const stop = initializeLocale();
    try { uiLocale.value = "en"; expect(t("navSettings")).toBe("Settings"); } finally { stop(); }
  });

  it("uses locale-aware dates/numbers and reactive messages", () => {
    const label = computed(() => t("navSettings"));
    uiLocale.value = "en";
    expect(label.value).toBe("Settings");
    expect(formatNumber(1234.5)).toBe("1,234.5");
    uiLocale.value = "fr";
    expect(label.value).toBe("Paramètres");
    expect(formatNumber(1234.5)).toBe("1\u202f234,5");
    expect(formatDate("2026-08-31T12:00:00Z")).toContain("31/08/2026");
    expect(formatDate("invalid")).toBe("—");
  });

  it("uses English and French plural rules, including zero", () => {
    uiLocale.value = "en";
    expect(t("resultCount", { count: 0 })).toBe("0 results");
    expect(t("resultCount", { count: 1 })).toBe("1 result");
    expect(t("resultCount", { count: 2 })).toBe("2 results");
    uiLocale.value = "fr";
    expect(t("resultCount", { count: 0 })).toBe("0 résultat");
    expect(t("resultCount", { count: 1 })).toBe("1 résultat");
    expect(t("resultCount", { count: 2 })).toBe("2 résultats");
  });
});

describe("catalogue contracts", () => {
  const catalogues = UI_LOCALES.map(locale => JSON.parse(readFileSync(resolve("src/i18n/locales", `${locale}.json`), "utf8")) as Record<MessageKey, string>);
  const placeholders = (text: string) => [...new Set(text.match(/\{\w+\}/g) ?? [])].sort();
  it("contains the same keys and placeholders in all three languages", () => {
    for (const catalogue of catalogues) {
      expect(Object.keys(catalogue).sort()).toEqual(Object.keys(catalogues[0]).sort());
      for (const key of Object.keys(catalogues[0]) as MessageKey[]) {
        expect(catalogue[key].trim(), key).not.toBe("");
        expect(placeholders(catalogue[key]), key).toEqual(placeholders(catalogues[0][key]));
      }
    }
    for (const catalogue of catalogues.slice(1)) {
      for (const [key, value] of Object.entries(catalogue)) expect(value, key).not.toMatch(/\p{Script=Han}/u);
    }
  });

  it("compiles and renders every translation without missing-key or syntax warnings", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const error = vi.spyOn(console, "error").mockImplementation(() => {});
    for (const locale of UI_LOCALES) {
      uiLocale.value = locale;
      for (const key of Object.keys(catalogues[0]) as MessageKey[]) {
        expect(i18n.global.te(key, locale), key).toBe(true);
        expect(t(key, { p0: 2, p1: "value", p2: 3, p3: "other", count: 2 }).length, key).toBeGreaterThan(0);
      }
    }
    expect(warn).not.toHaveBeenCalled();
    expect(error).not.toHaveBeenCalled();
  });

  it("keeps UI text and accessibility labels out of Vue templates", () => {
    const sourceRoot = resolve("src");
    const files = (dir: string): string[] => readdirSync(dir, { withFileTypes: true }).flatMap(entry => entry.isDirectory() ? files(join(dir, entry.name)) : [join(dir, entry.name)]);
    const failures: string[] = [];
    for (const file of files(sourceRoot).filter(file => file.endsWith(".vue"))) {
      const { descriptor } = parse(readFileSync(file, "utf8"));
      type TemplateNode = { type: number; content?: unknown; props?: { type: number; name?: string; value?: { content?: string } }[]; children?: (TemplateNode | string | symbol)[] };
      const visit = (node: TemplateNode) => {
        if (node.type === 2 && typeof node.content === "string" && /\p{L}/u.test(node.content)) failures.push(`${file}: ${node.content}`);
        for (const prop of node.props ?? []) {
          if (prop.type === 6 && ["title", "placeholder", "aria-label", "alt"].includes(prop.name ?? "") && /\p{L}/u.test(prop.value?.content ?? "")) failures.push(`${file}: ${prop.name}`);
        }
        for (const child of node.children ?? []) if (typeof child === "object") visit(child);
      };
      if (descriptor.template?.ast) visit(descriptor.template.ast);
    }
    expect(failures).toEqual([]);
  });

  it("keeps installer languages complete and preserves the existing MSI upgrade identity", () => {
    const config = JSON.parse(readFileSync(resolve("src-tauri/tauri.conf.json"), "utf8"));
    expect(config.bundle.windows.nsis.languages).toEqual(["SimpChinese", "English", "French"]);
    expect(config.bundle.windows.wix.upgradeCode).toBe("2e90697d-b29d-58f5-b283-b08403419c63");
    const template = readFileSync(resolve("src-tauri", config.bundle.windows.wix.template), "utf8");
    const referencedKeys = [...new Set([...template.matchAll(/!\(loc\.(\w+)\)/g)].map(match => match[1]))].filter(key => !["TauriLanguage", "TauriCodepage"].includes(key));
    expect(Object.keys(config.bundle.windows.wix.language).sort()).toEqual(["en-US", "fr-FR", "zh-CN"]);
    for (const [locale, configValue] of Object.entries(config.bundle.windows.wix.language)) {
      const xml = new DOMParser().parseFromString(readFileSync(resolve("src-tauri", (configValue as { localePath: string }).localePath), "utf8"), "application/xml");
      expect(xml.querySelector("parsererror")).toBeNull();
      expect(xml.documentElement.getAttribute("Culture")).toBe(locale);
      for (const key of referencedKeys) expect(xml.querySelector(`String[Id="${key}"]`)?.textContent?.trim(), `${locale}:${key}`).toBeTruthy();
    }
    expect(template).toContain('UpgradeCode="{{upgrade_code}}"');
    expect(template).not.toContain("{{product_name}}");
  });
});

describe("presentation-only localization", () => {
  it("localizes project language names without changing the language catalogue", () => {
    const before = JSON.stringify(fallbackLanguages);
    uiLocale.value = "fr";
    expect(languageLabel(fallbackLanguages, "en")).toBe("anglais");
    expect(fallbackLanguages.map(x => x.language_id)).toContain("bn");
    expect(JSON.stringify(fallbackLanguages)).toBe(before);
  });

  it("localizes diagnostics without changing raw errors or control-flow strings", () => {
    const raw = "one or more selected segments are already aligned; confirm replacement before linking";
    uiLocale.value = "fr";
    expect(formatError(raw)).toContain("Certains segments");
    expect(rawErrorMessage(raw)).toBe(raw);
    expect(formatError({ code: "cancelled", message: "cancelled" })).toBe("Annuler");
    expect(formatError({ code: "not_found", message: "No local project is open." })).toContain("Créez");
    expect(formatError("unknown diagnostic: file.txt")).toBe("unknown diagnostic: file.txt");
    expect(formatError("stale revision: expected 9007199254740993, provided 1")).toContain("9007199254740993");
    const error = new LocalizedError("desktopContentRequired");
    const displayed = computed(() => formatError(error));
    expect(displayed.value).toBe(t("desktopContentRequired"));
    uiLocale.value = "en";
    expect(displayed.value).toBe(t("desktopContentRequired"));
  });

  it("localizes generated history but preserves opaque IDs and user text", () => {
    const summary = "Created annotation 术语核对 <script> é";
    uiLocale.value = "fr";
    expect(revisionAction("undo:10:9")).toBe("Annuler");
    expect(revisionSummary(summary)).toContain("术语核对 <script> é");
    expect(revisionSummary("Custom history note 原文")).toBe("Custom history note 原文");
    expect(summary).toBe("Created annotation 术语核对 <script> é");
  });

  it("preserves a dirty edit session across locale changes without kernel writes", async () => {
    vi.useFakeTimers();
    const persist = vi.fn();
    let controller!: ReturnType<typeof useViewModeController>;
    const wrapper = mount(defineComponent({ setup() { controller = useViewModeController({ autosaveDelayMs: ref(30000), persist }); return () => null; } }));
    controller.enterEdit("opaque-segment", "opaque-alignment", "原文 é");
    controller.updateDraft("原文 é — draft");
    for (const locale of UI_LOCALES) { uiLocale.value = locale; await nextTick(); }
    expect(controller.editSession.value).toMatchObject({ segmentId: "opaque-segment", alignmentId: "opaque-alignment", draft: "原文 é — draft", status: "dirty" });
    expect(persist).not.toHaveBeenCalled();
    expect(controller.requestMode("review")).toBe("guarded");
    wrapper.unmount();
  });

  it("retains raw save errors so visible failures can change language", async () => {
    vi.useFakeTimers();
    const error = new LocalizedError("desktopContentRequired");
    let controller!: ReturnType<typeof useViewModeController>;
    const wrapper = mount(defineComponent({ setup() { controller = useViewModeController({ autosaveDelayMs: ref(30000), persist: async () => { throw error; } }); return () => null; } }));
    controller.enterEdit("segment-id", "alignment-id", "original");
    controller.updateDraft("draft");
    expect(await controller.persistDraft(false)).toBe(false);
    const displayed = computed(() => formatError(controller.editSession.value?.error));
    uiLocale.value = "fr";
    expect(displayed.value).toBe(t("desktopContentRequired"));
    uiLocale.value = "en";
    expect(displayed.value).toBe(t("desktopContentRequired"));
    expect(controller.editSession.value?.draft).toBe("draft");
    wrapper.unmount();
  });

  it("switches settings in place without remounting or changing other preferences", async () => {
    const wrapper = mount(SettingsWorkspace, { props: { usesMacShortcuts: false, shortcutRows: [], cacheCleaning: false, lastCacheCleanupAt: null, theme: "eye", fontScale: 110, uiScale: 100, shortcutProfile: "auto", trackpadOptimized: true, autoSaveDelayMs: 3000, cacheCleanupPolicy: "weekly" } });
    await wrapper.get('select[aria-label="界面语言"]').setValue("fr");
    expect(wrapper.text()).toContain("Paramètres");
    expect(wrapper.text()).toContain("Apparence et échelle");
    expect(wrapper.props("theme")).toBe("eye");
    expect(wrapper.emitted("applyUi")).toBeUndefined();
    await wrapper.get('select[aria-label="Langue de l’interface"]').setValue("en");
    expect(wrapper.text()).toContain("Interface language");
    wrapper.unmount();
  });

  it("refreshes annotation statuses without translating annotation bodies", async () => {
    const wrapper = mount(AnnotationPanel, { props: { annotations: [{ id: "annotation-id", number: 1, status: "draft", title: "原文 title", body: "中文 English français", createdAt: "10:00", links: [] }] } });
    uiLocale.value = "fr";
    await nextTick();
    expect(wrapper.text()).toContain("Brouillon");
    expect(wrapper.text()).toContain("中文 English français");
    expect(wrapper.text()).not.toContain("草稿");
    wrapper.unmount();
  });

  it("translates search scopes and fallback project labels without changing query DTOs", async () => {
    const wrapper = mount(SearchReplaceWorkspace, { props: { results: [], query: "原文 é", side: "both" } });
    for (const locale of UI_LOCALES) {
      uiLocale.value = locale;
      await nextTick();
      expect(wrapper.text()).toContain(t("currentProject"));
      expect(wrapper.get(".sr-side-button").text()).not.toContain("both");
      await wrapper.get(".sr-side-button").trigger("click");
      expect(wrapper.emitted("update:side")?.slice(-1)[0]).toEqual(["source"]);
      await wrapper.get("form").trigger("submit");
      expect(wrapper.emitted("search")?.slice(-1)[0]).toEqual([{ query: "原文 é", side: "both", regex: false, caseSensitive: false }]);
    }
    wrapper.unmount();
  });

  it("retains a split draft and its lossless marker when language changes", async () => {
    const wrapper = mount(SegmentContentDialog, { props: { operation: { kind: "split", segmentId: "stable-id", original: "alphaβ" } }, global: { stubs: { Teleport: true } } });
    const marker = t("splitMarker");
    await wrapper.get("textarea").setValue(`alpha${marker}β`);
    uiLocale.value = "fr";
    await nextTick();
    expect(wrapper.text()).toContain("sans perte");
    await wrapper.get("footer .primary-button").trigger("click");
    expect(wrapper.emitted("split")?.[0]).toEqual(["stable-id", ["alpha", "β"]]);
    wrapper.unmount();
  });
});
