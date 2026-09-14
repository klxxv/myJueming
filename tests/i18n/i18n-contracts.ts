import assert from "node:assert/strict";
import { UI_LOCALES, resolveLocale, messages, i18n, setUiLocale, t } from "../../apps/desktop/src/i18n";
import { createSSRApp, h, computed } from "vue";
import { renderToString } from "vue/server-renderer";
import SettingsWorkspace from "../../apps/desktop/src/components/SettingsWorkspace.vue";
import AgentPanel from "../../apps/desktop/src/components/AgentPanel.vue";
import AnnotationPanel from "../../apps/desktop/src/components/AnnotationPanel.vue";
import AgentConnectionSettings from "../../apps/desktop/src/components/AgentConnectionSettings.vue";
import AgentModelSettings from "../../apps/desktop/src/components/AgentModelSettings.vue";
import ResearchFeatureSettings from "../../apps/desktop/src/components/ResearchFeatureSettings.vue";
import ImportSegmentationPreview from "../../apps/desktop/src/components/ImportSegmentationPreview.vue";
import CompanionGarden from "../../apps/desktop/src/components/CompanionGarden.vue";
import CompanionHabitat from "../../apps/desktop/src/components/CompanionHabitat.vue";
import AgentActionBar from "../../apps/desktop/src/components/AgentActionBar.vue";
import BookmarksWorkspace from "../../apps/desktop/src/components/BookmarksWorkspace.vue";
import HistoryWorkspace from "../../apps/desktop/src/components/HistoryWorkspace.vue";
import SearchReplaceWorkspace from "../../apps/desktop/src/components/SearchReplaceWorkspace.vue";
import ResearchWorkspace from "../../apps/desktop/src/components/ResearchWorkspace.vue";
import PipelineV2Workspace from "../../apps/desktop/src/components/PipelineV2Workspace.vue";
import AlignmentUngroupDialog from "../../apps/desktop/src/components/AlignmentUngroupDialog.vue";
import SegmentContentDialog from "../../apps/desktop/src/components/SegmentContentDialog.vue";
import { createSettingsCapabilities } from "../../apps/desktop/src/settings/capabilities";
import { createDefaultAppSettings, parseAppSettings } from "../../apps/desktop/src/settings/schema";
import { formatError, revisionAction, revisionSummary } from "../../apps/desktop/src/i18n/kernel-messages";

const expectedUiLocales = ["zh", "en", "ja", "fr", "de"] as const;
const expectedSettingsLanguages = ["zh-CN", "en", "ja", "fr", "de"] as const;

type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };

function flattenStrings(value: JsonValue, prefix = ""): Map<string, string> {
  const result = new Map<string, string>();
  if (typeof value === "string") {
    result.set(prefix, value);
    return result;
  }
  assert.ok(value !== null && typeof value === "object" && !Array.isArray(value),
    `translation ${prefix || "<root>"} must be a string or object`);
  for (const [key, child] of Object.entries(value)) {
    const childPrefix = prefix ? `${prefix}.${key}` : key;
    for (const [leafKey, text] of flattenStrings(child, childPrefix)) result.set(leafKey, text);
  }
  return result;
}

function placeholders(text: string): string[] {
  return [...new Set(text.match(/\{[A-Za-z_]\w*\}/g) ?? [])].sort();
}

assert.deepEqual([...UI_LOCALES], expectedUiLocales,
  "the bundled UI locale list must contain Chinese, English, Japanese, French, and German in the agreed order");

const catalogues = new Map(expectedUiLocales.map((locale) => {
  return [locale, flattenStrings(messages[locale])] as const;
}));

const reference = catalogues.get("zh")!;
assert.ok(reference.size > 0, "the Chinese catalogue must not be empty");
const referenceKeys = [...reference.keys()].sort();

for (const [locale, catalogue] of catalogues) {
  assert.deepEqual([...catalogue.keys()].sort(), referenceKeys,
    `${locale}.json must contain exactly the same translation keys as zh.json`);
  for (const key of referenceKeys) {
    const text = catalogue.get(key);
    assert.equal(typeof text, "string", `${locale}.${key} is missing`);
    assert.ok(text!.trim().length > 0, `${locale}.${key} must not be empty or whitespace-only`);
    assert.deepEqual(placeholders(text!), placeholders(reference.get(key)!),
      `${locale}.${key} must preserve the placeholders used by zh.${key}`);
  }
}

