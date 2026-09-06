import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";
import { build } from "esbuild";

const source = fileURLToPath(new URL("../../apps/desktop/src/domain/companion/controller.ts", import.meta.url));
const outputDirectory = await mkdtemp(path.join(os.tmpdir(), "jueming-companion-production-"));
const priorWindow = globalThis.window;
const priorDocument = globalThis.document;
const priorElement = globalThis.Element;

try {
  const result = await build({
    entryPoints: { controller: source },
    bundle: true,
    splitting: true,
    format: "esm",
    platform: "browser",
    target: "es2020",
    minify: true,
    metafile: true,
    outdir: outputDirectory,
  });
  assert.ok(Object.keys(result.metafile.outputs).length >= 2, "production bundle keeps renderer in a dynamic chunk");
  await writeFile(path.join(outputDirectory, "package.json"), "{\"type\":\"module\"}");

  const listeners = new Map();
  globalThis.window = {
    matchMedia: () => null,
    addEventListener: (name, listener) => listeners.set(name, listener),
    removeEventListener: (name) => listeners.delete(name),
  };
  globalThis.document = {
    hasFocus: () => true,
    visibilityState: "visible",
    addEventListener: (name, listener) => listeners.set(`document:${name}`, listener),
    removeEventListener: (name) => listeners.delete(`document:${name}`),
  };
  globalThis.Element = class {};

  const animations = [];
  const actor = {
    animate(keyframes, options) {
      const animation = { keyframes, options, cancelled: false, cancel() { this.cancelled = true; } };
      animations.push(animation);
      return animation;
    },
  };
  const root = {
    addEventListener: (name, listener) => listeners.set(`root:${name}`, listener),
    removeEventListener: (name) => listeners.delete(`root:${name}`),
    querySelector: () => actor,
  };

  const { createCompanionController } = await import(pathToFileURL(path.join(outputDirectory, "controller.js")).href);
  const controller = createCompanionController();
  controller.mount(root);
  assert.equal(animations.length, 0, "static startup imports no renderer and creates no animation");
  controller.update({ mode: "animated", activity: "running", dirtyEditor: false });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.ok(animations.length > 0, "eligible animated mode dynamically imports and mounts the production renderer chunk");
  controller.dispose();
  assert.ok(animations.every((animation) => animation.cancelled), "production renderer disposal cancels all animations");
} finally {
  if (priorWindow === undefined) delete globalThis.window; else globalThis.window = priorWindow;
  if (priorDocument === undefined) delete globalThis.document; else globalThis.document = priorDocument;
  if (priorElement === undefined) delete globalThis.Element; else globalThis.Element = priorElement;
  await rm(outputDirectory, { recursive: true, force: true });
}

console.log("production companion dynamic import and disposal: passed");
