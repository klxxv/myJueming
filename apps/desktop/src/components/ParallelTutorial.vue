<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { ArrowLeft, ArrowRight, RotateCcw, X } from "@lucide/vue";
import butterflyGuide from "../assets/tutorial/butterfly-guide-v2.gif";
import type { ParallelTutorialStep } from "../domain/parallel-tutorial";
import { t } from "../i18n";

const props = defineProps<{ open: boolean; step: number; steps: ParallelTutorialStep[] }>();
const emit = defineEmits<{ close: []; dismiss: []; reset: []; "update:step": [step: number] }>();
const targetRect = ref<DOMRect | null>(null);
const current = computed(() => props.steps[props.step]);
const viewport = ref({ width: window.innerWidth, height: window.innerHeight });
const flightPath = ref("");
const flightPathRef = ref<SVGPathElement | null>(null);
const butterfly = ref({ x: 28, y: 92, angle: 0, resting: false });
const butterflyVisible = ref(false);
let observedElement: HTMLElement | null = null;
let observer: ResizeObserver | undefined;
let layoutObserver: MutationObserver | undefined;
let layoutMeasureFrame = 0;
let animationFrame = 0;
let flightSequence = 0;

const clamp = (value: number, minimum: number, maximum: number) => Math.min(maximum, Math.max(minimum, value));
const butterflyGoal = (rect: DOMRect) => ({
  x: clamp(rect.right - (rect.width > 420 ? 24 : 14), 16, window.innerWidth - 16),
  y: clamp(rect.top + Math.min(20, rect.height / 2), 16, window.innerHeight - 16),
});
const butterflyStyle = computed(() => ({
  transform: `translate3d(${butterfly.value.x - 12}px, ${butterfly.value.y - 12}px, 0) rotate(${butterfly.value.angle}deg)`,
}));

function observe(element: HTMLElement | null) {
  if (observedElement === element) return;
  observer?.disconnect();
  observedElement = element;
  if (!element) return;
  observer = new ResizeObserver(() => measure(false));
  observer.observe(element);
}

function observeLayout() {
  layoutObserver?.disconnect();
  const layout = document.querySelector<HTMLElement>(".content-grid");
  if (!layout) return;
  layoutObserver = new MutationObserver(() => {
    cancelAnimationFrame(layoutMeasureFrame);
    layoutMeasureFrame = requestAnimationFrame(() => measure(false));
  });
  layoutObserver.observe(layout, { attributes: true, attributeFilter: ["class", "style"] });
}

