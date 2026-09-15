import assert from 'node:assert/strict';
import { computed, createRenderer, h, ref, watch } from 'vue';
import { useAlignedBlockLayout } from '../../apps/desktop/src/components/parallel-workspace/useAlignedBlockLayout';
import type { AlignmentBlockView } from '../../apps/desktop/src/domain/workspace-projection';

const frames = new Map<number, FrameRequestCallback>();
let frameId = 0;
globalThis.requestAnimationFrame = callback => { frames.set(++frameId, callback); return frameId; };
globalThis.cancelAnimationFrame = id => { frames.delete(id); };
const flush = () => { const pending = [...frames.values()]; frames.clear(); pending.forEach(callback => callback(0)); };
const rows = ref<AlignmentBlockView[]>(Array.from({ length: 10000 }, (_, index) => ({
  alignmentId: `alignment-${index}`, index, linked: true, sourceFragmented: false, targetFragmented: false,
  sourceSegments: [{ id: `source-${index}`, order: index, side: 'source', text: '原文' }],
  targetSegments: [{ id: `target-${index}`, order: index, side: 'target', text: 'Translation' }],
})));
// Mount the production composable so lifecycle cleanup is exercised, without a browser DOM.
const renderer = createRenderer<object, object>({
  createElement: () => ({}), createText: () => ({}), createComment: () => ({}),
  insert() {}, remove() {}, setText() {}, setElementText() {}, patchProp() {},
  parentNode: () => null, nextSibling: () => null,
});
let layout!: ReturnType<typeof useAlignedBlockLayout>;
let updates = 0;
const app = renderer.createApp({ setup() {
  layout = useAlignedBlockLayout({ rows: computed(() => rows.value), viewportWidth: ref(1200), scrollTop: ref(0), viewportHeight: ref(700) });
  watch(layout.positions, () => updates++, { flush: 'sync' });
  return () => h('div');
} });
app.mount({});
assert.ok(layout.visiblePositions.value.length < 30, 'large document stays windowed');
for (let index = 0; index < 20; index++) {
  layout.reportHeight(`alignment-${index}`, 'source', 100 + index);
  layout.reportHeight(`alignment-${index}`, 'target', 150 + index);
}
assert.equal(updates, 0);
assert.equal(frames.size, 1);
flush();
assert.equal(updates, 1, '40 measurements publish one layout and anchor update');
assert.equal(layout.positions.value[1].bandTop, 162);
assert.equal(layout.positions.value[19].bandHeight, 169);
layout.reportHeight('alignment-0', 'target', 150.1);
assert.equal(frames.size, 0, 'ignore subpixel noise');
layout.reportHeight('alignment-0', 'target', 900);
layout.resetMeasurements();
flush();
assert.notEqual(layout.positions.value[0].targetHeight, 900, 'reset discards stale reports');
layout.reportHeight('alignment-0', 'target', 800);
app.unmount();
assert.equal(frames.size, 0, 'unmount cancels scheduled work');
console.log('PASS: 10,000 blocks; windowing, batched measurement, reset and disposal');
