import assert from "node:assert/strict";
import { build } from "esbuild";
import { fileURLToPath } from "node:url";

const source = fileURLToPath(new URL("../../apps/desktop/src/domain/agent-client.ts", import.meta.url));
const bundled = await build({
  entryPoints: [source],
  bundle: true,
  format: "esm",
  platform: "browser",
  target: "esnext",
  write: false,
});
const moduleUrl = `data:text/javascript;base64,${Buffer.from(bundled.outputFiles[0].text).toString("base64")}`;
const { createAgentClient } = await import(moduleUrl);

let listener;
let disposed = false;
const calls = [];
const projections = [
  { request_id: "projection-50", data: {}, sequence: "50" },
  { request_id: "projection-52", data: {}, sequence: "52" },
];
const client = createAgentClient({
  available: () => true,
  async invoke(command) {
    calls.push(command);
    if (command === "agent_projection") return projections.shift();
    throw new Error(`unexpected command: ${command}`);
  },
  async listen(channel, next) {
    assert.equal(channel, "agent_subscribe");
    listener = next;
    calls.push("listen");
    return () => { disposed = true; };
  },
});

const updates = [];
const subscription = await client.subscribe((update) => updates.push(update));
assert.deepEqual(calls.slice(0, 2), ["listen", "agent_projection"], "listener registration precedes the projection watermark");

listener({ contract_version: "1.0", sequence: "50", kind: "context_changed", binding_id: null, origin: "native", payload: {} });
listener({ contract_version: "1.0", sequence: "51", kind: "context_changed", binding_id: null, origin: "external", payload: {} });
await new Promise((resolve) => setTimeout(resolve, 0));
assert.deepEqual(updates.map((update) => update.type), ["projection", "event"], "duplicate snapshot event is discarded");

listener({ contract_version: "1.0", sequence: "53", kind: "search_changed", binding_id: null, origin: "external", payload: {} });
await new Promise((resolve) => setTimeout(resolve, 0));
assert.deepEqual(updates.map((update) => update.type), ["projection", "event", "projection", "event"], "a gap resnapshots before applying the event");

subscription.dispose();
assert.equal(disposed, true, "disposing releases the native event listener");
listener({ contract_version: "1.0", sequence: "54", kind: "context_changed", binding_id: null, origin: "native", payload: {} });
await new Promise((resolve) => setTimeout(resolve, 0));
assert.equal(updates.length, 4, "disposed client ignores later events");

console.log("agent stream ordering, gap recovery, and disposal: passed");
