<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Check, CornerUpLeft, MessageSquareText, Pencil, Plus, Save, Trash2, X } from "@lucide/vue";
import { newCommandId, type CommandContext, type CommandScope } from "../domain/kernel-client";
import { t } from "../i18n";
import type { PanelMessageKey } from "../i18n/panel-messages";

defineOptions({ name: "AnnotationPanel" });

export type AnnotationStatus = "draft" | "in_progress" | "resolved";
export type AnnotationFilter = "all" | AnnotationStatus;

export interface AnnotationLink {
  side: "source" | "target";
  segmentId: string;
  label?: string;
  text?: string;
}

export interface AnnotationItem {
  id: string;
  number: number;
  status: AnnotationStatus;
  title: string;
  body: string;
  createdAt: string;
  links: AnnotationLink[];
}

export interface AnnotationDraft {
  commandContext?: CommandContext;
  title: string;
  body: string;
  status: AnnotationStatus;
  links: AnnotationLink[];
}

const props = withDefaults(
  defineProps<{
    annotations: AnnotationItem[];
    activeFilter?: AnnotationFilter;
    selectedId?: string | null;
    readonly?: boolean;
    scopeKey?: string;
    commandScope?: CommandScope | null;
  }>(),
  { activeFilter: "all", selectedId: null, readonly: false, scopeKey: "unbound" },
);

const emit = defineEmits<{
  "update:activeFilter": [filter: AnnotationFilter];
  select: [annotationId: string];
  "open-link": [segmentId: string];
  create: [draft: AnnotationDraft, done: (saved: boolean) => void];
  edit: [annotationId: string, draft: AnnotationDraft, done: (saved: boolean) => void];
  delete: [annotationId: string];
  resolve: [annotationId: string];
  close: [];
}>();
const panelT: (key: PanelMessageKey, params?: Record<string, unknown>) => string = t;

const creating = ref(false);
const saving = ref(false);
const editingId = ref<string | null>(null);
const draft = ref<AnnotationDraft>({ title: "", body: "", status: "draft", links: [] });
// The global panel survives route changes; drafts remain isolated by project.
const projectDrafts = new Map<string, { creating: boolean; editingId: string | null; draft: AnnotationDraft }>();
watch(() => props.scopeKey, (next, previous) => {
  projectDrafts.set(previous, { creating: creating.value, editingId: editingId.value, draft: { ...draft.value, links: draft.value.links.map(link => ({ ...link })) } });
  const saved = projectDrafts.get(next);
  creating.value = saved?.creating ?? false;
  editingId.value = saved?.editingId ?? null;
  draft.value = saved ? { ...saved.draft, links: saved.draft.links.map(link => ({ ...link })) } : { title: "", body: "", status: "draft", links: [] };
}, { flush: "sync" });

const statusMessageKeys: Record<AnnotationStatus, PanelMessageKey> = {
  draft: "annotationDraft",
  in_progress: "annotationInProgress",
  resolved: "annotationResolvedStatus",
};
const statusLabel = (status: AnnotationStatus) => panelT(statusMessageKeys[status]);
const counts = computed(() => ({
  all: props.annotations.length,
  draft: props.annotations.filter((item) => item.status === "draft").length,
  in_progress: props.annotations.filter((item) => item.status === "in_progress").length,
  resolved: props.annotations.filter((item) => item.status === "resolved").length,
}));
const filtered = computed(() => props.activeFilter === "all" ? props.annotations : props.annotations.filter((item) => item.status === props.activeFilter));
const statusClass = (status: AnnotationStatus) => `annotation-status--${status}`;
const setFilter = (filter: AnnotationFilter) => emit("update:activeFilter", filter);
const startCreate = () => { creating.value = true; editingId.value = null; draft.value = { commandContext: props.commandScope ? { ...props.commandScope } : undefined, title: "", body: "", status: "draft", links: [] }; };
const cancelCreate = () => { creating.value = false; };
let pendingSubmission: { fingerprint: string; commandId: string } | null = null;
const submit = (annotationId: string | null) => {
  if (props.readonly || saving.value || !draft.value.title.trim() || !draft.value.body.trim()) return;
  const current = draft.value;
  const value = { ...current, title: current.title.trim(), body: current.body.trim() };
  const fingerprint = JSON.stringify({ annotationId, ...value });
  if (pendingSubmission?.fingerprint !== fingerprint) pendingSubmission = { fingerprint, commandId: newCommandId() };
  if (value.commandContext) value.commandContext = { ...value.commandContext, command_id: pendingSubmission.commandId };
  saving.value = true;
  const done = (saved: boolean) => {
    saving.value = false;
    if (!saved || draft.value !== current || JSON.stringify({ annotationId, ...current, title: current.title.trim(), body: current.body.trim() }) !== fingerprint) return;
    creating.value = false;
    editingId.value = null;
    pendingSubmission = null;
  };
  if (annotationId) emit("edit", annotationId, value, done);
  else emit("create", value, done);
};
const submitCreate = () => submit(null);
const startEdit = (annotation: AnnotationItem) => {
  editingId.value = annotation.id;
  creating.value = false;
  draft.value = { commandContext: props.commandScope ? { ...props.commandScope } : undefined, title: annotation.title, body: annotation.body, status: annotation.status, links: [...annotation.links] };
};
const cancelEdit = () => { editingId.value = null; };
const submitEdit = (annotationId: string) => submit(annotationId);
</script>

