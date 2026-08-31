<script setup lang="ts">
import { t, type LocalizedMessage } from '../i18n';
import { formatError, importWarning } from '../i18n/kernel-messages';
import { computed, ref } from "vue";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { Check, FolderOpen, X } from "@lucide/vue";
import { fallbackLanguages, languageLabel } from "../domain/languages";
import {
  makeImportProfile,
  type Encoding,
  type ImportPreviewResponse,
  type KernelClient,
  type ProjectSnapshot,
  type SegmentationMode,
  type SupportedLanguageDto,
} from "../domain/kernel-client";

const props = defineProps<{ kernelClient: KernelClient }>();
const busy = defineModel<boolean>("busy", { required: true });
const emit = defineEmits<{
  created: [snapshot: ProjectSnapshot, projectPath: string];
  status: [message: LocalizedMessage];
}>();

const visible = ref(false);
const previewTab = ref<"source" | "target">("source");
const projectName = ref("");
const projectDirectory = ref("");
const sourcePath = ref("");
const targetPath = ref("");
const sourceEncoding = ref<Encoding>("utf8");
const targetEncoding = ref<Encoding>("utf8");
const sourceLanguage = ref("zh");
const targetLanguage = ref("en");
const supportedLanguages = ref<SupportedLanguageDto[]>(fallbackLanguages);
const sourceSegmentation = ref<SegmentationMode>("non_empty_line");
const targetSegmentation = ref<SegmentationMode>("non_empty_line");
const sourcePreview = ref<ImportPreviewResponse | null>(null);
const targetPreview = ref<ImportPreviewResponse | null>(null);

const activePreview = computed(() => previewTab.value === "source" ? sourcePreview.value : targetPreview.value);
const languageOptions = computed(() => supportedLanguages.value.map((language) => ({ id: language.language_id, label: languageLabel(supportedLanguages.value, language.language_id) })));
const displayLanguageLabel = (id: string) => languageLabel(supportedLanguages.value, id);
const errorMessage = formatError;

function reset() {
  projectDirectory.value = "";
  sourcePath.value = "";
  targetPath.value = "";
  sourceEncoding.value = "utf8";
  targetEncoding.value = "utf8";
  sourceLanguage.value = "zh";
  targetLanguage.value = "en";
  sourceSegmentation.value = "non_empty_line";
  targetSegmentation.value = "non_empty_line";
  sourcePreview.value = null;
  targetPreview.value = null;
  previewTab.value = "source";
}

async function open() {
  reset();
  visible.value = true;
  try {
    supportedLanguages.value = await props.kernelClient.listSupportedLanguages();
  } catch {
    supportedLanguages.value = fallbackLanguages;
  }
}

function close() {
  visible.value = false;
}

async function chooseProjectDirectory() {
  const selected = await openFileDialog({ directory: true, multiple: false, title: t('projectLocationDialog') });
  if (selected) projectDirectory.value = selected;
}

async function chooseTextFile(side: "source" | "target") {
  const selected = await openFileDialog({
    directory: false,
    multiple: false,
    title: side === "source" ? t('chooseSource') : t('chooseTarget'),
    filters: [{ name: t('textFiles'), extensions: ["txt"] }],
  });
  if (!selected) return;
  if (side === "source") sourcePath.value = selected;
  else targetPath.value = selected;
  if (/阿古顿巴/i.test(selected)) {
    sourceEncoding.value = "gb18030";
    sourceSegmentation.value = "legacy_tagged_line";
    targetSegmentation.value = "legacy_tagged_line";
  }
  if (/Akhu Tenpa/i.test(selected)) targetSegmentation.value = "legacy_tagged_line";
  await previewFile(side);
}

