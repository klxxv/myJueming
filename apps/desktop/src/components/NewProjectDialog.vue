<script setup lang="ts">
import { encodingLabels, type EncodingChoice } from "../domain/import-encoding";
import { computed, markRaw, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { Check, ChevronLeft, ChevronRight, FileText, FolderOpen, Plus, Trash2, X } from "@lucide/vue";
import { fallbackLanguages, languageLabel } from "../domain/languages";
import {
  makeImportProfile,
  type ImportPreviewResponse,
  type ImportSideRequest,
  type KernelClient,
  type WorkspaceProject,
  type SegmentationMode,
  type SupportedLanguageDto,
} from "../domain/kernel-client";
import ImportSegmentationPreview from "./ImportSegmentationPreview.vue";
import { t, type LocalizedMessage } from "../i18n";
import { formatError, rawErrorMessage } from "../i18n/kernel-messages";

interface DocumentDraft {
  id: string;
  title: string;
  path: string;
  language: string;
  encoding: EncodingChoice;
  segmentation: SegmentationMode;
  preview: ImportPreviewResponse | null;
  previewKey: string;
  generation: number;
  loading: boolean;
  error: LocalizedMessage | null;
  generatedTitle: boolean;
}

const props = defineProps<{ kernelClient: KernelClient; beforeCreate: () => Promise<boolean> }>();
const busy = defineModel<boolean>("busy", { required: true });
const emit = defineEmits<{
  created: [snapshot: WorkspaceProject, projectPath: string];
  status: [message: LocalizedMessage];
}>();

const visible = ref(false);
const creating = ref(false);
const picking = ref(false);
const modal = ref<HTMLElement | null>(null);
const stepHeading = ref<HTMLElement | null>(null);
const projectNameInput = ref<HTMLInputElement | null>(null);
const projectName = ref("");
const projectDirectory = ref("");
const documents = ref<DocumentDraft[]>([]);
const currentStep = ref("project");
const supportedLanguages = ref<SupportedLanguageDto[]>(fallbackLanguages);
const wizardError = ref<LocalizedMessage | null>(null);
const errorText = (error: LocalizedMessage | null) => typeof error === "function" ? error() : error;
const wizardErrorElement = ref<HTMLElement | null>(null);
let session = 0;
let draftNumber = 0;
let previousFocus: HTMLElement | null = null;

watch(wizardError, async (message) => {
  if (!message) return;
  await nextTick();
  if (visible.value && wizardError.value === message) wizardErrorElement.value?.focus();
});

const displayLanguageLabel = (id: string) => languageLabel(supportedLanguages.value, id);
const segmentationLabels = computed<Record<SegmentationMode, string>>(() => ({
  non_empty_line: t("npSegNonEmpty"),
  sentence_rules: t("npSegSentences"),
  legacy_tagged_line: t("npSegLegacy"),
}));
const segmentationHelp = computed<Record<SegmentationMode, string>>(() => ({
  non_empty_line: t("npSegNonEmptyHelp"),
  sentence_rules: t("npSegSentencesHelp"),
  legacy_tagged_line: t("npSegLegacyHelp"),
}));

const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;
const documentRole = (index: number) => index === 0 ? t("npSource") : t("npTranslation", { p0: index });
const encodingChoiceLabel = (encoding: EncodingChoice) => encoding === "auto"
  ? t("npEncodingAuto")
  : encoding === "gb18030"
    ? t("npEncodingGbCompat")
    : encodingLabels[encoding];
const encodingOptions = ["auto", ...Object.keys(encodingLabels)] as EncodingChoice[];
const inputKey = (draft: DocumentDraft) => JSON.stringify([draft.path, draft.encoding, draft.segmentation]);
const documentReady = (draft: DocumentDraft) => !!draft.title.trim() && !draft.loading && !draft.error
  && !!draft.preview?.preview.segments.length && draft.previewKey === inputKey(draft);
const safeProjectName = computed(() => projectName.value.trim().replace(/[<>:"/\\|?*\u0000-\u001f]/g, "_").replace(/[. ]+$/, ""));
const projectReady = computed(() => !!safeProjectName.value && !!projectDirectory.value.trim());
const allDocumentsReady = computed(() => documents.value.length >= 2 && documents.value.every(documentReady));
const activeDocumentIndex = computed(() => documents.value.findIndex((draft) => draft.id === currentStep.value));
const activeDocument = computed(() => documents.value[activeDocumentIndex.value] ?? null);
const steps = computed(() => [
  { id: "project", label: t("npProjectInfo"), detail: t("npTranslationCount", { p0: documents.value.length - 1 }), ready: projectReady.value, removable: false },
  ...documents.value.map((draft, index) => ({
    id: draft.id,
    label: documentRole(index),
    detail: draft.title || t("npUntitled"),
    ready: documentReady(draft),
    removable: index > 0 && documents.value.length > 2,
  })),
  { id: "review", label: t("npReview"), detail: t("npReviewDetail"), ready: false, removable: false },
]);
const currentStepIndex = computed(() => steps.value.findIndex((step) => step.id === currentStep.value));
const canAdvance = computed(() => !creating.value && !picking.value && (currentStep.value === "project"
  ? projectReady.value : activeDocument.value ? documentReady(activeDocument.value) : false));
const createdPath = computed(() => {
  const directory = projectDirectory.value.trim();
  const separator = /^[A-Za-z]:\\|^\\\\/.test(directory) ? "\\" : "/";
  const baseName = safeProjectName.value.replace(/\.jm$/i, "");
  return `${directory.replace(/[\\/]+$/, "")}${separator}${baseName}.jm`;
});

function makeDocument(source: boolean): DocumentDraft {
  const index = source ? 0 : documents.value.length || 1;
  return {
    id: `import-draft-${++draftNumber}`,
    title: documentRole(index),
    path: "", language: source ? "zh" : "en", encoding: "auto", segmentation: "non_empty_line",
    preview: null, previewKey: "", generation: 0, loading: false, error: null, generatedTitle: true,
  };
}

async function focusStep() {
  await nextTick();
  if (!visible.value) return;
  if (currentStep.value === "project") projectNameInput.value?.focus();
  else stepHeading.value?.focus();
}

async function open() {
  if (visible.value || creating.value) return;
  const openingSession = ++session;
  previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  projectName.value = "";
  projectDirectory.value = "";
  documents.value = [];
  documents.value = [makeDocument(true), makeDocument(false)];
  currentStep.value = "project";
  wizardError.value = null;
  picking.value = false;
  supportedLanguages.value = fallbackLanguages;
  visible.value = true;
  void focusStep();
  try {
    const languages = await props.kernelClient.listSupportedLanguages();
    if (visible.value && session === openingSession && languages.length) supportedLanguages.value = languages;
  } catch {
    // The bundled language list remains available offline if the query fails.
  }
}

function invalidateSession() {
  session++;
  picking.value = false;
  for (const draft of documents.value) {
    draft.generation++;
    draft.loading = false;
  }
}

function close() {
  if (creating.value) return;
  visible.value = false;
  invalidateSession();
  if (previousFocus?.isConnected) previousFocus.focus();
}

function canVisit(index: number) {
  if (creating.value || picking.value) return false;
  return steps.value.slice(0, index).every((step) => step.ready);
}

function visitStep(id: string) {
  const index = steps.value.findIndex((step) => step.id === id);
  // Earlier pages remain reachable even if their drafts need attention.
  if (creating.value || picking.value || (index > currentStepIndex.value && !canVisit(index))) return;
  currentStep.value = id;
  wizardError.value = null;
  void focusStep();
}

function nextStep() {
  if (canAdvance.value) visitStep(steps.value[currentStepIndex.value + 1].id);
}

function addTranslation() {
  if (creating.value || picking.value) return;
  const draft = makeDocument(false);
  documents.value.push(draft);
  wizardError.value = null;
  // Appending a translation adds an import page immediately before confirmation.
  if (currentStep.value === "review") visitStep(draft.id);
}

function removeTranslation(id: string) {
  if (creating.value || picking.value || documents.value.length <= 2) return;
  const index = documents.value.findIndex((draft) => draft.id === id);
  if (index <= 0) return;
  documents.value[index].generation++;
  documents.value.splice(index, 1);
  // Keep untouched default names consistent with the remaining step numbers.
  for (let nextIndex = index; nextIndex < documents.value.length; nextIndex++) {
    const draft = documents.value[nextIndex];
    if (draft.generatedTitle) draft.title = documentRole(nextIndex);
  }
  if (currentStep.value === id) currentStep.value = documents.value[index - 1].id;
  wizardError.value = null;
  // The clicked delete button no longer exists, so restore focus within the current page.
  void focusStep();
}

async function chooseProjectDirectory() {
  if (picking.value || creating.value) return;
  const choosingSession = session;
  picking.value = true;
  wizardError.value = null;
  try {
    const selected = await openFileDialog({ directory: true, multiple: false, title: t("npChooseProjectDirDialog") });
    if (visible.value && choosingSession === session && typeof selected === "string") projectDirectory.value = selected;
  } catch (error) {
    const detail = rawErrorMessage(error);
    if (visible.value && choosingSession === session) wizardError.value = () => t("npProjectDirError", { p0: formatError(detail) });
  } finally {
    if (choosingSession === session) picking.value = false;
  }
}

async function chooseTextFile(draft: DocumentDraft) {
  if (picking.value || creating.value) return;
  const choosingSession = session;
  picking.value = true;
  wizardError.value = null;
  try {
    const selected = await openFileDialog({
      directory: false, multiple: false, title: t("npChooseTextDialog", { p0: documentRole(documents.value.indexOf(draft)) }),
      filters: [{ name: t("npTxtFilter"), extensions: ["txt"] }],
    });
    if (!visible.value || choosingSession !== session || typeof selected !== "string" || !documents.value.includes(draft)) return;
    const previousName = fileName(draft.path).replace(/\.txt$/i, "");
    if (!draft.title || draft.title === previousName || draft.generatedTitle) {
      draft.title = fileName(selected).replace(/\.txt$/i, "");
      draft.generatedTitle = false;
    }
    draft.path = selected;
    void previewFile(draft);
  } catch (error) {
    const detail = rawErrorMessage(error);
    if (visible.value && choosingSession === session) wizardError.value = () => t("npChooseTextError", { p0: formatError(detail) });
  } finally {
    if (choosingSession === session) picking.value = false;
  }
}

async function previewFile(draft: DocumentDraft) {
  if (creating.value) return;
  const generation = ++draft.generation;
  const previewSession = session;
  const key = inputKey(draft);
  draft.preview = null;
  draft.previewKey = "";
  draft.error = null;
  draft.loading = false;
  wizardError.value = null;
  if (!draft.path) return;
  draft.loading = true;
  const isCurrent = () => visible.value && session === previewSession && documents.value.includes(draft)
    && draft.generation === generation && inputKey(draft) === key;
  try {
    const preview = await props.kernelClient.previewImport({
      input: { kind: "file", path: draft.path },
      auto_detect_encoding: draft.encoding === "auto",
      profile: makeImportProfile(draft.encoding === "auto" ? "utf8" : draft.encoding, draft.segmentation),
    });
    if (!isCurrent()) return;
    draft.preview = markRaw(preview);
    draft.previewKey = key;
  } catch (error) {
    const detail = rawErrorMessage(error);
    if (isCurrent()) draft.error = () => t("npPreviewFailed", { p0: formatError(detail) });
  } finally {
    if (isCurrent()) draft.loading = false;
  }
}

function importRequest(draft: DocumentDraft): ImportSideRequest {
  return {
    language_id: draft.language,
    title: draft.title.trim(),
    input: { kind: "file", path: draft.path },
    profile: { ...draft.preview!.profile },
    expected_sha256: draft.preview!.sha256,
  };
}

async function createProject() {
  if (creating.value || picking.value || currentStep.value !== "review") return;
  if (!projectReady.value || !allDocumentsReady.value) {
    wizardError.value = () => t("npRequired");
    return;
  }
  const path = createdPath.value;
  const [source, target, ...additionalTargets] = documents.value.map(importRequest);
  const request = { project_path: path, name: projectName.value.trim(), source, target, additional_targets: additionalTargets };
  creating.value = true;
  busy.value = true;
  wizardError.value = null;
  try {
    if (!await props.beforeCreate()) return;
    const snapshot = await props.kernelClient.createProject(request);
    visible.value = false;
    invalidateSession();
    emit("created", snapshot, path);
    if (previousFocus?.isConnected) previousFocus.focus();
  } catch (error) {
    const detail = rawErrorMessage(error);
    wizardError.value = () => t("npCreateFailed", { p0: formatError(detail) });
    emit("status", () => t("npCreateFailed", { p0: formatError(detail) }));
  } finally {
    creating.value = false;
    busy.value = false;
  }
}

function handleKeydown(event: KeyboardEvent) {
  // Keep application shortcuts from acting on the workspace behind this dialog.
  event.stopPropagation();
  if (event.key === "Escape") {
    if ((event.target as HTMLElement | null)?.tagName === "SELECT") return;
    event.preventDefault();
    close();
  }
  if (event.key !== "Tab") return;
  const focusable = [...(modal.value?.querySelectorAll<HTMLElement>(
    'button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex="0"]',
  ) ?? [])].filter((element) => element.getClientRects().length > 0);
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (!first) { event.preventDefault(); modal.value?.focus(); return; }
  if (event.shiftKey && (document.activeElement === first || !focusable.includes(document.activeElement as HTMLElement))) {
    event.preventDefault(); last.focus();
  } else if (!event.shiftKey && (document.activeElement === last || !focusable.includes(document.activeElement as HTMLElement))) {
    event.preventDefault(); first.focus();
  }
}

onBeforeUnmount(invalidateSession);
defineExpose({ open });
</script>

<template>
  <div v-if="visible" class="modal-backdrop" @click.self="close">
    <section ref="modal" class="import-modal" role="dialog" aria-modal="true" aria-labelledby="import-title" tabindex="-1" @keydown="handleKeydown">
      <header class="wizard-header">
        <div><span class="eyebrow">{{ t("newProject") }}</span><h2 id="import-title">{{ t("npWizardTitle") }}</h2></div>
        <button class="icon-button" type="button" :aria-label="t('npCloseWizard')" :disabled="creating" @click="close"><X :size="19" /></button>
      </header>
      <div class="wizard-body">
        <nav class="wizard-navigation" :aria-label="t('npStepsAria')">
          <ol>
            <li v-for="(step, index) in steps" :key="step.id">
              <button class="step-link" type="button" :aria-current="currentStep === step.id ? 'step' : undefined" :disabled="creating || picking || (index > currentStepIndex && !canVisit(index))" @click="visitStep(step.id)">
                <span class="step-number"><Check v-if="step.ready && currentStep !== step.id" :size="15" /><template v-else>{{ index + 1 }}</template></span>
                <span class="step-text"><strong>{{ step.label }}</strong><small>{{ step.detail }}</small></span>
              </button>
              <button v-if="step.removable" class="icon-button remove-step" type="button" :aria-label="t('npDeleteStep', { p0: step.label })" :title="t('npDeleteStep', { p0: step.label })" :disabled="creating || picking" @click="removeTranslation(step.id)"><Trash2 :size="15" aria-hidden="true" /></button>
            </li>
          </ol>
          <button class="add-translation" type="button" :disabled="creating || picking" @click="addTranslation"><Plus :size="16" />{{ t("npAddTranslation") }}</button>
          <p>{{ t("npAddTranslationHelp") }}</p>
        </nav>
        <main class="wizard-content">
          <section v-if="currentStep === 'project'" class="wizard-page">
            <div class="step-intro"><span class="step-caption">{{ t("npStepProgress", { p0: 1, p1: steps.length }) }}</span><h3 ref="stepHeading" tabindex="-1">{{ t("npSetupProjectInfo") }}</h3><p>{{ t("npSetupProjectDescription") }}</p></div>
            <label class="field-label">{{ t("npProjectName") }}<input ref="projectNameInput" v-model="projectName" :placeholder="t('npProjectNamePlaceholder')" autocomplete="off" :disabled="creating" @input="wizardError = null" /></label>
            <label class="field-label">{{ t("npSaveLocation") }}<span class="compact-picker"><FolderOpen :size="17" aria-hidden="true" /><input :value="projectDirectory" readonly :placeholder="t('npChooseParentFolder')" /><button type="button" :disabled="picking || creating" @click="chooseProjectDirectory">{{ t("npBrowse") }}</button></span></label>
            <p v-if="projectReady" class="save-path">{{ t("npSavePath", { p0: createdPath }) }}</p>
            <div class="document-plan"><h4>{{ t("npImportPlan") }}</h4><div v-for="(draft, index) in documents" :key="draft.id" class="planned-document"><FileText :size="17" aria-hidden="true" /><strong>{{ documentRole(index) }}</strong><span>{{ draft.path ? fileName(draft.path) : t("npChooseTextLater") }}</span><button v-if="index > 0 && documents.length > 2" class="icon-button" type="button" :aria-label="t('npRemoveDocument', { p0: documentRole(index) })" :disabled="picking || creating" @click="removeTranslation(draft.id)"><Trash2 :size="15" /></button></div></div>
          </section>
          <section v-else-if="activeDocument" :key="activeDocument.id" class="wizard-page">
            <div class="step-intro"><span class="step-caption">{{ t("npStepProgress", { p0: currentStepIndex + 1, p1: steps.length }) }}</span><h3 ref="stepHeading" tabindex="-1">{{ t("npImportDocument", { p0: documentRole(activeDocumentIndex) }) }}</h3><p>{{ t("npImportDescription") }}</p></div>
            <div class="document-fields"><label class="field-label">{{ t(activeDocumentIndex === 0 ? "npSourceName" : "npTranslationName") }}<input v-model="activeDocument.title" :placeholder="t('npDocumentNamePlaceholder')" :disabled="creating" @input="activeDocument.generatedTitle = false" /></label><label class="field-label">{{ t("npTextLanguage") }}<select :aria-label="t('npTextLanguage')" v-model="activeDocument.language" :disabled="creating"><option v-for="language in supportedLanguages" :key="language.language_id" :value="language.language_id">{{ language.native_name }}</option></select></label></div>
            <label class="field-label">{{ t("npTxtFile") }}<span class="compact-picker"><FileText :size="17" aria-hidden="true" /><input :value="activeDocument.path" readonly :placeholder="t('npChooseTxt')" /><button type="button" :disabled="picking || creating" @click="chooseTextFile(activeDocument)">{{ t(activeDocument.path ? "npChangeFile" : "npChooseFile") }}</button></span></label>
            <div class="import-settings"><label>{{ t("npSegmentationMode") }}<select :aria-label="t('npSegmentationMode')" aria-describedby="segmentation-help" v-model="activeDocument.segmentation" :disabled="creating" @change="previewFile(activeDocument)"><option v-for="(label, value) in segmentationLabels" :key="value" :value="value">{{ label }}</option></select></label><label>{{ t("npEncoding") }}<select :aria-label="t('npEncoding')" v-model="activeDocument.encoding" :disabled="creating" @change="previewFile(activeDocument)"><option v-for="value in encodingOptions" :key="value" :value="value">{{ encodingChoiceLabel(value) }}</option></select></label><button class="text-button" type="button" :disabled="!activeDocument.path || activeDocument.loading || creating || picking" @click="previewFile(activeDocument)">{{ t("npRefreshPreview") }}</button></div>
            <p id="segmentation-help" class="segmentation-help">{{ segmentationHelp[activeDocument.segmentation] }}</p>
            <ImportSegmentationPreview :preview="activeDocument.preview" :loading="activeDocument.loading" :error="errorText(activeDocument.error)" @retry="previewFile(activeDocument)" />
            <button v-if="activeDocumentIndex > 0 && documents.length > 2" class="text-button remove-document" type="button" :disabled="creating || picking" @click="removeTranslation(activeDocument.id)"><Trash2 :size="14" />{{ t("npRemoveTranslation") }}</button>
          </section>
          <section v-else class="wizard-page">
            <div class="step-intro"><span class="step-caption">{{ t("npStepProgress", { p0: steps.length, p1: steps.length }) }}</span><h3 ref="stepHeading" tabindex="-1">{{ t("npConfirmTitle") }}</h3><p>{{ t("npConfirmDescription", { p0: documents.length - 1 }) }}</p></div>
            <div class="project-summary"><strong>{{ projectName }}</strong><span>{{ createdPath }}</span></div>
            <article v-for="(draft, index) in documents" :key="draft.id" class="document-summary"><div class="document-summary-heading"><span>{{ documentRole(index) }}</span><h4>{{ draft.title }}</h4><button class="text-button" type="button" :disabled="creating" :aria-label="t('npEditDocument', { p0: documentRole(index) })" @click="visitStep(draft.id)">{{ t("npEdit") }}</button></div><p>{{ fileName(draft.path) }}</p><ul><li>{{ displayLanguageLabel(draft.language) }}</li><li>{{ draft.encoding === "auto" ? t("npAutoDetectedPrefix") : "" }}{{ draft.preview ? encodingLabels[draft.preview.profile.encoding] : encodingChoiceLabel(draft.encoding) }}</li><li>{{ segmentationLabels[draft.segmentation] }}</li><li class="segment-count"><Check :size="14" aria-hidden="true" />{{ t("npSegmentCount", { p0: draft.preview?.preview.segments.length ?? 0 }) }}</li></ul></article>
            <p class="creation-note">{{ t("npCreationNote") }}</p>
          </section>
          <p v-if="wizardError" ref="wizardErrorElement" class="wizard-error" role="alert" tabindex="-1">{{ errorText(wizardError) }}</p>
        </main>
      </div>
      <footer class="wizard-footer">
        <span class="footer-progress" role="status">{{ creating ? t("npCreatingProject") : t("npStepProgress", { p0: currentStepIndex + 1, p1: steps.length }) }}</span>
        <button class="secondary-button" type="button" :disabled="creating" @click="close">{{ t("npCancel") }}</button>
        <button v-if="currentStepIndex > 0" class="secondary-button" type="button" :disabled="creating || picking" @click="visitStep(steps[currentStepIndex - 1].id)"><ChevronLeft :size="15" />{{ t("npPrevious") }}</button>
        <button v-if="currentStep !== 'review'" class="primary-button" type="button" :disabled="!canAdvance" @click="nextStep">{{ t("npNext") }}<ChevronRight :size="15" /></button>
        <button v-else class="primary-button" type="button" :disabled="creating || picking || !projectReady || !allDocumentsReady" @click="createProject"><Check :size="15" />{{ creating ? t("npCreating") : t("npCreateOpen") }}</button>
      </footer>
    </section>
  </div>
</template>

<style scoped>
.modal-backdrop { position: fixed; z-index: 20; inset: 0; display: grid; place-items: center; padding: 24px; background: rgb(31 42 34 / 22%); }
.import-modal { display: flex; flex-direction: column; width: min(1040px, 100%); height: min(820px, calc(100dvh - 48px)); max-height: 100%; min-height: 0; overflow: hidden; border: 1px solid var(--line); border-radius: 12px; color: var(--ink-900); background: var(--surface-raised); box-shadow: 0 22px 70px rgb(29 48 32 / 23%); }
.wizard-header, .wizard-footer { flex: none; display: flex; align-items: center; gap: 10px; justify-content: space-between; padding: 19px 24px; }
.wizard-header { border-bottom: 1px solid var(--line); }.wizard-header h2 { margin: 5px 0 0; }.eyebrow { color: var(--green-700); font-size: var(--jm-font-size-subheadline); line-height: var(--jm-line-height-subheadline); }
.icon-button { display: inline-flex; flex: none; align-items: center; justify-content: center; width: 34px; height: 34px; padding: 0; border: 0; border-radius: 6px; color: var(--ink-500); background: transparent; cursor: pointer; }.icon-button:hover:not(:disabled) { color: var(--ink-900); background: var(--surface-hover); }
.wizard-body { flex: 1; display: grid; grid-template-columns: 210px minmax(0, 1fr); min-height: 0; }
.wizard-navigation { display: flex; flex-direction: column; gap: 14px; padding: 20px 14px; border-right: 1px solid var(--line); overflow: auto; background: var(--surface-subtle); }.wizard-navigation ol { display: grid; gap: 5px; padding: 0; margin: 0; list-style: none; }.wizard-navigation .step-link { display: flex; align-items: center; gap: 11px; flex: 1; min-width: 0; min-height: 58px; padding: 9px 10px; text-align: left; border: 1px solid transparent; border-radius: 8px; background: transparent; cursor: pointer; }.wizard-navigation .step-link[aria-current="step"] { border-color: var(--line); color: var(--green-900); background: var(--surface-green-selected); }.wizard-navigation .step-link:hover:not(:disabled) { background: var(--surface-hover); }.wizard-navigation .step-link:disabled { opacity: .6; }
.wizard-navigation li { display: flex; align-items: center; gap: 2px; min-width: 0; }
.wizard-navigation .remove-step { width: 30px; height: 34px; }
.wizard-navigation .remove-step:hover:not(:disabled) { background: var(--surface-danger-soft); }
.step-number { display: inline-flex; flex: none; align-items: center; justify-content: center; width: 25px; height: 25px; border: 1px solid var(--line); border-radius: 50%; color: var(--ink-500); background: var(--surface-raised); font-size: var(--jm-font-size-callout); }[aria-current="step"] .step-number { border-color: var(--green-700); color: var(--green-900); }.step-text { display: grid; min-width: 0; gap: 5px; }.step-text strong { font-size: var(--jm-font-size-body); }.step-text small { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; color: var(--ink-500); }
.add-translation { display: flex; align-items: center; justify-content: center; flex: none; gap: 7px; min-height: 38px; padding: 8px; border: 1px solid var(--line); border-radius: 7px; color: var(--green-900); background: var(--surface-raised); cursor: pointer; }.wizard-navigation > p { margin: 0; padding: 0 7px; color: var(--ink-500); font-size: var(--jm-font-size-callout); line-height: 1.7; }
.wizard-content { min-width: 0; overflow: auto; overscroll-behavior: contain; padding: 25px 28px; }.wizard-page { display: flex; flex-direction: column; gap: 17px; min-width: 0; }.step-intro { margin-bottom: 3px; }.step-caption { color: var(--ink-500); font-size: var(--jm-font-size-callout); }.step-intro h3 { margin: 8px 0; font-size: var(--jm-font-size-title-2); line-height: var(--jm-line-height-title-2); }.step-intro h3:focus { outline: none; }.step-intro p { margin: 0; color: var(--ink-700); line-height: 1.7; }
.field-label { display: grid; gap: 8px; min-width: 0; color: var(--ink-700); }.field-label > input, .compact-picker { min-width: 0; height: 38px; border: 1px solid var(--line); border-radius: 7px; color: var(--ink-900); background: var(--surface-input); }.field-label > input { width: 100%; padding: 0 11px; }.compact-picker { display: flex; align-items: center; gap: 8px; padding-left: 11px; }.compact-picker > svg { flex: none; color: var(--ink-500); }.compact-picker input { flex: 1; min-width: 0; width: 100%; height: 100%; padding: 0; border: 0; border-radius: 0; color: var(--ink-900); background: transparent; }.compact-picker button { align-self: stretch; flex: none; padding: 0 13px; border: 0; border-left: 1px solid var(--line); border-radius: 0 6px 6px 0; color: var(--green-900); background: var(--surface-subtle); cursor: pointer; }.save-path { margin: -5px 0 0; color: var(--ink-500); font-size: var(--jm-font-size-callout); line-height: 1.6; }.save-path span { overflow-wrap: anywhere; }
.document-plan { margin-top: 5px; padding: 17px; border: 1px solid var(--line); border-radius: 8px; background: var(--surface-subtle); }.document-plan h4 { margin: 0 0 10px; font-size: var(--jm-font-size-body); }.planned-document { display: flex; align-items: center; gap: 10px; min-height: 43px; color: var(--ink-700); }.planned-document > svg { flex: none; color: var(--green-700); }.planned-document strong { flex: none; min-width: 50px; }.planned-document span { flex: 1; min-width: 0; overflow-wrap: anywhere; color: var(--ink-500); font-size: var(--jm-font-size-callout); }
.document-fields { display: grid; grid-template-columns: minmax(0, 1fr) minmax(160px, .7fr); gap: 16px; }.document-fields select { width: 100%; }.import-settings { display: flex; flex-wrap: wrap; align-items: center; gap: 12px 20px; }.import-settings label { display: flex; align-items: center; gap: 8px; color: var(--ink-700); }.import-settings > button { margin-left: auto; }.segmentation-help { margin: -6px 0 0; color: var(--ink-500); font-size: var(--jm-font-size-callout); line-height: 1.7; }
.text-button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; min-height: 30px; padding: 4px 5px; border: 0; border-radius: 4px; color: var(--green-900); background: transparent; cursor: pointer; }.text-button:hover:not(:disabled) { background: var(--surface-hover); }.remove-document { align-self: flex-start; color: var(--ink-500); }
.project-summary { display: grid; gap: 9px; padding: 17px; border-radius: 8px; background: var(--surface-green-soft); }.project-summary strong { color: var(--green-900); font-size: var(--jm-font-size-title-3); }.project-summary span { color: var(--ink-700); overflow-wrap: anywhere; line-height: 1.6; }.document-summary { padding: 13px 16px; border: 1px solid var(--line); border-radius: 8px; }.document-summary-heading { display: flex; align-items: center; gap: 10px; }.document-summary-heading > span { flex: none; padding: 4px 6px; border-radius: 4px; color: var(--ink-700); background: var(--surface-subtle); font-size: var(--jm-font-size-callout); }.document-summary h4 { margin: 0; overflow-wrap: anywhere; font-size: var(--jm-font-size-body); }.document-summary button { margin-left: auto; flex: none; }.document-summary p { margin: 6px 0 11px; overflow-wrap: anywhere; color: var(--ink-500); font-size: var(--jm-font-size-callout); }.document-summary ul { display: flex; flex-wrap: wrap; align-items: center; gap: 9px 16px; margin: 0; padding: 0; list-style: none; color: var(--ink-700); font-size: var(--jm-font-size-callout); }.segment-count { display: inline-flex; align-items: center; gap: 5px; color: var(--green-900); }.creation-note { margin: 0; color: var(--ink-500); line-height: 1.7; }
.wizard-error { margin: 18px 0 0; padding: 12px 14px; border: 1px solid var(--line); border-radius: 7px; color: var(--ink-900); background: var(--surface-danger-soft); overflow-wrap: anywhere; line-height: 1.65; }
.wizard-footer { justify-content: flex-end; border-top: 1px solid var(--line); }.footer-progress { margin-right: auto; color: var(--ink-500); font-size: var(--jm-font-size-callout); }.primary-button, .secondary-button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; min-height: 38px; padding: 0 16px; border-radius: 7px; cursor: pointer; }.primary-button { border: 1px solid var(--green-900); color: var(--paper); background: var(--green-900); }.secondary-button { border: 1px solid var(--line); color: var(--ink-700); background: var(--surface-raised); }
@media (max-width: 760px) { .modal-backdrop { padding: 12px; }.import-modal { height: calc(100dvh - 24px); }.wizard-body { grid-template-columns: 155px minmax(0, 1fr); }.wizard-navigation { padding: 14px 7px; }.wizard-navigation .step-link { gap: 7px; padding: 8px 6px; }.wizard-content { padding: 20px 16px; }.document-fields { grid-template-columns: minmax(0, 1fr); }.wizard-header, .wizard-footer { padding: 15px 16px; }.import-settings > button { margin-left: 0; } }
@media (max-width: 540px) { .wizard-body { grid-template-columns: minmax(0, 1fr); grid-template-rows: auto minmax(0, 1fr); }.wizard-navigation { display: block; max-height: 150px; border-right: 0; border-bottom: 1px solid var(--line); }.wizard-navigation ol { display: flex; gap: 6px; overflow-x: auto; }.wizard-navigation li { flex: none; }.wizard-navigation .step-text small, .wizard-navigation > p { display: none; }.wizard-navigation .step-link { min-height: 40px; }.add-translation { margin: 8px 6px 0; min-height: 32px; }.wizard-footer { gap: 6px; flex-wrap: wrap; }.primary-button, .secondary-button { padding-inline: 10px; }.footer-progress { font-size: var(--jm-font-size-subheadline); } }
</style>
