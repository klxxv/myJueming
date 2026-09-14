<script setup lang="ts">
import { computed } from "vue";
import { Link2, Link2Off } from "@lucide/vue";
import type { AlignmentBlockView } from "../../domain/workspace-projection";
import { t } from "../../i18n";

const props = withDefaults(defineProps<{
  block: AlignmentBlockView;
  writable: boolean;
  connectorY: number;
  active?: boolean;
  operationSelected?: boolean;
}>(), {
  active: false,
  operationSelected: false,
});

const emit = defineEmits<{
  select: [event: MouseEvent];
  quickAction: [];
}>();

const relationKind = computed(() => `${props.block.sourceSegments.length}:${props.block.targetSegments.length}`);
const relationLabel = computed(() => props.block.linked
  ? `A${String(props.block.index + 1).padStart(3, "0")} · ${relationKind.value}`
  : t("puiUnaligned"));
const canQuickAction = computed(() => props.writable && (props.block.linked
  || (props.block.sourceSegments.length > 0 && props.block.targetSegments.length > 0)));
const quickActionLabel = computed(() => props.block.linked
  ? t("puiQuickUnlink", { p0: relationLabel.value })
  : t("puiQuickLink", { p0: relationKind.value }));
</script>

<template>
  <div
    class="alignment-relation"
    :style="{ '--connector-y': `${connectorY}px` }"
    :class="{
      'alignment-relation--linked': block.linked,
      'alignment-relation--active': active,
      'alignment-relation--selected': operationSelected,
    }"
  >
    <span v-if="block.linked" class="alignment-relation__line" aria-hidden="true"></span>
    <div class="alignment-relation__pill">
      <button
        class="alignment-relation__label"
        type="button"
        :aria-pressed="operationSelected"
        :aria-label="block.linked ? t('puiSelectRelation', { p0: relationLabel }) : t('puiSelectUnalignedBlock')"
        :title="t(block.linked ? 'puiSelectAlignmentTitle' : 'puiSelectUnalignedTitle')"
        @click.stop="emit('select', $event)"
      >{{ relationLabel }}</button>
      <button
        class="alignment-relation__quick"
        type="button"
        :disabled="!canQuickAction"
        :aria-label="quickActionLabel"
        :title="quickActionLabel"
        @click.stop="emit('quickAction')"
      ><Link2Off v-if="block.linked" :size="15" /><Link2 v-else :size="15" /></button>
    </div>
  </div>
</template>

<style scoped>
.alignment-relation { position: relative; width: 100%; height: 100%; min-height: 76px; }
.alignment-relation__line { position: absolute; z-index: 1; top: var(--connector-y); right: 0; left: 0; height: 1px; background: #72b47c; transform: translateY(-50%); }
.alignment-relation__pill { position: absolute; z-index: 3; top: var(--connector-y); left: 50%; display: inline-flex; min-width: 0; overflow: hidden; border: 1px solid #a6cdaa; border-radius: 999px; color: var(--green-900); background: var(--surface-green-soft); box-shadow: 0 1px 3px rgb(31 53 34 / 8%); transform: translate(-50%, -50%); }
.alignment-relation__label, .alignment-relation__quick { height: 27px; border: 0; color: inherit; background: transparent; cursor: pointer; }
.alignment-relation__label { max-width: 112px; overflow: hidden; padding: 0 8px 0 10px; font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-semibold); text-overflow: ellipsis; white-space: nowrap; line-height: var(--jm-line-height-subheadline); }
.alignment-relation__quick { display: grid; width: 29px; padding: 0; place-items: center; border-left: 1px solid #b9d6bc; }
.alignment-relation__label:hover, .alignment-relation__quick:hover:not(:disabled) { background: var(--surface-green-selected); }
.alignment-relation--active .alignment-relation__pill { border-color: var(--green-700); box-shadow: 0 0 0 2px rgb(47 129 67 / 15%); }
.alignment-relation--selected .alignment-relation__pill { color: #fff; background: var(--green-700); }
.alignment-relation--selected .alignment-relation__quick { border-left-color: rgb(255 255 255 / 34%); }
.alignment-relation:not(.alignment-relation--linked) .alignment-relation__pill { border-color: #e5b74d; color: #a66f00; background: var(--surface-warm-soft); }
.alignment-relation__quick:disabled { cursor: not-allowed; opacity: .42; }
</style>