async function previewFile(side: "source" | "target") {
  const path = side === "source" ? sourcePath.value : targetPath.value;
  if (!path) return;
  const encoding = side === "source" ? sourceEncoding.value : targetEncoding.value;
  const segmentation = side === "source" ? sourceSegmentation.value : targetSegmentation.value;
  busy.value = true;
  try {
    const preview = await props.kernelClient.previewImport({
      input: { kind: "file", path },
      profile: makeImportProfile(encoding, segmentation),
    });
    if (side === "source") sourcePreview.value = preview;
    else targetPreview.value = preview;
    previewTab.value = side;
    emit("status", () => t('importPreviewReady', { p0: side === "source" ? t('source') : t('target'), p1: preview.preview.segments.length }));
  } catch (error) {
    emit("status", () => t('previewFailed', { p0: errorMessage(error) }));
  } finally {
    busy.value = false;
  }
}

function projectPath() {
  return `${projectDirectory.value.replace(/[\\/]+$/, "")}\\${projectName.value.trim().replace(/[<>:\"/\\|?*]/g, "_")}.jm`;
}

async function createProject() {
  if (!sourcePath.value || !targetPath.value || !projectDirectory.value || !projectName.value.trim()) {
    emit("status", () => t('projectRequiredFields'));
    return;
  }
  busy.value = true;
  try {
    const createdPath = projectPath();
    const snapshot = await props.kernelClient.createProject({
      project_path: createdPath,
      name: projectName.value.trim(),
      source: {
        language_id: sourceLanguage.value,
        title: t('sourceDocumentTitle', { p0: displayLanguageLabel(sourceLanguage.value) }),
        input: { kind: "file", path: sourcePath.value },
        profile: makeImportProfile(sourceEncoding.value, sourceSegmentation.value),
      },
      target: {
        language_id: targetLanguage.value,
        title: t('targetDocumentTitle', { p0: displayLanguageLabel(targetLanguage.value) }),
        input: { kind: "file", path: targetPath.value },
        profile: makeImportProfile(targetEncoding.value, targetSegmentation.value),
      },
    });
    visible.value = false;
    emit("created", snapshot, createdPath);
  } catch (error) {
    emit("status", () => t('createFailed', { p0: errorMessage(error) }));
  } finally {
    busy.value = false;
  }
}

defineExpose({ open });
</script>

