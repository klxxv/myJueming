import assert from "node:assert/strict";
import { build } from "esbuild";
import { fileURLToPath } from "node:url";

const source = fileURLToPath(new URL("../../apps/desktop/src/domain/companion/controller.ts", import.meta.url));
const bundled = await build({ entryPoints: [source], bundle: true, format: "esm", platform: "browser", target: "esnext", write: false });
const moduleUrl = `data:text/javascript;base64,${Buffer.from(bundled.outputFiles[0].text).toString("base64")}`;
const { createCompanionController } = await import(moduleUrl);

const runtimeListeners = { reduced: null, visible: null, focused: null };
const removed = [];
const runtime = {
  systemReducedMotion: () => false,
  windowFocused: () => true,
  documentVisible: () => true,
  listenSystemReducedMotion(listener) { runtimeListeners.reduced = listener; return () => removed.push("reduced"); },
  listenVisibility(listener) { runtimeListeners.visible = listener; return () => removed.push("visible"); },
  listenWindowFocus(listener) { runtimeListeners.focused = listener; return () => removed.push("focused"); },
};
const rootListeners = new Map();
const root = {
  addEventListener(name, listener) { rootListeners.set(name, listener); },
  removeEventListener(name) { rootListeners.delete(name); },
};
const rendererLog = [];
const factory = () => ({
  mount() { rendererLog.push("mount"); },
  update(state) { rendererLog.push(`update:${state.activity}`); },
  cancel() { rendererLog.push("cancel"); },
  dispose() { rendererLog.push("dispose"); },
  pet(actor) { rendererLog.push(`pet:${actor}`); },
});
const events = [];
const controller = createCompanionController({ runtime, rendererFactory: factory, onEvent: (event) => events.push(event) });

controller.mount(root);
assert.deepEqual(rendererLog, [], "static default must not load a renderer");
assert.ok(rootListeners.has("click"), "one click listener supports mouse and keyboard button activation");
assert.equal(rootListeners.has("pointerup"), false, "pet interaction does not install a duplicate pointer listener");
rootListeners.get("click")({
  target: { closest: () => ({ dataset: { companionActor: "dog" } }) },
});
assert.deepEqual(events.at(-1), { kind: "manual_pet", actor: "dog" }, "click activation emits a manual pet event");
controller.update({ mode: "quiet", activity: "running", dirtyEditor: false });
controller.update({ mode: "hidden", activity: "awaiting_approval", dirtyEditor: false });
assert.deepEqual(rendererLog, [], "quiet and hidden modes must not instantiate animation");

controller.update({ mode: "animated", activity: "running", dirtyEditor: false });
assert.deepEqual(rendererLog, ["mount", "update:running"], "animated mode lazily mounts the renderer");
controller.update({ mode: "animated", activity: "awaiting_approval", dirtyEditor: false });
assert.deepEqual(rendererLog.slice(-2), ["cancel", "update:awaiting_approval"], "state changes cancel old animation before update");

controller.setDirtyEditor(true);
assert.equal(rendererLog.at(-1), "dispose", "dirty editing releases animation immediately");
controller.setDirtyEditor(false);
assert.deepEqual(rendererLog.slice(-2), ["mount", "update:awaiting_approval"]);
runtimeListeners.reduced(true);
assert.equal(rendererLog.at(-1), "dispose", "reduced motion releases animation");
runtimeListeners.reduced(false);
runtimeListeners.visible(false);
assert.equal(rendererLog.at(-1), "dispose", "background visibility releases animation");
runtimeListeners.visible(true);
runtimeListeners.focused(false);
assert.equal(rendererLog.at(-1), "dispose", "background window releases animation");

controller.pet("cat");
assert.deepEqual(events.at(-1), { kind: "manual_pet", actor: "cat" }, "manual pet is an explicit event even when static");
controller.dispose();
assert.equal(rootListeners.size, 0, "dispose removes actor listeners");
assert.deepEqual(removed.sort(), ["focused", "reduced", "visible"], "dispose removes runtime listeners");
assert.ok(events.some((event) => event.kind === "idle") && events.some((event) => event.kind === "running") && events.some((event) => event.kind === "awaiting_approval"), "activity events remain deterministic without waiting for animation");

let resolveLazyRenderer;
let lazyFactoryCalls = 0;
const lateRendererLog = [];
const lateRenderer = {
  mount() { lateRendererLog.push("mount"); },
  update() { lateRendererLog.push("update"); },
  cancel() { lateRendererLog.push("cancel"); },
  dispose() { lateRendererLog.push("dispose"); },
};
const lazyController = createCompanionController({
  runtime,
  rendererFactory: () => {
    lazyFactoryCalls += 1;
    return new Promise((resolve) => { resolveLazyRenderer = resolve; });
  },
});
lazyController.mount(root);
assert.equal(lazyFactoryCalls, 0, "static startup must not even request the lazy renderer module");
lazyController.update({ mode: "animated", activity: "idle", dirtyEditor: false });
assert.equal(lazyFactoryCalls, 1, "eligible animated mode requests the lazy renderer once");
lazyController.dispose();
resolveLazyRenderer(lateRenderer);
await new Promise((resolve) => setTimeout(resolve, 0));
assert.deepEqual(lateRendererLog, ["dispose"], "a renderer resolving after disposal must never mount or animate");

console.log("companion lazy lifecycle, cancellation, and manual interaction: passed");
