<script setup lang="ts">
import { computed, ref } from "vue";
import { Check, CornerUpLeft, MessageSquareText, Pencil, Plus, Save, Trash2, X } from "@lucide/vue";

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
  }>(),
  { activeFilter: "all", selectedId: null, readonly: false },
);

const emit = defineEmits<{
  "update:activeFilter": [filter: AnnotationFilter];
  select: [annotationId: string];
  "open-link": [segmentId: string];
  create: [draft: AnnotationDraft];
  edit: [annotationId: string, draft: AnnotationDraft];
  delete: [annotationId: string];
  resolve: [annotationId: string];
  close: [];
}>();

const creating = ref(false);
const editingId = ref<string | null>(null);
const draft = ref<AnnotationDraft>({ title: "", body: "", status: "draft", links: [] });

const statusLabels: Record<AnnotationStatus, { zh: string; en: string }> = {
  draft: { zh: "草稿", en: "Draft" },
  in_progress: { zh: "进行中", en: "In Progress" },
  resolved: { zh: "已解决", en: "Resolved" },
};
const counts = computed(() => ({
  all: props.annotations.length,
  draft: props.annotations.filter((item) => item.status === "draft").length,
  in_progress: props.annotations.filter((item) => item.status === "in_progress").length,
  resolved: props.annotations.filter((item) => item.status === "resolved").length,
}));
const filtered = computed(() => props.activeFilter === "all" ? props.annotations : props.annotations.filter((item) => item.status === props.activeFilter));
const statusClass = (status: AnnotationStatus) => `annotation-status--${status}`;
const setFilter = (filter: AnnotationFilter) => emit("update:activeFilter", filter);
const startCreate = () => { creating.value = true; editingId.value = null; draft.value = { title: "", body: "", status: "draft", links: [] }; };
const cancelCreate = () => { creating.value = false; };
const submitCreate = () => {
  if (!draft.value.title.trim() || !draft.value.body.trim()) return;
  emit("create", { ...draft.value, title: draft.value.title.trim(), body: draft.value.body.trim() });
  creating.value = false;
};
const startEdit = (annotation: AnnotationItem) => {
  editingId.value = annotation.id;
  creating.value = false;
  draft.value = { title: annotation.title, body: annotation.body, status: annotation.status, links: [...annotation.links] };
};
const cancelEdit = () => { editingId.value = null; };
const submitEdit = (annotationId: string) => {
  if (!draft.value.title.trim() || !draft.value.body.trim()) return;
  emit("edit", annotationId, { ...draft.value, title: draft.value.title.trim(), body: draft.value.body.trim() });
  editingId.value = null;
};
</script>