<template>
  <div v-if="visible" class="modal-backdrop" @click.self="close">
    <section class="import-modal" role="dialog" aria-modal="true" aria-labelledby="import-title">
      <header><div><span class="eyebrow">{{ t('newProjectEyebrow') }}</span><h2 id="import-title">{{ t('newParallelProject') }}</h2></div><button type="button" :title="t('closeImportWizard')" @click="close"><X :size="19" /></button></header>
      <div class="import-project-grid"><label>{{ t('projectName') }}<input v-model="projectName" :placeholder="t('projectNamePlaceholder')" /></label><label>{{ t('saveLocation') }}<div class="compact-picker"><input v-model="projectDirectory" readonly :placeholder="t('chooseParentFolder')" /><button type="button" @click="chooseProjectDirectory">{{ t('browse') }}</button></div></label></div>
      <div class="language-pair-picker"><label>{{ t('sourceLanguage') }} <select v-model="sourceLanguage"><option v-for="language in languageOptions" :key="language.id" :value="language.id">{{ language.label }}</option></select></label><label>{{ t('targetLanguage') }} <select v-model="targetLanguage"><option v-for="language in languageOptions" :key="language.id" :value="language.id">{{ language.label }}</option></select></label><small>{{ t('supportedLanguagesHint') }}</small></div>
      <div class="import-source-grid">
        <section><div class="import-path"><label for="source-path">{{ displayLanguageLabel(sourceLanguage) }}{{ t('sourceSuffix') }}</label><div><FolderOpen :size="17" /><input id="source-path" v-model="sourcePath" readonly :placeholder="t('chooseSourceTxt')" /><button type="button" @click="chooseTextFile('source')">{{ t('browse') }}</button></div></div><div class="import-settings"><label>{{ t('segmentationMode') }} <select v-model="sourceSegmentation" @change="previewFile('source')"><option value="non_empty_line">{{ t('nonEmptyLines') }}</option><option value="sentence_rules">{{ t('sentenceRules') }}</option><option value="legacy_tagged_line">{{ t('legacyTaggedLines') }}</option></select></label><label>{{ t('encoding') }} <select v-model="sourceEncoding" @change="previewFile('source')"><option value="utf8">{{ t('encodingUtf8') }}</option><option value="utf8-bom">{{ t('encodingBom') }}</option><option value="gb18030">{{ t('encodingGb') }}</option></select></label></div></section>
        <section><div class="import-path"><label for="target-path">{{ displayLanguageLabel(targetLanguage) }}{{ t('targetSuffix') }}</label><div><FolderOpen :size="17" /><input id="target-path" v-model="targetPath" readonly :placeholder="t('chooseTargetTxt')" /><button type="button" @click="chooseTextFile('target')">{{ t('browse') }}</button></div></div><div class="import-settings"><label>{{ t('segmentationMode') }} <select v-model="targetSegmentation" @change="previewFile('target')"><option value="non_empty_line">{{ t('nonEmptyLines') }}</option><option value="sentence_rules">{{ t('sentenceRules') }}</option><option value="legacy_tagged_line">{{ t('legacyTaggedLines') }}</option></select></label><label>{{ t('encoding') }} <select v-model="targetEncoding" @change="previewFile('target')"><option value="utf8">{{ t('encodingUtf8') }}</option><option value="utf8-bom">{{ t('encodingBom') }}</option><option value="gb18030">{{ t('encodingGb') }}</option></select></label></div></section>
      </div>
      <div class="preview-tabs"><button :class="{ active: previewTab === 'source' }" type="button" @click="previewTab = 'source'">{{ displayLanguageLabel(sourceLanguage) }}{{ t('sourcePreviewSuffix') }} {{ t('segmentCount', { count: sourcePreview?.preview.segments.length ?? 0 }) }}</button><button :class="{ active: previewTab === 'target' }" type="button" @click="previewTab = 'target'">{{ displayLanguageLabel(targetLanguage) }}{{ t('targetPreviewSuffix') }} {{ t('segmentCount', { count: targetPreview?.preview.segments.length ?? 0 }) }}</button></div>
      <div class="segment-preview"><div v-if="!activePreview" class="preview-empty">{{ t('importPreviewHint') }}</div><div v-for="text in activePreview?.preview.segments ?? []" :key="text.ordinal" class="preview-line"><span>{{ String(text.ordinal + 1).padStart(2, '0') }}</span><p>{{ text.content }}</p></div></div>
      <p v-for="warning in activePreview?.preview.warnings ?? []" :key="warning" class="import-warning" role="status">{{ importWarning(warning) }}</p>
      <footer><span class="import-contract">{{ t('importStorageHint') }}</span><button class="secondary-button" type="button" @click="close">{{ t('cancel') }}</button><button class="primary-button" type="button" :disabled="busy" @click="createProject"><Check :size="15" />{{ t('createOpenProject') }}</button></footer>
    </section>
  </div>
</template>