async function flyTo(goal: { x: number; y: number }) {
  const sequence = ++flightSequence;
  cancelAnimationFrame(animationFrame);
  const prefersReducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (prefersReducedMotion) {
    flightPath.value = "";
    butterfly.value = { ...goal, angle: 0, resting: true };
    butterflyVisible.value = true;
    return;
  }

  let start = { x: butterfly.value.x, y: butterfly.value.y };
  const distance = Math.hypot(goal.x - start.x, goal.y - start.y);
  if (!butterflyVisible.value) start = { x: goal.x - 88, y: goal.y + 46 };
  else if (distance > 280) {
    const scale = 280 / distance;
    start = { x: goal.x - (goal.x - start.x) * scale, y: goal.y - (goal.y - start.y) * scale };
  }
  const jitter = Math.sin((props.step + 1) * 12.9898) * 16;
  const loopRadius = 10;
  const loopX = goal.x - loopRadius;
  const controlX = start.x + (loopX - start.x) * 0.5;
  const path = [
    `M ${start.x.toFixed(1)} ${start.y.toFixed(1)}`,
    `C ${(controlX - jitter).toFixed(1)} ${(start.y - 22 - jitter / 3).toFixed(1)}, ${(controlX + jitter).toFixed(1)} ${(goal.y + 24 + jitter / 3).toFixed(1)}, ${loopX.toFixed(1)} ${goal.y.toFixed(1)}`,
    `C ${(goal.x - 10).toFixed(1)} ${(goal.y - 9).toFixed(1)}, ${(goal.x - 2).toFixed(1)} ${(goal.y - 13).toFixed(1)}, ${(goal.x + 6).toFixed(1)} ${(goal.y - 8).toFixed(1)}`,
    `C ${(goal.x + 15).toFixed(1)} ${(goal.y - 2).toFixed(1)}, ${(goal.x + 12).toFixed(1)} ${(goal.y + 9).toFixed(1)}, ${(goal.x + 3).toFixed(1)} ${(goal.y + 11).toFixed(1)}`,
    `C ${(goal.x - 7).toFixed(1)} ${(goal.y + 13).toFixed(1)}, ${(goal.x - 13).toFixed(1)} ${(goal.y + 6).toFixed(1)}, ${loopX.toFixed(1)} ${goal.y.toFixed(1)}`,
    `Q ${(goal.x - 4).toFixed(1)} ${(goal.y - 1).toFixed(1)}, ${goal.x.toFixed(1)} ${goal.y.toFixed(1)}`,
  ].join(" ");
  flightPath.value = path;
  butterfly.value = { ...start, angle: 0, resting: false };
  butterflyVisible.value = true;
  await nextTick();
  if (sequence !== flightSequence || !flightPathRef.value) return;
  const svgPath = flightPathRef.value;
  const total = svgPath.getTotalLength();
  const startedAt = performance.now();
  const duration = 1350;
  const animate = (now: number) => {
    if (sequence !== flightSequence) return;
    const raw = clamp((now - startedAt) / duration, 0, 1);
    const progress = 1 - Math.pow(1 - raw, 3);
    const distanceAlongPath = total * progress;
    const point = svgPath.getPointAtLength(distanceAlongPath);
    const neighbor = svgPath.getPointAtLength(Math.min(total, distanceAlongPath + 1.5));
    const angle = clamp(Math.atan2(neighbor.y - point.y, neighbor.x - point.x) * 180 / Math.PI, -42, 42);
    butterfly.value = { x: point.x, y: point.y, angle, resting: false };
    if (raw < 1) animationFrame = requestAnimationFrame(animate);
    else butterfly.value = { ...goal, angle: 0, resting: true };
  };
  animationFrame = requestAnimationFrame(animate);
}

function measure(animate = false) {
  if (!props.open || !current.value) return;
  viewport.value = { width: window.innerWidth, height: window.innerHeight };
  const element = document.querySelector<HTMLElement>(current.value.selector);
  observe(element);
  const rect = element?.getClientRects().length ? element.getBoundingClientRect() : null;
  if (!rect) {
    if (!targetRect.value) butterflyVisible.value = false;
    return;
  }
  targetRect.value = rect;
  const goal = butterflyGoal(rect);
  if (animate) void flyTo(goal);
  else if (butterfly.value.resting) butterfly.value = { ...goal, angle: 0, resting: true };
}

const spotlightStyle = computed(() => {
  const rect = targetRect.value;
  if (!rect) return undefined;
  const left = clamp(rect.left - 5, 6, window.innerWidth - 12);
  const top = clamp(rect.top - 5, 6, window.innerHeight - 12);
  return {
    left: `${left}px`, top: `${top}px`,
    width: `${Math.max(0, Math.min(window.innerWidth - left - 6, rect.width + 10))}px`,
    height: `${Math.max(0, Math.min(window.innerHeight - top - 6, rect.height + 10))}px`,
  };
});
const panelStyle = computed(() => {
  const rect = targetRect.value;
  const width = Math.min(390, window.innerWidth - 32);
  if (!rect) return { right: "16px", top: "78px", width: `${width}px` };
  const left = rect.left + rect.width / 2 > window.innerWidth / 2
    ? Math.max(16, rect.left - width - 18)
    : Math.min(window.innerWidth - width - 16, rect.right + 18);
  const top = Math.max(16, Math.min(window.innerHeight - 330, Math.max(76, rect.top)));
  return { left: `${left}px`, top: `${top}px`, width: `${width}px` };
});

