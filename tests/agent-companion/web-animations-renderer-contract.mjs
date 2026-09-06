import assert from "node:assert/strict";
import { build } from "esbuild";
import { fileURLToPath } from "node:url";

const source = fileURLToPath(new URL("../../apps/desktop/src/domain/companion/web-animations-renderer.ts", import.meta.url));
const bundled = await build({ entryPoints: [source], bundle: true, format: "esm", platform: "browser", target: "esnext", write: false });
const moduleUrl = `data:text/javascript;base64,${Buffer.from(bundled.outputFiles[0].text).toString("base64")}`;
const { WebAnimationsCompanionRenderer } = await import(moduleUrl);

const created = [];
const actor = (name) => ({
  animate(keyframes, options) {
    const animation = { name, keyframes, options, cancelled: false, cancel() { this.cancelled = true; } };
    created.push(animation);
    return animation;
  },
});
const actors = new Map(["cat", "dog", "plant", "butterfly"].map((name) => [name, actor(name)]));
const root = {
  querySelector(selector) {
    const actorName = /data-companion-actor="([^"]+)"/.exec(selector)?.[1];
    return actorName ? actors.get(actorName) ?? null : null;
  },
};

const renderer = new WebAnimationsCompanionRenderer();
renderer.mount(root);
renderer.update({ activity: "running" });
assert.equal(created.length, 2, "running decorates only static actor elements with bounded animations");
assert.ok(created.every((item) => item.options.iterations === 1), "renderer never creates an infinite animation loop");
assert.ok(created.every((item) => item.options.duration <= 920), "actor transforms remain time-bounded");
assert.ok(created.every((item) => !JSON.stringify(item.keyframes).includes("translateX(50")), "actor transforms remain spatially bounded");

renderer.update({ activity: "idle" });
const idlePlant = created.at(-1);
assert.equal(idlePlant.name, "plant", "idle gives the planted decision tree a finite sway");
assert.equal(idlePlant.options.duration, 3600, "idle plant sway has a bounded 3.6 second duration");
assert.equal(idlePlant.options.iterations, 1, "idle plant sway never loops indefinitely");
assert.ok(JSON.stringify(idlePlant.keyframes).includes("rotate("), "idle plant sway uses bounded rotation");

renderer.update({ activity: "succeeded" });
assert.ok(created.slice(-4, -1).every((item) => item.cancelled), "state transition cancels previous animations");
renderer.pet("butterfly");
assert.equal(created.at(-2).cancelled, true, "manual interaction cancels its predecessor before a new animation");
renderer.dispose();
assert.equal(created.at(-1).cancelled, true, "dispose cancels every owned animation");

console.log("web animations renderer bounded lifecycle: passed");
