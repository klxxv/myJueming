<script setup lang="ts">
import { t } from '../i18n';
import { computed, ref, watch } from "vue";
import { Check, X } from "@lucide/vue";

export type ContentOperationRequest =
  | { kind: "merge"; segmentIds: string[]; initialValue: string }
  | { kind: "split"; segmentId: string; original: string };

const props = defineProps<{ operation: ContentOperationRequest }>();
const emit = defineEmits<{
  close: [];
  merge: [segmentIds: string[], mergedContent: string];
  split: [segmentId: string, parts: string[]];
}>();

// Pin this editing token for the dialog lifetime: switching UI language must not
// invalidate markers already inserted into a user's unsaved split draft.
const splitMarker = t('splitMarker');
const value = ref("");

const resetValue = () => {
  value.value = props.operation.kind === "merge" ? props.operation.initialValue : props.operation.original;
};
watch(() => props.operation, resetValue, { immediate: true });

const splitParts = computed(() => props.operation.kind === "split" ? value.value.split(splitMarker) : []);
const splitIsLossless = computed(() => props.operation.kind === "split"
  && splitParts.value.length > 1
  && splitParts.value.every(Boolean)
  && splitParts.value.join("") === props.operation.original);

const insertSplitMarker = () => { value.value += splitMarker; };
const confirm = () => {
  if (props.operation.kind === "merge") emit("merge", props.operation.segmentIds, value.value);
  else if (splitIsLossless.value) emit("split", props.operation.segmentId, splitParts.value);
};
</script>

<template>
  <Teleport to="body">
    <section class="content-operation-backdrop" role="dialog" aria-modal="true" :aria-label="operation.kind === 'merge' ? t('mergeSegmentTitle') : t('splitSegmentTitle')" @click.self="emit('close')">
      <div class="content-operation-dialog">
        <header><div><small>{{ t('segmentContentEyebrow') }}</small><h2>{{ operation.kind === 'merge' ? t('mergeContent') : t('splitContent') }}</h2></div><button type="button" :title="t('close')" @click="emit('close')"><X :size="17" /></button></header>
        <p v-if="operation.kind === 'merge'">{{ t('mergeDescription', { count: operation.segmentIds.length }) }}</p>
        <p v-else>{{ t('splitInstructions') }}</p>
        <textarea v-model="value" :aria-label="operation.kind === 'merge' ? t('mergedContent') : t('contentWithSplitPoints')" />
        <div v-if="operation.kind === 'split'" class="split-validation"><button type="button" @click="insertSplitMarker">{{ t('insertSplitPoint') }}</button><span :class="{ 'split-validation--valid': splitIsLossless }">{{ splitIsLossless ? t('splitValid', { p0: splitParts.length }) : t('splitInvalid') }}</span></div>
        <footer><button class="secondary-button" type="button" @click="emit('close')">{{ t('cancel') }}</button><button class="primary-button" type="button" :disabled="operation.kind === 'split' && !splitIsLossless" @click="confirm"><Check :size="15" />{{ operation.kind === 'merge' ? t('confirmMerge') : t('confirmSplit') }}</button></footer>
      </div>
    </section>
  </Teleport>
</template>

<style scoped>
.content-operation-backdrop { position: fixed; z-index: 100; inset: 0; display: grid; place-items: center; background: rgb(31 42 34 / 25%); }.content-operation-dialog { display: flex; flex-direction: column; width: min(620px, calc(100vw - 48px)); max-height: min(680px, calc(100vh - 64px)); padding: 20px; border: 1px solid var(--line); border-radius: 10px; color: var(--ink-900); background: var(--surface-raised); box-shadow: 0 22px 70px rgb(29 48 32 / 23%); }.content-operation-dialog header, .content-operation-dialog footer { display: flex; align-items: center; justify-content: space-between; }.content-operation-dialog header small { color: var(--green-700); font-size: 10px; font-weight: 700; letter-spacing: .08em; }.content-operation-dialog header h2 { margin: 4px 0 0; font-size: 20px; }.content-operation-dialog header button { border: 0; color: var(--ink-500); background: transparent; cursor: pointer; }.content-operation-dialog > p { margin: 14px 0 10px; color: var(--ink-700); font-size: 13px; line-height: 1.55; }.content-operation-dialog textarea { flex: 1; min-height: 220px; resize: vertical; padding: 11px; border: 1px solid #bfcdbf; border-radius: 6px; color: var(--ink-900); background: var(--paper); font: inherit; line-height: 1.6; }.content-operation-dialog textarea:focus { border-color: var(--green-700); outline: 2px solid rgb(55 127 66 / 14%); }.split-validation { display: flex; align-items: center; gap: 10px; margin-top: 10px; color: #916714; font-size: 12px; }.split-validation button { padding: 6px 8px; border: 1px solid #c4d0c5; border-radius: 5px; color: var(--green-900); background: var(--surface-raised); cursor: pointer; }.split-validation--valid { color: var(--green-700); }.content-operation-dialog footer { gap: 9px; margin-top: 17px; justify-content: flex-end; }
</style>
