import assert from "node:assert/strict";
import { build } from "esbuild";
import { fileURLToPath } from "node:url";

const source = fileURLToPath(new URL("../../apps/desktop/src/domain/agent-client.ts", import.meta.url));
const bundled = await build({
  entryPoints: [source],
  bundle: true,
  format: "esm",
  platform: "browser",
  write: false,
  target: "esnext",
});
const moduleUrl = `data:text/javascript;base64,${Buffer.from(bundled.outputFiles[0].text).toString("base64")}`;
const { AgentClientUnavailableError, createAgentClient } = await import(moduleUrl);

let eventListener;
let unlistened = false;
const calls = [];
const projections = [
  { request_id: "projection-1", data: {}, sequence: "41" },
  { request_id: "projection-2", data: {}, sequence: "43" },
];
const bridge = {
  available: () => true,
  async invoke(command, args) {
    calls.push({ command, args });
    if (command === "agent_projection") return projections.shift();
    return { request_id: args.call.request_id, data: { ok: true }, sequence: "41" };
  },
  async listen(channel, listener) {
    calls.push({ command: `listen:${channel}` });
    eventListener = listener;
    return () => { unlistened = true; };
  },
};

const client = createAgentClient(bridge);
const updates = [];
const subscription = await client.subscribe((update) => updates.push(update));
assert.equal(calls[0].command, "listen:agent_subscribe", "subscribe must attach before loading projection");
assert.equal(calls[1].command, "agent_projection");
assert.equal(updates[0].type, "projection");

eventListener({ contract_version: "1.0", sequence: "41", kind: "context_changed", binding_id: null, origin: "native", payload: {} });
eventListener({ contract_version: "1.0", sequence: "42", kind: "context_changed", binding_id: null, origin: "native", payload: {} });
await new Promise((resolve) => setTimeout(resolve, 0));
assert.deepEqual(updates.map((update) => update.type), ["projection", "event"], "snapshot duplicates must be discarded");

eventListener({ contract_version: "1.0", sequence: "44", kind: "search_changed", binding_id: null, origin: "native", payload: {} });
await new Promise((resolve) => setTimeout(resolve, 0));
assert.deepEqual(updates.map((update) => update.type), ["projection", "event", "projection", "event"], "event gaps must resnapshot before delivery");

await client.call("ui.get_context", {}, "binding-1");
const call = calls.at(-1);
assert.equal(call.command, "agent_call");
assert.equal(call.args.call.method, "ui.get_context");
assert.equal(call.args.call.binding_id, "binding-1");
assert.match(call.args.call.request_id, /^[0-9a-f-]{36}$/i);

subscription.dispose();
assert.equal(unlistened, true, "dispose must release the native listener");
eventListener({ contract_version: "1.0", sequence: "45", kind: "search_changed", binding_id: null, origin: "native", payload: {} });
await new Promise((resolve) => setTimeout(resolve, 0));
assert.equal(updates.length, 4, "disposed subscriptions must ignore later events");

const unavailable = createAgentClient({ available: () => false, invoke: bridge.invoke, listen: bridge.listen });
await assert.rejects(unavailable.call("app.describe"), AgentClientUnavailableError);
console.log("agent client subscription, resync, binding, and disposal: passed");
