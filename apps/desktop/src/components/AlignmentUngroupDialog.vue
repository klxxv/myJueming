<script setup lang="ts">
import { t } from '../i18n';
import { computed, ref, watch } from "vue";
import { Check, Split, X } from "@lucide/vue";
import type { AlignmentDto, SegmentDto } from "../domain/kernel-client";

const props = defineProps<{
  alignment: AlignmentDto;
  sourceSegments: SegmentDto[];
  targetSegments: SegmentDto[];
}>();
const emit = defineEmits<{
  close: [];
  confirm: [alignmentId: string, sourceGroups: string[][], targetGroups: string[][]];
}>();

const sourceBreaks = ref(new Set<number>());
const targetBreaks = ref(new Set<number>());
const sourceById = computed(() => new Map(props.sourceSegments.map((segment) => [segment.id, segment])));
const targetById = computed(() => new Map(props.targetSegments.map((segment) => [segment.id, segment])));

const resetSuggestion = () => {
  const canSuggestOneToOne = props.alignment.sourceIds.length === props.alignment.targetIds.length
    && props.alignment.sourceIds.length > 1;
  sourceBreaks.value = new Set(canSuggestOneToOne ? props.alignment.sourceIds.slice(1).map((_, index) => index + 1) : []);
  targetBreaks.value = new Set(canSuggestOneToOne ? props.alignment.targetIds.slice(1).map((_, index) => index + 1) : []);
};
watch(() => props.alignment.id, resetSuggestion, { immediate: true });

const toggleBreak = (side: "source" | "target", position: number) => {
  const current = side === "source" ? sourceBreaks : targetBreaks;
  const next = new Set(current.value);
  if (next.has(position)) next.delete(position); else next.add(position);
  current.value = next;
};
const makeGroups = (ids: string[], breaks: Set<number>) => {
  const groups: string[][] = [];
  let current: string[] = [];
  ids.forEach((id, index) => {
    current.push(id);
    if (breaks.has(index + 1) || index === ids.length - 1) {
      groups.push(current);
      current = [];
    }
  });
  return groups;
};
const sourceGroups = computed(() => makeGroups(props.alignment.sourceIds, sourceBreaks.value));
const targetGroups = computed(() => makeGroups(props.alignment.targetIds, targetBreaks.value));
const isValid = computed(() => sourceGroups.value.length >= 2 && sourceGroups.value.length === targetGroups.value.length);
const validationMessage = computed(() => {
  if (isValid.value) return t('ungroupPreview', { p0: sourceGroups.value.length });
  if (Math.min(props.alignment.sourceIds.length, props.alignment.targetIds.length) < 2) return t('ungroupSingleSide');
  return t('ungroupCountMismatch', { p0: sourceGroups.value.length, p1: targetGroups.value.length });
});
const segmentText = (side: "source" | "target", id: string) => {
  const segment = (side === "source" ? sourceById.value : targetById.value).get(id);
  return segment?.text ?? t('segmentOutsideSlice');
};
const segmentOrder = (side: "source" | "target", id: string) => {
  const segment = (side === "source" ? sourceById.value : targetById.value).get(id);
  return segment ? String(segment.order + 1).padStart(6, "0") : id.slice(0, 8);
};
</script>

