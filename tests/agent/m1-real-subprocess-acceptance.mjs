import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const workspace = fileURLToPath(new URL("../..", import.meta.url));
const fixtureRoot = await mkdtemp(path.join(os.tmpdir(), "jueming-m1-qa-"));
const projectPath = path.join(fixtureRoot, "fixture.jm");
const token = randomUUID();

function timeout(promise, label, milliseconds = 60_000) {
  let timer;
  const expiry = new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error(`${label} timed out`)), milliseconds);
  });
  return Promise.race([promise, expiry]).finally(() => clearTimeout(timer));
}

function spawnCaptured(command, args, options) {
  const child = spawn(command, args, { windowsHide: true, ...options });
  let stdout = "";
  let stderr = "";
  child.stdout?.setEncoding("utf8");
  child.stderr?.setEncoding("utf8");
  child.stdout?.on("data", (chunk) => { stdout += chunk; });
  child.stderr?.on("data", (chunk) => { stderr += chunk; });
  const finished = new Promise((resolve, reject) => {
    child.on("error", reject);
    child.on("exit", (code) => resolve({ code, stdout, stderr }));
  });
  return { child, finished, read: () => ({ stdout, stderr }) };
}

async function waitForFixture(fixture) {
  return timeout(new Promise((resolve, reject) => {
    let buffered = "";
    const onData = (chunk) => {
      buffered += chunk;
      const lines = buffered.split("\n");
      buffered = lines.pop() ?? "";
      for (const line of lines) {
        if (!line.trim()) continue;
        try {
          const value = JSON.parse(line);
          if (value.ready === true && typeof value.endpoint === "string") {
            fixture.child.stdout.off("data", onData);
            resolve(value.endpoint);
          }
        } catch {
          // Cargo diagnostics belong on stderr. Ignore non-JSON stdout before readiness.
        }
      }
    };
    fixture.child.stdout.on("data", onData);
    fixture.finished.then(({ code, stderr }) => reject(new Error(`fixture exited ${code}: ${stderr.slice(0, 600)}`))).catch(reject);
  }), "fixture startup");
}

async function openSse(endpoint) {
  const controller = new AbortController();
  const response = await fetch(`${endpoint}/v1/ag-ui/events`, {
    headers: { Authorization: `Bearer ${token}` },
    signal: controller.signal,
  });
  assert.equal(response.status, 200, "AG-UI SSE must require and accept the fixture bearer token");
  assert.ok(response.body, "AG-UI SSE response must have a body");
  const events = [];
  const waiters = [];
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let pending = "";
  const read = (async () => {
    try {
      for (;;) {
        const { done, value } = await reader.read();
        if (done) return;
        pending += decoder.decode(value, { stream: true });
        for (;;) {
          const boundary = pending.indexOf("\n\n");
          if (boundary < 0) break;
          const frame = pending.slice(0, boundary);
          pending = pending.slice(boundary + 2);
          const data = frame.split("\n").find((line) => line.startsWith("data:"))?.slice(5).trim();
          if (!data) continue;
          const event = JSON.parse(data);
          if (event.name === "jueming.app_event") {
            events.push(event.value);
            for (const waiter of waiters.splice(0)) {
              if (waiter.kinds.every((kind) => events.some((entry) => entry.kind === kind))) waiter.resolve();
              else waiters.push(waiter);
            }
          }
        }
      }
    } catch (error) {
      if (error.name !== "AbortError") throw error;
    }
  })();
  return {
    events,
    controller,
    read,
    waitForKinds(kinds) {
      if (kinds.every((kind) => events.some((event) => event.kind === kind))) return Promise.resolve();
      return new Promise((resolve) => waiters.push({ kinds, resolve }));
    },
  };
}

try {
  const fixture = spawnCaptured(
    "cargo",
    ["run", "--quiet", "-p", "jueming-agent-transport", "--example", "qa_fixture_server", "--", projectPath],
    { cwd: workspace, env: { ...process.env, JUEMING_AGENT_TOKEN: token }, stdio: ["pipe", "pipe", "pipe"] },
  );
  const endpoint = await waitForFixture(fixture);
  const sse = await openSse(endpoint);
  const mcp = spawnCaptured(
    process.execPath,
    [path.join(workspace, "tests", "agent", "mcp-stdio-acceptance.mjs")],
    {
      cwd: workspace,
      env: {
        ...process.env,
        JUEMING_AGENT_ENDPOINT: endpoint,
        JUEMING_AGENT_TOKEN: token,
        JUEMING_MCP_COMMAND: "cargo",
        JUEMING_MCP_ARGS: JSON.stringify(["run", "--quiet", "-p", "jueming-mcp", "--"]),
        JUEMING_MCP_REQUIRE_PROJECT: "1",
      },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  const mcpResult = await timeout(mcp.finished, "MCP stdio acceptance");
  assert.equal(mcpResult.code, 0, `MCP stdio acceptance failed: ${mcpResult.stderr.slice(0, 600)}`);

  await timeout(
    sse.waitForKinds(["search_changed", "navigation_requested", "reveal_requested"]),
    "AG-UI event delivery",
    10_000,
  );
  sse.controller.abort();
  await sse.read;

  fixture.child.stdin.write("shutdown\n");
  const fixtureResult = await timeout(fixture.finished, "fixture shutdown", 10_000);
  assert.equal(fixtureResult.code, 0, `fixture shutdown failed: ${fixtureResult.stderr.slice(0, 600)}`);
  console.log("M1 real subprocess MCP tools, native-only exclusion, search, and AG-UI SSE: passed");
} finally {
  await rm(fixtureRoot, { recursive: true, force: true });
}
