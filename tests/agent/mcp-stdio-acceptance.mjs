import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";

const command = process.env.JUEMING_MCP_COMMAND;
const args = process.env.JUEMING_MCP_ARGS ? JSON.parse(process.env.JUEMING_MCP_ARGS) : [];
const toolMap = {
  describe: "jueming_describe",
  bind: "jueming_bind_session",
  context: "jueming_get_context",
  navigate: "jueming_navigate",
  reveal: "jueming_reveal",
  summary: "jueming_project_summary",
  segment: "jueming_get_segment",
  alignment: "jueming_get_alignment",
  search: "jueming_search",
  searchResults: "jueming_search_results",
  previewReplace: "jueming_preview_replace",
  listProposals: "jueming_list_proposals",
  rejectProposal: "jueming_reject_proposal",
  ...(process.env.JUEMING_MCP_TOOL_MAP ? JSON.parse(process.env.JUEMING_MCP_TOOL_MAP) : {}),
};

if (!command) {
  console.error("Set JUEMING_MCP_COMMAND and optional JUEMING_MCP_ARGS before running this acceptance test.");
  process.exit(2);
}
if (!Array.isArray(args)) throw new TypeError("JUEMING_MCP_ARGS must be a JSON array");

const child = spawn(command, args, { stdio: ["pipe", "pipe", "pipe"], windowsHide: true });
let buffer = "";
let nextId = 1;
const pending = new Map();
const stderr = [];
child.stderr.setEncoding("utf8");
child.stderr.on("data", (chunk) => stderr.push(chunk));
child.stdout.setEncoding("utf8");
child.stdout.on("data", (chunk) => {
  buffer += chunk;
  for (;;) {
    const newline = buffer.indexOf("\n");
    if (newline < 0) return;
    const line = buffer.slice(0, newline).trim();
    buffer = buffer.slice(newline + 1);
    if (!line) continue;
    const message = JSON.parse(line);
    if (message.id !== undefined && pending.has(message.id)) {
      const { resolve, reject } = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) reject(Object.assign(new Error(message.error.message), { rpc: message.error }));
      else resolve(message.result);
    }
  }
});
child.on("error", (error) => failAll(error));
child.on("exit", (code) => failAll(new Error(`MCP process exited ${code}; stderr: ${stderr.join("")}`)));

function failAll(error) {
  for (const { reject } of pending.values()) reject(error);
  pending.clear();
}
function request(method, params) {
  const id = nextId++;
  child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`);
  return new Promise((resolve, reject) => pending.set(id, { resolve, reject }));
}
function notify(method, params) {
  child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", method, params })}\n`);
}
async function callTool(name, arguments_) {
  return request("tools/call", { name, arguments: arguments_ });
}
function text(result) {
  return (result.content ?? []).filter((item) => item.type === "text").map((item) => item.text).join("\n");
}
function structured(result) {
  return result.structuredContent ?? result.structured_content ?? null;
}

try {
  const initialized = await request("initialize", {
    protocolVersion: "2025-06-18",
    capabilities: {},
    clientInfo: { name: "jueming-agent-qa", version: "0.1" },
  });
  assert.ok(initialized.protocolVersion, "initialize must negotiate an MCP protocol version");
  notify("notifications/initialized", {});

  const tools = await request("tools/list", {});
  const names = new Set((tools.tools ?? []).map((tool) => tool.name));
  for (const name of Object.values(toolMap)) assert.ok(names.has(name), `tools/list must advertise ${name}`);
  for (const nativeOnly of ["jueming_approve_proposal", "app.get_projection", "ui.publish_context", "ui.ack", "proposal.approve"]) {
    assert.ok(!names.has(nativeOnly), `tools/list must not expose native-only operation ${nativeOnly}`);
  }

  const description = await callTool(toolMap.describe, {});
  assert.ok(!description.isError, `app.describe failed: ${text(description)}`);

  if (process.env.JUEMING_MCP_REQUIRE_PROJECT === "1") {
    const bound = await callTool(toolMap.bind, {});
    assert.ok(!bound.isError, `jueming_bind_session failed: ${text(bound)}`);
    const bindingId = structured(bound)?.data?.binding_id;
    assert.match(bindingId ?? "", /^[0-9a-f-]{36}$/i, "bind result must include a UUID binding_id");

    const context = await callTool(toolMap.context, { binding_id: bindingId });
    assert.ok(!context.isError, `jueming_get_context failed: ${text(context)}`);
    const invalidRegex = await callTool(toolMap.search, { binding_id: bindingId, query: "[", regex: true });
    assert.ok(invalidRegex.isError || /invalid|regex/i.test(text(invalidRegex)), "invalid regex must be rejected or reported");
    const search = await callTool(toolMap.search, { binding_id: bindingId, query: "one", regex: false, page_size: 1 });
    assert.ok(!search.isError, `jueming_search failed: ${text(search)}`);
    const sessionId = structured(search)?.data?.session_id;
    assert.equal(typeof sessionId, "string", "jueming_search must return a stable search session ID");
    const searchResults = await callTool(toolMap.searchResults, { binding_id: bindingId, session_id: sessionId, page_size: 1 });
    assert.ok(!searchResults.isError, `jueming_search_results failed: ${text(searchResults)}`);
    const navigation = await callTool(toolMap.navigate, { binding_id: bindingId, tab: "search" });
    assert.ok(!navigation.isError, `jueming_navigate failed: ${text(navigation)}`);
    const reveal = await callTool(toolMap.reveal, { binding_id: bindingId, node_id: randomUUID() });
    assert.ok(!reveal.isError, `jueming_reveal failed: ${text(reveal)}`);
  }

  console.log("MCP initialize/tools/list/native-only exclusion acceptance: passed");
} finally {
  child.kill();
}
