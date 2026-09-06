<script setup lang="ts">
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

const splitMarker = "\n[[切分点：此标记不会写入文本]]\n";
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
    <section class="content-operation-backdrop" role="dialog" aria-modal="true" :aria-label="operation.kind === 'merge' ? '合并 Segment 内容' : '拆分 Segment 内容'" @click.self="emit('close')">
      <div class="content-operation-dialog">
        <header><div><small>SEGMENT CONTENT</small><h2>{{ operation.kind === 'merge' ? 'Merge 内容' : 'Split 内容' }}</h2></div><button type="button" title="关闭" @click="emit('close')"><X :size="17" /></button></header>
        <p v-if="operation.kind === 'merge'">将 {{ operation.segmentIds.length }} 个连续 Segment 合为一个内容单元；首个 Segment ID、批注与书签锚点会保留。</p>
        <p v-else>在保留原文每一个字符的前提下插入切分点。切分后的 parts 拼接必须等于原内容。</p>
        <textarea v-model="value" :aria-label="operation.kind === 'merge' ? '合并后的内容' : '包含切分点的原始内容'" />
        <div v-if="operation.kind === 'split'" class="split-validation"><button type="button" @click="insertSplitMarker">插入切分点</button><span :class="{ 'split-validation--valid': splitIsLossless }">{{ splitIsLossless ? `可无损拆分为 ${splitParts.length} 段` : '至少插入一个切分点；不可增删原文' }}</span></div>
        <footer><button class="secondary-button" type="button" @click="emit('close')">取消</button><button class="primary-button" type="button" :disabled="operation.kind === 'split' && !splitIsLossless" @click="confirm"><Check :size="15" />确认{{ operation.kind === 'merge' ? '合并' : '拆分' }}</button></footer>
      </div>
    </section>
  </Teleport>
</template>

<style scoped>
.content-operation-backdrop { position: fixed; z-index: 100; inset: 0; display: grid; place-items: center; background: rgb(31 42 34 / 25%); }.content-operation-dialog { display: flex; flex-direction: column; width: min(620px, calc(100vw - 48px)); max-height: min(680px, calc(100vh - 64px)); padding: 20px; border: 1px solid var(--line); border-radius: 10px; color: var(--ink-900); background: var(--surface-raised); box-shadow: 0 22px 70px rgb(29 48 32 / 23%); }.content-operation-dialog header, .content-operation-dialog footer { display: flex; align-items: center; justify-content: space-between; }.content-operation-dialog header small { color: var(--green-700); font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-semibold); letter-spacing: .08em; line-height: var(--jm-line-height-subheadline); }.content-operation-dialog header h2 { margin: 4px 0 0; font-size: var(--jm-font-size-title-2); line-height: var(--jm-line-height-title-2); font-weight: var(--jm-font-weight-regular); }.content-operation-dialog header button { border: 0; color: var(--ink-500); background: transparent; cursor: pointer; }.content-operation-dialog > p { margin: 14px 0 10px; color: var(--ink-700); font-size: var(--jm-font-size-body); line-height: 1.55; }.content-operation-dialog textarea { flex: 1; min-height: 220px; resize: vertical; padding: 11px; border: 1px solid #bfcdbf; border-radius: 6px; color: var(--ink-900); background: var(--paper); font: inherit; line-height: 1.6; }.content-operation-dialog textarea:focus { border-color: var(--green-700); outline: 2px solid rgb(55 127 66 / 14%); }.split-validation { display: flex; align-items: center; gap: 10px; margin-top: 10px; color: #916714; font-size: var(--jm-font-size-callout); line-height: var(--jm-line-height-callout); }.split-validation button { padding: 6px 8px; border: 1px solid #c4d0c5; border-radius: 5px; color: var(--green-900); background: var(--surface-raised); cursor: pointer; }.split-validation--valid { color: var(--green-700); }.content-operation-dialog footer { gap: 9px; margin-top: 17px; justify-content: flex-end; }
</style>
