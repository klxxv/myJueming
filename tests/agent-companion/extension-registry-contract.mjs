import assert from "node:assert/strict";
import { build } from "esbuild";
import { fileURLToPath } from "node:url";

const source = fileURLToPath(new URL("../../apps/desktop/src/domain/extensions/registry.ts", import.meta.url));
const bundled = await build({ entryPoints: [source], bundle: true, format: "esm", platform: "browser", target: "esnext", write: false });
const moduleUrl = `data:text/javascript;base64,${Buffer.from(bundled.outputFiles[0].text).toString("base64")}`;
const { createExtensionCapabilityRegistry } = await import(moduleUrl);

const registry = createExtensionCapabilityRegistry();
let notifications = 0;
const unsubscribe = registry.subscribe(() => { notifications += 1; });
assert.equal(registry.activeSubscriberCount, 1);

const first = registry.beginGeneration("pipeline-ui");
first.register({
  navigation: [{ id: "pipeline", label: "Pipeline", location: "primary", order: 40 }],
  tools: [{ id: "pipeline.inspect", label: "Inspect", access: "read" }],
});
assert.equal(registry.snapshot().navigation[0].id, "pipeline");
assert.throws(() => first.register({ navigation: [
  { id: "preview", label: "Preview", location: "primary", order: 41 },
  { id: "pipeline", label: "Duplicate", location: "primary", order: 42 },
] }), /already registered/);
assert.deepEqual(registry.snapshot().navigation.map((item) => item.id), ["pipeline"], "failed registrations are atomic");

const replacement = registry.beginGeneration("pipeline-ui");
assert.deepEqual(registry.snapshot().navigation, [], "a new generation removes old capability descriptors");
replacement.register({ context: [{ id: "pipeline-selection", label: "Pipeline selection", scope: "project", fields: ["project_id", "revision_id"] }] });
first.dispose();
assert.equal(registry.snapshot().context.length, 1, "disposing an old generation cannot remove replacement entries");

unsubscribe();
const beforeDispose = notifications;
registry.dispose();
assert.equal(registry.activeSubscriberCount, 0, "registry disposal clears subscriptions");
assert.equal(notifications, beforeDispose, "disposed subscriber receives no cleanup callback");
assert.throws(() => registry.beginGeneration("later"), /disposed/);

console.log("extension generation reload, atomic registration, and subscription cleanup: passed");