<template>
  <aside class="annotation-workspace" aria-label="批注">
    <header class="annotation-heading"><div><span class="annotation-eyebrow">ANNOTATIONS</span><h2>批注 / Annotations <small>{{ annotations.length }}</small></h2></div><button type="button" title="关闭批注面板" @click="emit('close')"><X :size="18" /></button></header>
    <nav class="annotation-filter" aria-label="批注状态筛选">
      <button v-for="filter in (['all', 'draft', 'in_progress', 'resolved'] as AnnotationFilter[])" :key="filter" :class="{ active: activeFilter === filter }" type="button" @click="setFilter(filter)">{{ filter === "all" ? "全部" : statusLabels[filter].zh }} <span>{{ counts[filter] }}</span></button>
    </nav>

    <form v-if="creating" class="annotation-editor annotation-editor--new" @submit.prevent="submitCreate">
      <div class="annotation-editor-title"><MessageSquareText :size="16" /><strong>新建批注</strong><button type="button" title="取消新建" @click="cancelCreate"><X :size="15" /></button></div>
      <label>标题<input v-model="draft.title" placeholder="例如：术语确认" required /></label><label>内容<textarea v-model="draft.body" rows="4" placeholder="描述需要核对或处理的问题" required></textarea></label><label>状态<select v-model="draft.status"><option value="draft">草稿</option><option value="in_progress">进行中</option><option value="resolved">已解决</option></select></label><footer><button class="annotation-secondary" type="button" @click="cancelCreate">取消</button><button class="annotation-primary" type="submit"><Save :size="14" />创建</button></footer>
    </form>

    <div class="annotation-list">
      <article v-for="annotation in filtered" :key="annotation.id" class="annotation-card" :class="[{ 'annotation-card--selected': annotation.id === selectedId }, statusClass(annotation.status)]" @click="emit('select', annotation.id)">
        <div class="annotation-number" :class="statusClass(annotation.status)">{{ annotation.number }}</div>
        <div class="annotation-card-body">
          <div class="annotation-state"><span class="annotation-state-pill" :class="statusClass(annotation.status)">{{ statusLabels[annotation.status].zh }}</span><span>{{ statusLabels[annotation.status].en }}</span><time>{{ annotation.createdAt }}</time></div>
          <template v-if="editingId === annotation.id">
            <div class="annotation-editor"><label>标题<input v-model="draft.title" /></label><label>内容<textarea v-model="draft.body" rows="4"></textarea></label><label>状态<select v-model="draft.status"><option value="draft">草稿</option><option value="in_progress">进行中</option><option value="resolved">已解决</option></select></label><footer><button class="annotation-secondary" type="button" @click.stop="cancelEdit">取消</button><button class="annotation-primary" type="button" @click.stop="submitEdit(annotation.id)"><Save :size="14" />保存</button></footer></div>
          </template>
          <template v-else>
            <h3>{{ annotation.title }}</h3><p>{{ annotation.body }}</p>
            <div v-if="annotation.links.length" class="annotation-links"><button v-for="link in annotation.links" :key="`${annotation.id}-${link.side}-${link.segmentId}`" type="button" :title="`回到 ${link.label ?? link.segmentId}`" @click.stop="emit('open-link', link.segmentId)"><span>{{ link.side === "source" ? "中文（原文）" : "English（译文）" }} <b :title="link.segmentId">{{ link.label ?? link.segmentId }}</b><small v-if="link.text">{{ link.text }}</small></span><CornerUpLeft :size="14" /></button></div>
            <footer class="annotation-card-actions"><button type="button" @click.stop="startEdit(annotation)"><Pencil :size="13" />编辑</button><button type="button" @click.stop="emit('delete', annotation.id)"><Trash2 :size="13" />删除</button><button v-if="annotation.status !== 'resolved'" class="annotation-resolve" type="button" @click.stop="emit('resolve', annotation.id)"><Check :size="13" />标记已解决</button></footer>
          </template>
        </div>
      </article>
      <div v-if="!filtered.length" class="annotation-empty"><MessageSquareText :size="18" />当前筛选下暂无批注。</div>
    </div>
    <button v-if="!readonly && !creating" class="annotation-new" type="button" @click="startCreate"><Plus :size="16" />新建批注</button>
  </aside>
</template>