watch(() => [props.open, props.step, current.value?.selector], async () => {
  observer?.disconnect(); observer = undefined; observedElement = null; targetRect.value = null;
  layoutObserver?.disconnect(); layoutObserver = undefined;
  cancelAnimationFrame(animationFrame); flightSequence++;
  if (!props.open) { butterflyVisible.value = false; flightPath.value = ""; return; }
  await nextTick();
  observeLayout();
  requestAnimationFrame(() => measure(true));
}, { immediate: true });
const onViewportChange = () => measure(false);
const onKeydown = (event: KeyboardEvent) => { if (props.open && event.key === "Escape") emit("dismiss"); };
window.addEventListener("resize", onViewportChange);
document.addEventListener("scroll", onViewportChange, true);
window.addEventListener("keydown", onKeydown);
onBeforeUnmount(() => {
  flightSequence++;
  cancelAnimationFrame(animationFrame);
  cancelAnimationFrame(layoutMeasureFrame);
  observer?.disconnect();
  layoutObserver?.disconnect();
  window.removeEventListener("resize", onViewportChange);
  document.removeEventListener("scroll", onViewportChange, true);
  window.removeEventListener("keydown", onKeydown);
});
const go = (offset: number) => emit("update:step", Math.min(props.steps.length - 1, Math.max(0, props.step + offset)));
</script>

<template>
  <div v-if="open && current" class="parallel-tutorial" aria-live="polite">
    <div class="parallel-tutorial__shade" aria-hidden="true"></div>
    <div v-if="targetRect" class="parallel-tutorial__spotlight" :style="spotlightStyle" aria-hidden="true"></div>
    <svg class="parallel-tutorial__trail" :viewBox="`0 0 ${viewport.width} ${viewport.height}`" aria-hidden="true">
      <path v-if="flightPath" ref="flightPathRef" :d="flightPath" :class="{ 'parallel-tutorial__trail-path--resting': butterfly.resting }" />
    </svg>
    <div v-if="butterflyVisible" class="parallel-tutorial__butterfly" :style="butterflyStyle" aria-hidden="true">
      <img :key="step" :src="butterflyGuide" alt="" :class="{ 'parallel-tutorial__butterfly-image--resting': butterfly.resting }" />
    </div>
    <aside class="parallel-tutorial__card" :style="panelStyle" role="dialog" aria-modal="false" :aria-label="t('rwTutorialAria')">
      <header><span>{{ t('rwTutorialProgress', { current: step + 1, total: steps.length }) }}</span><button type="button" :aria-label="t('rwTutorialDismissAria')" :title="t('rwTutorialDismiss')" @click="emit('dismiss')"><X :size="17" /></button></header>
      <div class="parallel-tutorial__progress" aria-hidden="true"><i :style="{ width: `${(step + 1) / steps.length * 100}%` }"></i></div>
      <h2>{{ t(current.titleKey) }}</h2>
      <p>{{ t(current.bodyKey) }}</p>
      <div class="parallel-tutorial__tip">{{ t(current.tipKey) }}</div>
      <footer>
        <button class="parallel-tutorial__reset" type="button" @click="emit('reset')"><RotateCcw :size="15" />{{ t('rwTutorialReset') }}</button>
        <span></span>
        <button type="button" :disabled="step === 0" @click="go(-1)"><ArrowLeft :size="15" />{{ t('rwTutorialPrevious') }}</button>
        <button v-if="step + 1 < steps.length" class="primary-button" type="button" @click="go(1)">{{ t('rwTutorialNext') }}<ArrowRight :size="15" /></button>
        <button v-else class="primary-button" type="button" @click="emit('close')">{{ t('rwTutorialFinish') }}</button>
      </footer>
    </aside>
  </div>
</template>