assert.equal(reference.get("navPipeline"), "处理流程",
  "the Chinese navigation catalogue must expose navPipeline as 处理流程");
assert.equal(catalogues.get("en")!.get("navPipeline"), "Pipeline",
  "the English navigation catalogue must expose navPipeline as Pipeline");

for (const locale of expectedUiLocales) {
  assert.equal(resolveLocale(locale, []), locale, `saved locale ${locale} must round-trip`);
}
assert.equal(resolveLocale("zh-CN", []), "zh", "the settings value zh-CN must map to the zh UI catalogue");
assert.equal(resolveLocale(null, ["ja-JP"]), "ja", "Japanese system locales must resolve to ja");
assert.equal(resolveLocale(null, ["fr-CA"]), "fr", "French system locales must resolve to fr");
assert.equal(resolveLocale(null, ["de-DE"]), "de", "German system locales must resolve to de");

for (const language of expectedSettingsLanguages) {
  const parsed = parseAppSettings({ device: { general: { interfaceLanguage: language } } }, false);
  assert.equal(parsed.device.general.interfaceLanguage, language,
    `settings schema must accept and preserve ${language}`);
}
assert.equal(
  parseAppSettings({ device: { general: { interfaceLanguage: "unsupported" } } }, false).device.general.interfaceLanguage,
  createDefaultAppSettings(false).device.general.interfaceLanguage,
  "settings schema must safely reject unsupported interface languages",
);