<style scoped>
.annotation-workspace { display: flex; width: 100%; height: 100%; min-width: 0; min-height: 0; flex-direction: column; overflow: hidden; border: 1px solid #ccd9cd; border-right: 0; border-radius: 16px 0 0 16px; color: var(--ink-900); background: var(--surface-subtle); box-shadow: -12px 0 30px rgb(32 51 35 / 10%); }.annotation-heading { display: flex; align-items: flex-start; justify-content: space-between; padding: 19px 18px 12px; }.annotation-heading button { padding: 5px; border: 0; color: var(--ink-500); background: transparent; cursor: pointer; }.annotation-eyebrow { color: var(--green-700); font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-semibold); letter-spacing: .1em; line-height: var(--jm-line-height-subheadline); }.annotation-heading h2 { margin: 4px 0 0; font-size: var(--jm-font-size-title-3); line-height: var(--jm-line-height-title-3); font-weight: var(--jm-font-weight-semibold); }.annotation-heading h2 small { display: inline-block; margin-left: 3px; padding: 2px 6px; border-radius: 10px; color: var(--green-900); background: var(--green-100); font-size: var(--jm-font-size-subheadline); line-height: var(--jm-line-height-subheadline); }.annotation-filter { display: flex; gap: 4px; padding: 0 13px 13px; border-bottom: 1px solid var(--line); }.annotation-filter button { padding: 7px 8px; border: 1px solid transparent; border-radius: 5px; color: var(--ink-500); background: transparent; font-size: var(--jm-font-size-callout); cursor: pointer; line-height: var(--jm-line-height-callout); }.annotation-filter button.active { border-color: #b3d5b7; color: var(--green-900); background: var(--surface-green-selected); }.annotation-filter span { margin-left: 2px; color: var(--ink-500); }.annotation-list { min-height: 0; overflow: auto; padding-bottom: 4px; }.annotation-card { display: grid; grid-template-columns: 25px 1fr; gap: 7px; margin: 13px 13px 0; padding: 13px 11px; border: 1px solid #d3dde5; border-radius: 8px; background: var(--surface-raised); cursor: pointer; }.annotation-card--selected { outline: 2px solid rgb(47 129 67 / 20%); }.annotation-card--draft { border-color: #d3c4ec; }.annotation-card--resolved { opacity: .78; }.annotation-number { display: grid; place-items: center; width: 22px; height: 22px; border-radius: 6px; color: #fff; background: #9864d5; font-size: var(--jm-font-size-callout); line-height: var(--jm-line-height-callout); }.annotation-number.annotation-status--in_progress { background: #3c9a5a; }.annotation-number.annotation-status--resolved { background: #7d877f; }.annotation-state { display: flex; align-items: center; gap: 7px; color: var(--ink-900); font-size: var(--jm-font-size-callout); font-weight: var(--jm-font-weight-semibold); line-height: var(--jm-line-height-callout); }.annotation-state > span:nth-child(2) { color: var(--ink-500); font-weight: var(--jm-font-weight-medium); }.annotation-state time { margin-left: auto; color: var(--ink-500); font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-regular); line-height: var(--jm-line-height-subheadline); }.annotation-state-pill { padding: 3px 6px; border-radius: 4px; color: #fff; background: #9864d5; }.annotation-state-pill.annotation-status--in_progress { background: #3c9a5a; }.annotation-state-pill.annotation-status--resolved { background: #7d877f; }.annotation-card h3 { margin: 12px 0 7px; font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); font-weight: var(--jm-font-weight-semibold); }.annotation-card p { margin: 0; color: var(--ink-700); font-size: var(--jm-font-size-body); line-height: 1.55; }.annotation-links { display: flex; flex-direction: column; gap: 5px; margin-top: 10px; padding: 8px; border-radius: 5px; background: var(--surface-muted); color: var(--ink-700); font-size: var(--jm-font-size-subheadline); line-height: var(--jm-line-height-subheadline); }.annotation-links b { float: right; color: var(--ink-500); font-weight: var(--jm-font-weight-medium); }.annotation-links small { display: block; color: var(--ink-700); }.annotation-card-actions { display: flex; flex-wrap: wrap; gap: 12px; margin-top: 10px; padding-top: 9px; border-top: 1px solid var(--line); }.annotation-card-actions button, .annotation-new { display: inline-flex; align-items: center; gap: 5px; border: 0; color: var(--ink-700); background: transparent; font-size: var(--jm-font-size-callout); cursor: pointer; line-height: var(--jm-line-height-callout); }.annotation-card-actions button:hover { color: var(--green-900); }.annotation-resolve { color: var(--green-900) !important; }.annotation-new { justify-content: center; margin: 13px; padding: 10px; border: 1px solid #abd1af; border-radius: 6px; color: var(--green-900); background: var(--surface-green-soft); }.annotation-editor { display: grid; gap: 8px; }.annotation-editor--new { margin: 13px; padding: 13px; border: 1px solid #abd1af; border-radius: 7px; background: var(--surface-green-soft); }.annotation-editor-title { display: flex; align-items: center; gap: 7px; color: var(--green-900); }.annotation-editor-title button { margin-left: auto; padding: 2px; border: 0; color: var(--ink-500); background: transparent; cursor: pointer; }.annotation-editor label { display: grid; gap: 4px; color: var(--ink-700); font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }.annotation-editor input, .annotation-editor textarea, .annotation-editor select { width: 100%; padding: 7px; border: 1px solid #c8d3c9; border-radius: 4px; color: var(--ink-900); background: var(--surface-input); font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }.annotation-editor textarea { resize: vertical; }.annotation-editor footer { display: flex; justify-content: flex-end; gap: 7px; margin-top: 2px; }.annotation-primary, .annotation-secondary { display: inline-flex; align-items: center; gap: 5px; padding: 7px 11px; border: 1px solid var(--line); border-radius: 5px; background: var(--surface-raised); cursor: pointer; }.annotation-primary { border-color: var(--green-700); color: #fff; background: var(--green-700); }.annotation-empty { display: flex; min-height: 90px; align-items: center; justify-content: center; gap: 7px; color: var(--ink-500); font-size: var(--jm-font-size-callout); line-height: var(--jm-line-height-callout); }
.annotation-links button { display: grid; grid-template-columns: minmax(0, 1fr) 18px; align-items: center; gap: 7px; padding: 5px; border: 0; border-radius: 5px; color: inherit; background: transparent; text-align: left; cursor: pointer; }
.annotation-links button:hover { color: var(--green-900); background: var(--surface-green-soft); }
.annotation-links button > svg { justify-self: end; }
.annotation-links button small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