<template>
  <Teleport to="body">
    <section class="ungroup-backdrop" role="dialog" aria-modal="true" aria-labelledby="ungroup-title" @click.self="emit('close')">
      <div class="ungroup-dialog">
        <header><div><small>{{ t('relationOnly') }}</small><h2 id="ungroup-title">{{ t('ungroupTitle') }}</h2></div><button type="button" :title="t('close')" @click="emit('close')"><X :size="17" /></button></header>
        <p>{{ t('ungroupInstructions') }}</p>
        <div class="ungroup-editor">
          <section :aria-label="t('sourceGroups')">
            <h3>{{ t('sourceSegment') }} <span>{{ t('groupCount', { count: sourceGroups.length }) }}</span></h3>
            <template v-for="(id, index) in alignment.sourceIds" :key="id">
              <article><b>{{ segmentOrder('source', id) }}</b><span>{{ segmentText('source', id) }}</span></article>
              <button v-if="index < alignment.sourceIds.length - 1" class="group-boundary" :class="{ 'group-boundary--active': sourceBreaks.has(index + 1) }" type="button" @click="toggleBreak('source', index + 1)"><Split :size="14" />{{ sourceBreaks.has(index + 1) ? t('groupBoundary') : t('splitGroupHere') }}</button>
            </template>
          </section>
          <section :aria-label="t('targetGroups')">
            <h3>{{ t('targetSegment') }} <span>{{ t('groupCount', { count: targetGroups.length }) }}</span></h3>
            <template v-for="(id, index) in alignment.targetIds" :key="id">
              <article><b>{{ segmentOrder('target', id) }}</b><span>{{ segmentText('target', id) }}</span></article>
              <button v-if="index < alignment.targetIds.length - 1" class="group-boundary" :class="{ 'group-boundary--active': targetBreaks.has(index + 1) }" type="button" @click="toggleBreak('target', index + 1)"><Split :size="14" />{{ targetBreaks.has(index + 1) ? t('groupBoundary') : t('splitGroupHere') }}</button>
            </template>
          </section>
        </div>
        <p class="ungroup-validation" :class="{ 'ungroup-validation--valid': isValid }">{{ validationMessage }}</p>
        <footer><button class="secondary-button" type="button" @click="emit('close')">{{ t('cancel') }}</button><button class="primary-button" type="button" :disabled="!isValid" @click="emit('confirm', alignment.id, sourceGroups, targetGroups)"><Check :size="15" />{{ t('confirmUngroup') }}</button></footer>
      </div>
    </section>
  </Teleport>
</template>

<style scoped>
.ungroup-backdrop { position: fixed; z-index: 100; inset: 0; display: grid; place-items: center; background: rgb(31 42 34 / 25%); }.ungroup-dialog { display: flex; flex-direction: column; width: min(860px, calc(100vw - 48px)); max-height: min(760px, calc(100vh - 48px)); padding: 20px; border: 1px solid var(--line); border-radius: 10px; color: var(--ink-900); background: var(--surface-raised); box-shadow: 0 22px 70px rgb(29 48 32 / 23%); }.ungroup-dialog header, .ungroup-dialog footer { display: flex; align-items: center; justify-content: space-between; }.ungroup-dialog header small { color: var(--green-700); font-size: 10px; font-weight: 700; letter-spacing: .08em; }.ungroup-dialog header h2 { margin: 4px 0 0; font-size: 20px; }.ungroup-dialog header button { border: 0; color: var(--ink-500); background: transparent; cursor: pointer; }.ungroup-dialog > p { margin: 14px 0; color: var(--ink-700); font-size: 13px; line-height: 1.55; }.ungroup-editor { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 14px; min-height: 0; overflow: auto; }.ungroup-editor > section { min-width: 0; padding: 12px; border: 1px solid var(--line); border-radius: 8px; background: var(--surface-muted); }.ungroup-editor h3 { display: flex; align-items: center; justify-content: space-between; margin: 0 0 10px; font-size: 13px; }.ungroup-editor h3 span { color: var(--green-700); font-size: 11px; }.ungroup-editor article { display: grid; grid-template-columns: 62px minmax(0, 1fr); gap: 9px; padding: 10px; border: 1px solid var(--line); border-radius: 6px; background: var(--paper); }.ungroup-editor article b { color: var(--ink-500); font-family: ui-monospace, Consolas, monospace; font-size: 10px; }.ungroup-editor article span { overflow: hidden; font-size: 12px; line-height: 1.45; text-overflow: ellipsis; white-space: nowrap; }.group-boundary { display: flex; width: 100%; height: 28px; align-items: center; justify-content: center; gap: 5px; border: 0; color: var(--ink-500); background: transparent; font-size: 10px; cursor: pointer; }.group-boundary::before, .group-boundary::after { height: 1px; flex: 1; background: var(--line); content: ""; }.group-boundary--active { color: var(--green-900); font-weight: 700; }.group-boundary--active::before, .group-boundary--active::after { height: 2px; background: var(--green-700); }.ungroup-validation { color: #916714 !important; }.ungroup-validation--valid { color: var(--green-700) !important; }.ungroup-dialog footer { gap: 9px; margin-top: 3px; justify-content: flex-end; }
@media (max-width: 680px) { .ungroup-editor { grid-template-columns: 1fr; } }
</style>
