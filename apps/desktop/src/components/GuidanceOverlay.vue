<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from "vue";
import butterfly from "../assets/companion/butterfly.svg";
const props = defineProps<{ target: { selector: string; label: string; key: number } | null; animated: boolean; visible: boolean }>();
const position = ref<{ x: number; y: number } | null>(null);
const icon = ref<HTMLElement | null>(null);
let timer: ReturnType<typeof setTimeout> | undefined;
let animation: Animation | undefined;
let observer: ResizeObserver | undefined;
let generation = 0;
function dispose() { generation++; clearTimeout(timer); animation?.cancel(); animation = undefined; observer?.disconnect(); observer = undefined; window.removeEventListener("resize", measure); document.removeEventListener("scroll", measure, true); }
function measure() {
  const anchor = props.target ? document.querySelector<HTMLElement>(props.target.selector) : null;
  if (!anchor || !anchor.getClientRects().length) { position.value = null; return; }
  const rect = anchor.getBoundingClientRect();
  position.value = { x: Math.min(window.innerWidth - 180, Math.max(8, rect.right - 20)), y: Math.max(8, rect.top - 27) };
}
watch(() => [props.target?.key, props.visible, props.animated], async () => {
  dispose(); position.value = null;
  const activeGeneration = generation;
  if (!props.visible || !props.target) return;
  await nextTick();
  if (activeGeneration !== generation) return;
  measure();
  window.addEventListener("resize", measure); document.addEventListener("scroll", measure, true);
  const anchor = document.querySelector(props.target.selector);
  if (anchor) { observer = new ResizeObserver(measure); observer.observe(anchor); }
  await nextTick();
  if (activeGeneration !== generation) return;
  if (props.animated && icon.value) animation = icon.value.animate([{ transform: "translate(-20px, 14px)", opacity: 0 }, { transform: "translate(0, 0)", opacity: 1 }], { duration: 450, easing: "ease-out", iterations: 1 });
  timer = setTimeout(() => { position.value = null; dispose(); }, 4500);
}, { immediate: true });
onBeforeUnmount(dispose);
</script>

<template><div v-if="position && visible" ref="icon" class="guidance" :style="{ left: `${position.x}px`, top: `${position.y}px` }" role="status"><img :src="butterfly" alt="" /><span>{{ target?.label }}</span></div></template>

<style scoped>
.guidance { position: fixed; z-index: 60; display: flex; gap: 6px; align-items: center; pointer-events: none; }.guidance img { width: 32px; height: 28px; }.guidance span { padding: 5px 8px; border: 1px solid var(--line); border-radius: 6px; @apply bg-raised text-accent-strong; font-size: 11px; box-shadow: 0 3px 10px rgb(35 55 38 / 8%); }
</style>
