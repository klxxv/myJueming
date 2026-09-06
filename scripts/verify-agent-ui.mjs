import { spawn } from "node:child_process";
import { createRequire } from "node:module";
import { readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const workspace = fileURLToPath(new URL("..", import.meta.url));
const timeoutMs = 90_000;
const workspaceRequire = createRequire(path.join(workspace, "package.json"));
const esbuildPackage = workspaceRequire.resolve("esbuild/package.json");
if (!esbuildPackage.startsWith(path.join(workspace, "node_modules"))) {
  throw new Error(`esbuild must resolve from this workspace: ${esbuildPackage}`);
}

const productionContracts = [
  "tests/agent-context/run-agent-context-contract.mjs",
  "tests/agent-context/run-agent-workspace-wire-contract.mjs",
  "tests/agent-context/agent-client-stream-contract.mjs",
  "tests/agent-companion/companion-production-import-contract.mjs",
  "tests/agent-companion/web-animations-renderer-contract.mjs",
  "tests/agent-companion/extension-registry-contract.mjs",
  "tests/agent/agent-client-integration.mjs",
  "tests/agent/run-two-binding-workspace-ack.mjs",
  "tests/agent/agent-panel-proposal-availability.mjs",
];

async function productionSearchContracts(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const contracts = [];
  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      contracts.push(...await productionSearchContracts(entryPath));
      continue;
    }
    if (!entry.isFile() || !entry.name.endsWith(".mjs")) continue;
    const name = entry.name.toLowerCase();
    const isProductionSearchContract = name.includes("search")
      && (name.includes("production") || name.includes("component"))
      && (name.includes("contract") || name.includes("integration") || name.includes("acceptance"));
    if (isProductionSearchContract && !name.includes("mirror") && !name.includes("model")) {
      contracts.push(path.relative(workspace, entryPath));
    }
  }
  return contracts;
}

function runContract(contract) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [contract], {
      cwd: workspace,
      shell: false,
      stdio: "inherit",
      windowsHide: true,
    });
    const timer = setTimeout(() => {
      child.kill();
      reject(new Error(`${contract} timed out after ${timeoutMs}ms`));
    }, timeoutMs);
    child.once("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child.once("exit", (code, signal) => {
      clearTimeout(timer);
      if (code === 0) resolve();
      else reject(new Error(`${contract} failed (exit ${code ?? "none"}, signal ${signal ?? "none"})`));
    });
  });
}

const searchContracts = (await productionSearchContracts(path.join(workspace, "tests", "agent-context"))).sort();
for (const contract of [...productionContracts, ...searchContracts]) {
  console.log(`\n[agent-ui] ${contract}`);
  try {
    await runContract(contract);
  } catch (error) {
    console.error(`[agent-ui] failed: ${error.message}`);
    process.exit(1);
  }
}

console.log("\n[agent-ui] production contract suite: passed");
