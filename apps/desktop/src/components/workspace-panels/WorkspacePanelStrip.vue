<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { ArrowLeft, ArrowRight, GripVertical } from '@lucide/vue';
import { draggable, dropTargetForElements } from '@atlaskit/pragmatic-drag-and-drop/element/adapter';
import { t } from '../../i18n';

/** Generic panel chrome. No Segment, Alignment, text loading or domain commands. */
export interface PanelDescriptor { id: string; title: string; subtitle?: string }
const props = defineProps<{ panels: PanelDescriptor[]; storageKey: string }>();
const emit = defineEmits<{ layout: [widths: Record<string, number>] }>();
const order = ref<string[]>([]);
const widths = ref<Record<string, number>>({});
const dropId = ref('');
const ordered = computed(() => order.value.flatMap(id => { const panel = props.panels.find(item => item.id === id); return panel ? [panel] : []; }));
const persist = () => { try { localStorage.setItem(props.storageKey, JSON.stringify({ order: order.value, widths: widths.value })); } catch { /* Device preferences are discardable. */ } };
watch(() => [props.storageKey, props.panels.map(panel => panel.id).join(',')], () => {
  let saved: { order?: unknown; widths?: Record<string, unknown> } = {};
  try { saved = JSON.parse(localStorage.getItem(props.storageKey) ?? '{}'); } catch { /* Reset invalid layout. */ }
  const ids = props.panels.map(panel => panel.id);
  order.value = [...new Set([...(Array.isArray(saved?.order) ? saved.order.filter((id): id is string => typeof id === 'string' && ids.includes(id)) : []), ...ids])];
  widths.value = Object.fromEntries(ids.map(id => [id, typeof saved?.widths?.[id] === 'number' ? Math.min(900, Math.max(280, saved.widths[id] as number)) : 360]));
}, { immediate: true });
watch(widths, value => emit('layout', value), { immediate: true });
const move = (id: string, target: string) => { const next = order.value.filter(item => item !== id); next.splice(order.value.indexOf(target), 0, id); order.value = next; persist(); };
const moveBy = (id: string, offset: number) => { const target = order.value[order.value.indexOf(id) + offset]; if (target) move(id, target); };
const bindings = new Map<string, { element: HTMLElement; cleanup: () => void }>();
const bind = (id: string, element: unknown) => {
  const previous = bindings.get(id);
  if (previous?.element === element) return;
  previous?.cleanup(); bindings.delete(id);
  if (!(element instanceof HTMLElement)) return;
  const handle = element.querySelector<HTMLElement>('[data-panel-handle]')!;
  const cleanDrag = draggable({ element, dragHandle: handle, onGenerateDragPreview: ({ nativeSetDragImage }) => nativeSetDragImage?.(handle, 0, 0), getInitialData: () => ({ type: 'workspace-panel', scope: props.storageKey, id }) });
  const cleanDrop = dropTargetForElements({ element, canDrop: ({ source }) => source.data.type === 'workspace-panel' && source.data.scope === props.storageKey && source.data.id !== id,
    onDragEnter: () => { dropId.value = id; }, onDragLeave: () => { dropId.value = ''; }, onDrop: ({ source }) => { dropId.value = ''; move(String(source.data.id), id); } });
  bindings.set(id, { element, cleanup: () => { cleanDrag(); cleanDrop(); } });
};
let stopResize: (() => void) | undefined;
const setWidth = (id: string, value: number) => { widths.value = { ...widths.value, [id]: Math.min(900, Math.max(280, value)) }; };
const resize = (id: string, event: PointerEvent) => {
  if (event.button !== 0) return;
  stopResize?.(); event.preventDefault();
  const start = event.clientX; const width = widths.value[id];
  const scale = (event.currentTarget as HTMLElement).closest<HTMLElement>('.workspace-panel')!.getBoundingClientRect().width / width;
  const onMove = (move: PointerEvent) => setWidth(id, width + (move.clientX - start) / scale);
  stopResize = () => { window.removeEventListener('pointermove', onMove); window.removeEventListener('pointerup', stop); window.removeEventListener('pointercancel', stop); persist(); stopResize = undefined; };
  const stop = () => stopResize?.();
  window.addEventListener('pointermove', onMove); window.addEventListener('pointerup', stop); window.addEventListener('pointercancel', stop);
};
onBeforeUnmount(() => { stopResize?.(); bindings.forEach(binding => binding.cleanup()); });
</script>

<template>
  <div class="workspace-panel-strip">
    <section v-for="(panel, index) in ordered" :key="panel.id" :ref="element => bind(panel.id, element)" class="workspace-panel" :class="{ 'workspace-panel--drop': dropId === panel.id }" :data-panel-id="panel.id" :aria-label="panel.title" :style="{ width: `${widths[panel.id]}px` }">
      <header class="workspace-panel__header">
        <span data-panel-handle class="workspace-panel__handle" :title="t('puiPanelDragTitle')"><GripVertical :size="17" /></span>
        <div class="workspace-panel__title"><strong>{{ panel.title }}</strong><small>{{ panel.subtitle }}</small></div>
        <button type="button" :disabled="index === 0" :aria-label="t('puiPanelMoveLeft', { p0: panel.title })" @click="moveBy(panel.id, -1)"><ArrowLeft :size="14" /></button>
        <button type="button" :disabled="index === ordered.length - 1" :aria-label="t('puiPanelMoveRight', { p0: panel.title })" @click="moveBy(panel.id, 1)"><ArrowRight :size="14" /></button>
      </header>
      <slot :panel="panel" :width="widths[panel.id]" />
      <div class="workspace-panel__resize" role="separator" tabindex="0" aria-orientation="vertical" :aria-label="t('puiPanelResize', { p0: panel.title })" :aria-valuenow="widths[panel.id]" :aria-valuemin="280" :aria-valuemax="900" @pointerdown="resize(panel.id, $event)" @keydown.left.prevent="setWidth(panel.id, widths[panel.id] - 20); persist()" @keydown.right.prevent="setWidth(panel.id, widths[panel.id] + 20); persist()" @dblclick="setWidth(panel.id, 360); persist()"></div>
    </section>
  </div>
</template>

<style scoped>
.workspace-panel-strip { display: flex; min-width: 100%; width: max-content; align-items: stretch; }
.workspace-panel { position: relative; flex: none; min-width: 0; border-right: 1px solid var(--line); @apply bg-raised; }
.workspace-panel--drop { box-shadow: inset 4px 0 var(--green-700); }
.workspace-panel__header { position: sticky; top: 0; z-index: 25; display: flex; align-items: center; height: 58px; gap: 6px; padding: 8px 12px; border-bottom: 1px solid var(--line); @apply bg-subtle; }
.workspace-panel__title { min-width: 0; flex: 1; display: grid; gap: 4px; }
.workspace-panel__title strong { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
.workspace-panel__title small { color: var(--text-muted); font-size: 11px; }
.workspace-panel__handle { display: flex; cursor: grab; padding: 6px 0; }
.workspace-panel__header button { display: grid; place-items: center; padding: 5px; border: 0; border-radius: 4px; color: var(--text-muted); background: transparent; cursor: pointer; }
.workspace-panel__header button:hover { @apply bg-green-soft; }
.workspace-panel__header button:disabled { opacity: .3; }
.workspace-panel__resize { position: absolute; z-index: 30; top: 0; right: -3px; width: 6px; height: 100%; cursor: col-resize; touch-action: none; }
.workspace-panel__resize:hover, .workspace-panel__resize:focus-visible { @apply bg-accent; }
</style>
