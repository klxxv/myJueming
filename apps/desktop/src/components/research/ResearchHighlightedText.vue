<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{ text: string; ranges: Array<{ start_utf8: number; end_utf8: number }> }>();
const pieces = computed(() => {
  const encoder = new TextEncoder();
  let offset = 0;
  const result: Array<{ text: string; highlighted: boolean }> = [];
  for (const character of props.text) {
    const end = offset + encoder.encode(character).length;
    const highlighted = props.ranges.some(range => range.start_utf8 <= offset && range.end_utf8 >= end);
    const previous = result[result.length - 1];
    if (previous && previous.highlighted === highlighted) previous.text += character;
    else result.push({ text: character, highlighted });
    offset = end;
  }
  return result;
});
</script>

<template><span><template v-for="(piece, index) in pieces" :key="index"><mark v-if="piece.highlighted">{{ piece.text }}</mark><template v-else>{{ piece.text }}</template></template></span></template>

<style scoped>
mark { @apply text-accent-strong bg-green-soft; border-radius: 3px; padding-block: 1px; }
</style>