<style scoped>
.import-warning { margin: 0; padding: 6px 24px; color: var(--ink-700); font-size: 12px; }
.import-modal > footer { flex-wrap: wrap; gap: 10px; }
.import-contract { flex: 1 1 280px; }
.modal-backdrop { position: fixed; z-index: 20; inset: 0; display: grid; place-items: center; background: rgb(31 42 34 / 22%); }
.import-modal { display: flex; flex-direction: column; width: min(980px, calc(100vw - 80px)); max-height: calc(100vh - 90px); overflow: hidden; border: 1px solid #cbd6cc; border-radius: 10px; background: var(--surface-raised); box-shadow: 0 22px 70px rgb(29 48 32 / 23%); }
.import-modal > header, .import-modal > footer { display: flex; align-items: center; justify-content: space-between; padding: 20px 24px; border-bottom: 1px solid var(--line); }
.import-modal > header button { padding: 5px; border: 0; background: transparent; color: var(--ink-500); cursor: pointer; }
.import-modal > header h2 { margin: 3px 0 0; color: var(--ink-900); font-size: 21px; }
.eyebrow { color: var(--green-700); font-size: 10px; font-weight: 700; letter-spacing: .1em; }
.import-project-grid, .import-source-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; padding: 16px 24px; }
.import-project-grid { border-bottom: 1px solid var(--line); background: var(--surface-subtle); }
.import-project-grid > label { display: grid; gap: 7px; color: var(--ink-700); font-size: 12px; }
.import-project-grid > label > input, .compact-picker { height: 38px; border: 1px solid #c8d3c9; border-radius: 6px; background: var(--surface-raised); }
.import-project-grid > label > input { padding: 0 11px; outline: none; }
.compact-picker { display: flex; overflow: hidden; }
.compact-picker input { flex: 1; min-width: 0; padding: 0 11px; border: 0; outline: none; background: transparent; }
.compact-picker button { padding: 0 13px; border: 0; border-left: 1px solid #c8d3c9; color: var(--green-900); background: var(--surface-raised); cursor: pointer; }
.language-pair-picker { display: flex; align-items: center; gap: 18px; padding: 12px 24px; border-bottom: 1px solid var(--line); color: var(--ink-700); background: var(--surface-subtle); font-size: 12px; }
.language-pair-picker label { display: flex; align-items: center; gap: 7px; }
.language-pair-picker select { height: 30px; border: 1px solid #cbd6cc; border-radius: 4px; color: var(--ink-700); background: var(--surface-raised); }
.language-pair-picker small { margin-left: auto; color: var(--ink-500); }
.import-source-grid { padding-top: 3px; padding-bottom: 3px; }
.import-source-grid > section { min-width: 0; }
.import-path { padding: 12px 0 8px; }
.import-path label { display: block; margin-bottom: 8px; color: var(--ink-700); font-size: 13px; }
.import-path > div { display: flex; align-items: center; gap: 8px; height: 40px; padding: 0 11px; border: 1px solid #c8d3c9; border-radius: 6px; color: var(--ink-500); }
.import-path input { flex: 1; min-width: 0; border: 0; outline: none; background: transparent; }
.import-path button { height: 29px; padding: 0 10px; border: 1px solid #c4d0c5; border-radius: 4px; color: var(--green-900); background: var(--surface-raised); cursor: pointer; }
.import-settings { display: flex; justify-content: space-between; gap: 10px; padding: 8px 0 12px; }
.import-settings label { display: flex; align-items: center; gap: 7px; color: var(--ink-700); font-size: 12px; }
.import-settings select { height: 30px; border: 1px solid #cbd6cc; border-radius: 4px; color: var(--ink-700); background: var(--surface-raised); }
.preview-tabs { display: flex; gap: 4px; padding: 0 24px; border-bottom: 1px solid var(--line); }
.preview-tabs button { padding: 11px 13px; border: 0; border-bottom: 2px solid transparent; color: var(--ink-500); background: transparent; font-size: 13px; cursor: pointer; }
.preview-tabs button.active { border-bottom-color: var(--green-700); color: var(--green-900); font-weight: 600; }
.segment-preview { min-height: 170px; max-height: 255px; overflow: auto; padding: 5px 24px; }
.preview-empty { display: grid; min-height: 160px; place-items: center; color: var(--ink-500); font-size: 13px; }
.preview-line { display: grid; grid-template-columns: 30px 1fr; gap: 11px; padding: 8px 0; border-bottom: 1px solid var(--line); }
.preview-line span { color: var(--ink-500); font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: 11px; }
.preview-line p { margin: 0; color: var(--ink-900); font-size: 13px; line-height: 1.5; }
.import-modal > footer { justify-content: flex-end; gap: 10px; border-top: 1px solid var(--line); border-bottom: 0; }
.import-contract { margin-right: auto; color: var(--ink-500); font-size: 11px; }
.primary-button, .secondary-button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; height: 38px; padding: 0 17px; border-radius: 6px; cursor: pointer; }
.primary-button { border: 1px solid var(--green-900); color: #fff; background: var(--green-900); }
.secondary-button { border: 1px solid #bdcabf; color: var(--ink-700); background: var(--surface-raised); }
.primary-button:disabled { opacity: .55; cursor: wait; }
</style>
