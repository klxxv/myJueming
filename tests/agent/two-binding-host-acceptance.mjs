import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const workspace = fileURLToPath(new URL("../..", import.meta.url));
const fixtureRoot = await mkdtemp(path.join(os.tmpdir(), "jueming-two-binding-qa-"));
const projectPath = path.join(fixtureRoot, "fixture.jm");
const token = randomUUID();

function timeout(promise, label, milliseconds = 30_000) {
  let timer;
  const expiry = new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error(`${label} timed out`)), milliseconds);
  });
  return Promise.race([promise, expiry]).finally(() => clearTimeout(timer));
}

function startFixture() {
  const child = spawn(
    "cargo",
    ["run", "--quiet", "-p", "jueming-agent-transport", "--example", "qa_fixture_server", "--", projectPath],
    { cwd: workspace, env: { ...process.env, JUEMING_AGENT_TOKEN: token }, stdio: ["pipe", "pipe", "pipe"], windowsHide: true },
  );
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  let stderr = "";
  let buffered = "";
  const records = [];
  const waiters = [];
  const finish = new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("exit", (code) => resolve({ code, stderr }));
  });
  const record = (value) => {
    records.push(value);
    for (const waiter of waiters.splice(0)) {
      if (waiter.predicate(value)) waiter.resolve(value);
      else waiters.push(waiter);
    }
  };
  child.stdout.on("data", (chunk) => {
    buffered += chunk;
    const lines = buffered.split("\n");
    buffered = lines.pop() ?? "";
    for (const line of lines) {
      if (!line.trim()) continue;
      try { record(JSON.parse(line)); } catch { /* Cargo protocol output is never expected here. */ }
    }
  });
  child.stderr.on("data", (chunk) => { stderr += chunk; });
  return {
    child,
    finish,
    waitFor(predicate) {
      const known = records.find(predicate);
      if (known) return Promise.resolve(known);
      return new Promise((resolve) => waiters.push({ predicate, resolve }));
    },
    command(value) { child.stdin.write(`${value}\n`); },
  };
}

async function bridge(endpoint, call) {
  const response = await fetch(`${endpoint}/v1/agent/call`, {
    method: "POST",
    headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json" },
    body: JSON.stringify(call),
  });
  return { status: response.status, body: await response.json() };
}

async function openSse(endpoint) {
  const controller = new AbortController();
  const response = await fetch(`${endpoint}/v1/ag-ui/events`, {
    headers: { Authorization: `Bearer ${token}` },
    signal: controller.signal,
  });
  assert.equal(response.status, 200, "two-binding test must observe the production AG-UI stream");
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  const events = [];
  const waiters = [];
  const read = (async () => {
    let pending = "";
    try {
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        pending += decoder.decode(value, { stream: true });
        for (;;) {
          const boundary = pending.indexOf("\n\n");
          if (boundary < 0) break;
          const frame = pending.slice(0, boundary);
          pending = pending.slice(boundary + 2);
          const data = frame.split("\n").find((line) => line.startsWith("data:"))?.slice(5).trim();
          if (!data) continue;
          const event = JSON.parse(data);
          if (event.name !== "jueming.app_event") continue;
          events.push(event.value);
          for (const waiter of waiters.splice(0)) {
            if (waiter.predicate(event.value)) waiter.resolve(event.value);
            else waiters.push(waiter);
          }
        }
      }
    } catch (error) {
      if (error.name !== "AbortError") throw error;
    }
  })();
  return {
    controller,
    read,
    waitFor(predicate) {
      const known = events.find(predicate);
      if (known) return Promise.resolve(known);
      return new Promise((resolve) => waiters.push({ predicate, resolve }));
    },
  };
}

let fixture;
let sse;
try {
  fixture = startFixture();
  const ready = await timeout(fixture.waitFor((value) => value.ready === true), "fixture readiness");
  const endpoint = ready.endpoint;
  sse = await openSse(endpoint);

  const bound = await bridge(endpoint, {
    request_id: randomUUID(), method: "app.bind_session", params: {}, binding_id: null,
  });
  assert.equal(bound.status, 200, "external client A must bind through the production bridge");
  const externalBinding = bound.body.data.binding_id;

  const navigate = async (requestId) => {
    const result = await bridge(endpoint, {
      request_id: requestId, method: "ui.navigate", params: { tab: "search" }, binding_id: externalBinding,
    });
    assert.equal(result.status, 200, "external client A navigation must be accepted");
    return result.body.data.operation_id;
  };
  const operationStatus = async (operationId) => {
    const result = await bridge(endpoint, {
      request_id: randomUUID(), method: "operation.get", params: { operation_id: operationId }, binding_id: externalBinding,
    });
    assert.equal(result.status, 200, "external client A must read its operation status");
    return result.body.data.status;
  };

  const firstRequest = randomUUID();
  const firstOperation = await navigate(firstRequest);
  const published = await timeout(
    sse.waitFor((event) => event.kind === "navigation_requested" && event.binding_id === externalBinding && event.payload.operation_id === firstOperation),
    "external A navigation event",
  );
  assert.equal(published.payload.request_id, firstRequest, "published navigation retains the exact external request ID");
  fixture.command(`ack ${firstOperation} ${firstRequest}`);
  const acknowledged = await timeout(fixture.waitFor((value) => value.qa === "native_ack"), "native B acknowledgement");
  assert.deepEqual(acknowledged, { qa: "native_ack", ok: true, status: "ui_applied" }, "native binding B must complete external A's exact operation");
  assert.equal(await operationStatus(firstOperation), "ui_applied");

  const cancelledRequest = randomUUID();
  const cancelledOperation = await navigate(cancelledRequest);
  const cancelled = await bridge(endpoint, {
    request_id: randomUUID(), method: "operation.cancel", params: { operation_id: cancelledOperation }, binding_id: externalBinding,
  });
  assert.equal(cancelled.status, 200, "external A may cancel its pending navigation");
  assert.equal(await operationStatus(cancelledOperation), "cancelled");
  fixture.command(`ack ${cancelledOperation} ${cancelledRequest}`);
  const lateAck = await timeout(fixture.waitFor((value) => value.qa === "native_ack" && value.ok === false), "late native acknowledgement");
  assert.equal(lateAck.code, "operation_terminal", "a late acknowledgement must not revive a cancelled action");
  assert.equal(await operationStatus(cancelledOperation), "cancelled", "late acknowledgement must preserve cancellation");

  const wrongProjectRequest = randomUUID();
  const wrongProjectOperation = await navigate(wrongProjectRequest);
  fixture.command(`wrong-project-ack ${wrongProjectOperation} ${wrongProjectRequest}`);
  const wrongProjectAck = await timeout(fixture.waitFor((value) => value.qa === "wrong_project_ack"), "wrong-project native acknowledgement");
  assert.equal(wrongProjectAck.ok, false, "a native binding created after project switch must not acknowledge an earlier project action");
  assert.ok(["operation_not_found", "operation_terminal", "stale_operation_epoch", "stale_binding", "stale_revision"].includes(wrongProjectAck.code), "wrong-project acknowledgement must be rejected by host lifecycle validation");

  console.log("two-binding external A / native B operation lifecycle: passed");
} finally {
  sse?.controller.abort();
  await sse?.read.catch(() => undefined);
  if (fixture) {
    fixture.command("shutdown");
    const stopped = await timeout(fixture.finish, "fixture shutdown", 10_000);
    assert.equal(stopped.code, 0, `fixture shutdown failed: ${stopped.stderr.slice(0, 600)}`);
  }
  await rm(fixtureRoot, { recursive: true, force: true });
}