<style scoped>
.parallel-tutorial { position: fixed; z-index: 100; inset: 0; pointer-events: none; }
.parallel-tutorial__shade { position: absolute; inset: 0; background: rgb(26 46 29 / 12%); pointer-events: none; }
.parallel-tutorial__spotlight { position: fixed; z-index: 1; border: 2px solid var(--green-700); border-radius: 10px; box-shadow: 0 0 0 4px rgb(255 255 255 / 76%), 0 8px 26px rgb(18 46 24 / 16%); pointer-events: none; transition: inset 180ms ease, width 180ms ease, height 180ms ease; }
.parallel-tutorial__trail { position: fixed; z-index: 2; inset: 0; width: 100%; height: 100%; overflow: visible; pointer-events: none; }
.parallel-tutorial__trail path { fill: none; stroke: #d79636; stroke-width: 1.4; stroke-linecap: round; stroke-dasharray: 2.5 6; opacity: .62; transition: opacity 700ms ease; }
.parallel-tutorial__trail .parallel-tutorial__trail-path--resting { opacity: .16; }
.parallel-tutorial__butterfly { position: fixed; z-index: 3; top: 0; left: 0; width: 24px; height: 24px; pointer-events: none; transform-origin: 50% 50%; }
.parallel-tutorial__butterfly img { display: block; width: 24px; height: 24px; object-fit: contain; }
.parallel-tutorial__butterfly-image--resting { animation: parallel-tutorial-butterfly-settle 1.4s ease-in-out 2; }
.parallel-tutorial__card { position: fixed; z-index: 4; overflow: hidden; border: 1px solid #b8ceb9; border-radius: 12px; color: var(--ink-900); background: var(--surface-raised); box-shadow: 0 18px 50px rgb(24 40 28 / 20%); pointer-events: auto; }
.parallel-tutorial__card header { display: flex; height: 42px; align-items: center; justify-content: space-between; padding: 0 14px 0 17px; color: var(--green-900); background: var(--surface-green-soft); font-size: var(--jm-font-size-callout); font-weight: var(--jm-font-weight-semibold); }
.parallel-tutorial__card header button { display: grid; width: 28px; height: 28px; place-items: center; padding: 0; border: 0; border-radius: 6px; background: transparent; cursor: pointer; }
.parallel-tutorial__card header button:hover { background: var(--surface-hover); }
.parallel-tutorial__progress { height: 3px; background: var(--line); }
.parallel-tutorial__progress i { display: block; height: 100%; background: var(--green-700); transition: width 180ms ease; }
.parallel-tutorial__card h2 { margin: 18px 20px 8px; color: var(--ink-900); font-size: var(--jm-font-size-title-2); font-weight: var(--jm-font-weight-semibold); }
.parallel-tutorial__card p { min-height: 68px; margin: 0 20px 13px; color: var(--ink-700); font-size: var(--jm-font-size-body); line-height: 1.65; }
.parallel-tutorial__tip { margin: 0 20px 17px; padding: 9px 11px; border-left: 3px solid var(--green-700); border-radius: 0 6px 6px 0; color: var(--green-900); background: var(--surface-green-soft); font-size: var(--jm-font-size-callout); line-height: 1.5; }
.parallel-tutorial__card footer { display: grid; grid-template-columns: auto 1fr auto auto; gap: 8px; align-items: center; padding: 12px 14px; border-top: 1px solid var(--line); background: var(--surface-subtle); }
.parallel-tutorial__card footer button { display: inline-flex; height: 32px; align-items: center; justify-content: center; gap: 5px; padding: 0 11px; border: 1px solid var(--line); border-radius: 6px; background: var(--surface-raised); cursor: pointer; white-space: nowrap; }
.parallel-tutorial__card footer .primary-button { border-color: var(--green-700); color: white; background: var(--green-700); }
.parallel-tutorial__card footer .parallel-tutorial__reset { border-color: transparent; color: var(--ink-500); background: transparent; }
@keyframes parallel-tutorial-butterfly-settle { 0%, 100% { transform: translateY(0) rotate(0); } 45% { transform: translateY(-2px) rotate(-3deg); } 70% { transform: translateY(1px) rotate(2deg); } }
@media (max-width: 900px) { .parallel-tutorial__card { right: 16px !important; bottom: 72px; left: 16px !important; top: auto !important; width: auto !important; } }
@media (prefers-reduced-motion: reduce) { .parallel-tutorial__spotlight, .parallel-tutorial__progress i, .parallel-tutorial__trail path { transition: none; } .parallel-tutorial__butterfly-image--resting { animation: none; } }
</style>