<template>
  <aside class="annotation-workspace" :aria-label="panelT('annotation')">
    <header class="annotation-heading"><div><span class="annotation-eyebrow">{{ panelT('annotationsEyebrow') }}</span><h2>{{ panelT('annotationsTitle') }} <small>{{ annotations.length }}</small></h2></div><button type="button" :title="panelT('closeAnnotations')" @click="emit('close')"><X :size="18" /></button></header>
    <nav class="annotation-filter" :aria-label="panelT('annotationFilter')">
      <button v-for="filter in (['all', 'draft', 'in_progress', 'resolved'] as AnnotationFilter[])" :key="filter" :class="{ active: activeFilter === filter }" type="button" @click="setFilter(filter)">{{ filter === "all" ? panelT('annotationFilterAll') : statusLabel(filter) }} <span>{{ counts[filter] }}</span></button>
    </nav>

    <form v-if="creating" class="annotation-editor annotation-editor--new" @submit.prevent="submitCreate">
      <div class="annotation-editor-title"><MessageSquareText :size="16" /><strong>{{ panelT('newAnnotation') }}</strong><button type="button" :title="panelT('cancelCreate')" @click="cancelCreate"><X :size="15" /></button></div>
      <label>{{ panelT('title') }}<input v-model="draft.title" :disabled="saving" :placeholder="panelT('annotationTitlePlaceholder')" required /></label><label>{{ panelT('content') }}<textarea v-model="draft.body" :disabled="saving" rows="4" :placeholder="panelT('annotationBodyPlaceholder')" required></textarea></label><label>{{ panelT('status') }}<select v-model="draft.status" :disabled="saving"><option value="draft">{{ statusLabel('draft') }}</option><option value="in_progress">{{ statusLabel('in_progress') }}</option><option value="resolved">{{ statusLabel('resolved') }}</option></select></label><footer><button class="annotation-secondary" type="button" @click="cancelCreate">{{ panelT('cancel') }}</button><button class="annotation-primary" :disabled="saving" type="submit"><Save :size="14" />{{ panelT('create') }}</button></footer>
    </form>

    <div class="annotation-list">
      <article v-for="annotation in filtered" :key="annotation.id" class="annotation-card" :class="[{ 'annotation-card--selected': annotation.id === selectedId }, statusClass(annotation.status)]" @click="emit('select', annotation.id)">
        <div class="annotation-number" :class="statusClass(annotation.status)">{{ annotation.number }}</div>
        <div class="annotation-card-body">
          <div class="annotation-state"><span class="annotation-state-pill" :class="statusClass(annotation.status)">{{ statusLabel(annotation.status) }}</span><time>{{ annotation.createdAt }}</time></div>
          <template v-if="editingId === annotation.id">
            <div class="annotation-editor"><label>{{ panelT('title') }}<input v-model="draft.title" :disabled="saving" /></label><label>{{ panelT('content') }}<textarea v-model="draft.body" :disabled="saving" rows="4"></textarea></label><label>{{ panelT('status') }}<select v-model="draft.status" :disabled="saving"><option value="draft">{{ statusLabel('draft') }}</option><option value="in_progress">{{ statusLabel('in_progress') }}</option><option value="resolved">{{ statusLabel('resolved') }}</option></select></label><footer><button class="annotation-secondary" type="button" @click.stop="cancelEdit">{{ panelT('cancel') }}</button><button class="annotation-primary" :disabled="saving" type="button" @click.stop="submitEdit(annotation.id)"><Save :size="14" />{{ panelT('save') }}</button></footer></div>
          </template>
          <template v-else>
            <h3>{{ annotation.title }}</h3><p>{{ annotation.body }}</p>
            <div v-if="annotation.links.length" class="annotation-links"><button v-for="link in annotation.links" :key="`${annotation.id}-${link.side}-${link.segmentId}`" type="button" :title="panelT('returnTo', { p0: link.label ?? link.segmentId })" @click.stop="emit('open-link', link.segmentId)"><span>{{ panelT(link.side === 'source' ? 'sourceHeading' : 'targetHeading') }} <b :title="link.segmentId">{{ link.label ?? link.segmentId }}</b><small v-if="link.text">{{ link.text }}</small></span><CornerUpLeft :size="14" /></button></div>
            <footer class="annotation-card-actions"><button type="button" @click.stop="startEdit(annotation)"><Pencil :size="13" />{{ panelT('edit') }}</button><button type="button" @click.stop="emit('delete', annotation.id)"><Trash2 :size="13" />{{ panelT('delete') }}</button><button v-if="annotation.status !== 'resolved'" class="annotation-resolve" type="button" @click.stop="emit('resolve', annotation.id)"><Check :size="13" />{{ panelT('markResolved') }}</button></footer>
          </template>
        </div>
      </article>
      <div v-if="!filtered.length" class="annotation-empty"><MessageSquareText :size="18" />{{ panelT('annotationsEmptyFilter') }}</div>
    </div>
    <button v-if="!readonly && !creating" class="annotation-new" type="button" @click="startCreate"><Plus :size="16" />{{ panelT('newAnnotation') }}</button>
  </aside>