// Render real components with the real translation runtime. Equal catalogue
// keys alone cannot detect untranslated text still hard-coded in a template.
const navLabel = computed(() => t("navSettings"));
i18n.global.setMissingHandler((locale, key) => { throw new Error(`Missing runtime translation: ${locale}.${key}`); });
const sections = ["general", "appearance", "accessibility", "input", "pet", "agent", "persistence", "storage", "privacy", "about"];
for (const locale of expectedUiLocales) {
  setUiLocale(locale);
  assert.equal(formatError("no earlier revision is available for undo"), t("errorNoUndo"));
  assert.equal(revisionAction("undo"), t("undo"));
  assert.equal(revisionSummary("Imported 2 source and 3 target segments"), t("revisionImport", { p0: "2", p1: "3" }));
  assert.equal(formatError("User-supplied diagnostic"), "User-supplied diagnostic", "unknown details are never discarded");
  if (locale === "ja" || locale === "de") {
    for (const key of ["errorNoUndo", "errorDecode", "revisionImport", "revisionReplace"] as const) {
      assert.notEqual(messages[locale][key], messages.en[key], `${locale}.${key} must not retain the English baseline`);
    }
  }
  assert.equal(navLabel.value, messages[locale].navSettings, "computed labels follow language changes");
  for (const section of sections) {
    const settings = createDefaultAppSettings(false);
    settings.device.navigation.lastSection = section;
    const app = createSSRApp({ render: () => h(SettingsWorkspace, {
      settings, capabilities: createSettingsCapabilities(), usesMacShortcuts: false,
      shortcutRows: [], cacheCleaning: false, lastCacheCleanupAt: 1700000000000,
      settingsSaving: false, settingsSaveError: null, effectiveMotionMode: "standard",
      motionSummary: t("standard"), systemReducedMotion: false, projectOpen: true, appVersion: "test",
    }) });
    app.use(i18n);
    const html = await renderToString(app);
    if (["en", "fr", "de"].includes(locale)) {
      const visible = html.replace(/<option value="(?:zh-CN|ja)"[^>]*>[\s\S]*?<\/option>/g, "");
      assert.ok(!/\p{Script=Han}/u.test(visible), `${locale} settings/${section} contains untranslated Chinese: ${visible.match(/.{0,20}\p{Script=Han}.{0,40}/u)?.[0]}`);
    }
    assert.ok(!html.includes("undefined"), `${locale} settings/${section} rendered an undefined value`);
  }
  for (const [component, props] of [
    [AgentPanel, { tabLabel: t("navSettings"), selectedText: "", connected: false, runtimeAvailable: false, running: false, reviewing: false, showContext: true, messages: [], activity: [], proposals: [], error: null }],
    [AnnotationPanel, { annotations: [], readonly: true }],
    [AgentConnectionSettings, {}],
    [AgentModelSettings, {}],
    [ResearchFeatureSettings, { available: true, feature: null }],
    [BookmarksWorkspace, { bookmarks: [], previews: [], sourceSegments: [], targetSegments: [], alignments: [], segmentLabels: new Map(), alignmentLabels: new Map() }],
    [HistoryWorkspace, { revisions: [], diff: null }],
    [HistoryWorkspace, { revisions: [{ id: "1", label: "R1", timestamp: "2026-01-01", action: "edit", summary: "Test", current: true }], diff: { segmentId: "seg-1", sourceOld: "Before", sourceNew: "After", targetOld: "Before", targetNew: "After" } }],
    [AgentActionBar, { proposal: { id: "proposal-1", title: "Test", status: "pending", revision: "1", changes: [{ id: "seg-1", before: "Before", after: "After" }] }, busy: false }],
    [AlignmentUngroupDialog, { alignment: { id: "alignment-1", sourceIds: ["s1", "s2"], targetIds: ["t1", "t2"] }, sourceSegments: [], targetSegments: [] }],
    [SegmentContentDialog, { operation: { kind: "merge", segmentIds: ["s1", "s2"], initialValue: "Example text" } }],
    [SegmentContentDialog, { operation: { kind: "split", segmentId: "s1", original: "Example text" } }],
    [SearchReplaceWorkspace, { results: [] }],
    [ResearchWorkspace, { bindingId: null, projectId: null, revisionId: "0", available: false, active: false, feature: null }],
    [PipelineV2Workspace, { bindingId: null, projectId: null, revisionId: "0", active: false, available: false, refreshKey: 0 }],
    [CompanionHabitat, { settings: createDefaultAppSettings(false).device.pet, activity: "idle", inGarden: false, dirtyEditor: false, motion: "off" }],
    [CompanionGarden, { settings: createDefaultAppSettings(false).device.pet, activity: "running", dirtyEditor: false, motion: "off", proposal: null, busy: false, guidance: "" }],
    [ImportSegmentationPreview, { preview: null, loading: false, error: null }],
    [ImportSegmentationPreview, { preview: null, loading: true, error: null }],
    [ImportSegmentationPreview, { preview: null, loading: false, error: "Test failure" }],
    [ImportSegmentationPreview, { loading: false, error: null, preview: {
      label: "fixture.txt", profile: { encoding: "utf8" }, had_bom: true, encoding_detection: "statistical",
      preview: { warnings: [], segments: [
        { ordinal: 0, original_text: "Example.", content: "Example.", boundary: "sentence_punctuation", boundary_marker: ".", cleanups: ["seg_wrappers", "pos_suffixes_and_whitespace", "cjk_spaces", "punctuation_spaces"] },
        { ordinal: 1, original_text: "End", content: "End", boundary: "text_end", boundary_marker: null, cleanups: [] },
        { ordinal: 2, original_text: "Line", content: "Line", boundary: "non_empty_line", boundary_marker: null, cleanups: [] },
      ] },
    } }],
  ] as const) {
    const app = createSSRApp({ render: () => h(component as any, props) });
    app.use(i18n);
    const context = { teleports: {} as Record<string, string> };
    const rendered = await renderToString(app, context);
    const html = rendered + Object.values(context.teleports).join("");
    if (["en", "fr", "de"].includes(locale)) assert.ok(!/\p{Script=Han}/u.test(html), `${locale} component contains untranslated Chinese: ${html.match(/.{0,20}\p{Script=Han}.{0,40}/u)?.[0]}`);
  }
}
setUiLocale("zh-CN");
console.log(`i18n contracts and real component renders passed (${reference.size} keys, 5 locales, 50 settings pages)`);