</template>

<style scoped>
.annotation-workspace { display: flex; width: 100%; height: 100%; min-width: 0; min-height: 0; flex-direction: column; overflow: hidden; border: 1px solid var(--line); border-right: 0; border-radius: 0; color: var(--ink-900); background: var(--surface-subtle); box-shadow: var(--shadow-soft); }.annotation-heading { display: flex; align-items: flex-start; justify-content: space-between; padding: 19px 18px 12px; }.annotation-heading button { padding: 5px; border: 0; color: var(--ink-500); background: transparent; cursor: pointer; }.annotation-eyebrow { color: var(--green-700); font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-semibold); letter-spacing: .1em; line-height: var(--jm-line-height-subheadline); }.annotation-heading h2 { margin: 4px 0 0; font-size: var(--jm-font-size-title-3); line-height: var(--jm-line-height-title-3); font-weight: var(--jm-font-weight-semibold); }.annotation-heading h2 small { display: inline-block; margin-left: 3px; padding: 2px 6px; border-radius: 10px; color: var(--green-900); background: var(--green-100); font-size: var(--jm-font-size-subheadline); line-height: var(--jm-line-height-subheadline); }.annotation-filter { display: flex; gap: 4px; padding: 0 13px 13px; border-bottom: 1px solid var(--line); }.annotation-filter button { padding: 7px 8px; border: 1px solid transparent; border-radius: 5px; color: var(--ink-500); background: transparent; font-size: var(--jm-font-size-callout); cursor: pointer; line-height: var(--jm-line-height-callout); }.annotation-filter button.active { border-color: #b3d5b7; color: var(--green-900); background: var(--surface-green-selected); }.annotation-filter span { margin-left: 2px; color: var(--ink-500); }.annotation-list { min-height: 0; overflow: auto; padding-bottom: 4px; }.annotation-card { display: grid; grid-template-columns: 25px 1fr; gap: 7px; margin: 13px 13px 0; padding: 13px 11px; border: 1px solid #d3dde5; border-radius: 8px; background: var(--surface-raised); cursor: pointer; }.annotation-card--selected { outline: 2px solid rgb(47 129 67 / 20%); }.annotation-card--draft { border-color: #d3c4ec; }.annotation-card--resolved { opacity: .78; }.annotation-number { display: grid; place-items: center; width: 22px; height: 22px; border-radius: 6px; color: #fff; background: #9864d5; font-size: var(--jm-font-size-callout); line-height: var(--jm-line-height-callout); }.annotation-number.annotation-status--in_progress { background: #3c9a5a; }.annotation-number.annotation-status--resolved { background: #7d877f; }.annotation-state { display: flex; align-items: center; gap: 7px; color: var(--ink-900); font-size: var(--jm-font-size-callout); font-weight: var(--jm-font-weight-semibold); line-height: var(--jm-line-height-callout); }.annotation-state time { margin-left: auto; color: var(--ink-500); font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-regular); line-height: var(--jm-line-height-subheadline); }.annotation-state-pill { padding: 3px 6px; border-radius: 4px; color: #fff; background: #9864d5; }.annotation-state-pill.annotation-status--in_progress { background: #3c9a5a; }.annotation-state-pill.annotation-status--resolved { background: #7d877f; }.annotation-card h3 { margin: 12px 0 7px; font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); font-weight: var(--jm-font-weight-semibold); }.annotation-card p { margin: 0; color: var(--ink-700); font-size: var(--jm-font-size-body); line-height: 1.55; }.annotation-links { display: flex; flex-direction: column; gap: 5px; margin-top: 10px; padding: 8px; border-radius: 5px; background: var(--surface-muted); color: var(--ink-700); font-size: var(--jm-font-size-subheadline); line-height: var(--jm-line-height-subheadline); }.annotation-links b { float: right; color: var(--ink-500); font-weight: var(--jm-font-weight-medium); }.annotation-links small { display: block; color: var(--ink-700); }.annotation-card-actions { display: flex; flex-wrap: wrap; gap: 12px; margin-top: 10px; padding-top: 9px; border-top: 1px solid var(--line); }.annotation-card-actions button, .annotation-new { display: inline-flex; align-items: center; gap: 5px; border: 0; color: var(--ink-700); background: transparent; font-size: var(--jm-font-size-callout); cursor: pointer; line-height: var(--jm-line-height-callout); }.annotation-card-actions button:hover { color: var(--green-900); }.annotation-resolve { color: var(--green-900) !important; }.annotation-new { justify-content: center; margin: 13px; padding: 10px; border: 1px solid #abd1af; border-radius: 6px; color: var(--green-900); background: var(--surface-green-soft); }.annotation-editor { display: grid; gap: 8px; }.annotation-editor--new { margin: 13px; padding: 13px; border: 1px solid #abd1af; border-radius: 7px; background: var(--surface-green-soft); }.annotation-editor-title { display: flex; align-items: center; gap: 7px; color: var(--green-900); }.annotation-editor-title button { margin-left: auto; padding: 2px; border: 0; color: var(--ink-500); background: transparent; cursor: pointer; }.annotation-editor label { display: grid; gap: 4px; color: var(--ink-700); font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }.annotation-editor select { width: 100%; }
.annotation-editor input, .annotation-editor textarea { width: 100%; padding: 7px; border: 1px solid #c8d3c9; border-radius: 4px; color: var(--ink-900); background: var(--surface-input); font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }.annotation-editor textarea { resize: vertical; }.annotation-editor footer { display: flex; justify-content: flex-end; gap: 7px; margin-top: 2px; }.annotation-primary, .annotation-secondary { display: inline-flex; align-items: center; gap: 5px; padding: 7px 11px; border: 1px solid var(--line); border-radius: 5px; background: var(--surface-raised); cursor: pointer; }.annotation-primary { border-color: var(--green-700); color: #fff; background: var(--green-700); }.annotation-empty { display: flex; min-height: 90px; align-items: center; justify-content: center; gap: 7px; color: var(--ink-500); font-size: var(--jm-font-size-callout); line-height: var(--jm-line-height-callout); }
.annotation-links button { display: grid; grid-template-columns: minmax(0, 1fr) 18px; align-items: center; gap: 7px; padding: 5px; border: 0; border-radius: 5px; color: inherit; background: transparent; text-align: left; cursor: pointer; }
.annotation-links button:hover { color: var(--green-900); background: var(--surface-green-soft); }
.annotation-links button > svg { justify-self: end; }
.annotation-links button small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
