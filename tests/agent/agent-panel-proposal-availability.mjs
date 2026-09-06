import assert from "node:assert/strict";
import { build } from "esbuild";
import { createRequire } from "node:module";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const workspace = fileURLToPath(new URL("../..", import.meta.url));
const desktopConfig = path.join(workspace, "apps", "desktop", "vite.config.ts");
const desktopRequire = createRequire(desktopConfig);
const pluginRequire = createRequire(desktopRequire.resolve("@vitejs/plugin-vue"));
const { compileScript, compileTemplate, parse } = pluginRequire("@vue/compiler-sfc");
const entry = path.join(workspace, "tests", "agent", "agent-panel-proposal-availability-entry.ts");
const outputDirectory = await mkdtemp(path.join(os.tmpdir(), "jueming-agent-panel-qa-"));

function compileVue(source, filename) {
  const { descriptor, errors } = parse(source, { filename });
  assert.deepEqual(errors, [], `SFC parse failed for ${filename}`);
  const id = `qa-${Buffer.from(filename).toString("base64url")}`;
  const script = compileScript(descriptor, { id, genDefaultAs: "__sfc__" });
  const template = compileTemplate({
    source: descriptor.template?.content ?? "",
    filename,
    id,
    compilerOptions: { bindingMetadata: script.bindings },
  });
  assert.deepEqual(template.errors, [], `SFC template compile failed for ${filename}`);
  assert.match(script.content, /const __sfc__/, `unexpected SFC script output for ${filename}`);
  return `${script.content}\n${template.code.replace("export function render", "function render")}\n__sfc__.render = render;\nexport default __sfc__;`;
}

function text(node) {
  if (node.type === "#text") return node.text;
  return node.children.map(text).join("");
}

function walk(node, predicate, found = []) {
  if (predicate(node)) found.push(node);
  for (const child of node.children) walk(child, predicate, found);
  return found;
}

function createMemoryRenderer(createRenderer) {
  const element = (type) => {
    const listeners = new Map();
    return {
      type,
      props: {},
      style: {
        display: "",
        setProperty(name, value) { this[name] = value; },
        removeProperty(name) { delete this[name]; },
      },
      children: [],
      parent: null,
      addEventListener(name, listener) { listeners.set(name, listener); },
      removeEventListener(name) { listeners.delete(name); },
    };
  };
  return createRenderer({
    createElement: element,
    createText(value) { return { type: "#text", text: value, props: {}, children: [], parent: null }; },
    createComment(value) { return { type: "#comment", text: value, props: {}, children: [], parent: null }; },
    setText(node, value) { node.text = value; },
    setElementText(node, value) {
      node.children = [{ type: "#text", text: value, props: {}, children: [], parent: node }];
    },
    parentNode(node) { return node.parent; },
    nextSibling(node) {
      const siblings = node.parent?.children ?? [];
      return siblings[siblings.indexOf(node) + 1] ?? null;
    },
    insert(node, parent, anchor) {
      node.parent = parent;
      const index = anchor ? parent.children.indexOf(anchor) : -1;
      if (index >= 0) parent.children.splice(index, 0, node);
      else parent.children.push(node);
    },
    remove(node) {
      const index = node.parent?.children.indexOf(node) ?? -1;
      if (index >= 0) node.parent.children.splice(index, 1);
    },
    patchProp(node, key, _previous, value) { node.props[key] = value; },
  });
}

function proposalPanel(h, GlobalSidePanel, AgentPanel, reviewing, emitted) {
  return h(GlobalSidePanel, {
    activeTab: "agent",
    projectLabel: "QA project",
    annotationCount: 0,
    pendingCount: 1,
  }, {
    agent: () => h(AgentPanel, {
      tabLabel: "审阅",
      selectedText: "",
      connected: true,
      runtimeAvailable: true,
      running: true,
      reviewing,
      showContext: false,
      messages: [],
      activity: [],
      proposals: [{
        id: "proposal-qa",
        title: "QA 审核修改",
        status: "pending",
        revision: "7",
        changes: [{ id: "segment-qa", before: "before", after: "after" }],
      }],
      error: null,
      onApprove: (id) => emitted.push(["approve", id]),
      onReject: (id) => emitted.push(["reject", id]),
    }),
  });
}

try {
  const result = await build({
    entryPoints: { panel: entry },
    bundle: true,
    format: "cjs",
    platform: "node",
    target: "node24",
    nodePaths: [path.join(workspace, "apps", "desktop", "node_modules")],
    outdir: outputDirectory,
    plugins: [{
      name: "compile-real-vue-sfc",
      setup(build_) {
        build_.onResolve({ filter: /^vue$/ }, () => ({ path: desktopRequire.resolve("vue") }));
        build_.onLoad({ filter: /\.vue$/ }, async (args) => ({
          contents: compileVue(await (await import("node:fs/promises")).readFile(args.path, "utf8"), args.path),
          loader: "ts",
        }));
      },
    }],
  });
  assert.ok(Object.keys(result.metafile ?? {}).length >= 0);
  await writeFile(path.join(outputDirectory, "package.json"), '{"type":"commonjs"}');
  const bundle = createRequire(import.meta.url)(path.join(outputDirectory, "panel.js"));
  const renderer = createMemoryRenderer(bundle.createRenderer);

  const enabledRoot = { type: "root", props: {}, children: [] };
  const enabledEvents = [];
  renderer.createApp({ render: () => proposalPanel(bundle.h, bundle.GlobalSidePanel, bundle.AgentPanel, false, enabledEvents) }).mount(enabledRoot);
  const enabledButtons = walk(enabledRoot, (node) => node.type === "button" && ["拒绝修改", "批准并应用"].includes(text(node)));
  assert.equal(enabledButtons.length, 2, "global right panel renders proposal actions even without a CompanionGarden instance");
  assert.ok(enabledButtons.every((button) => !button.props.disabled), "runtime running alone must leave proposal actions enabled");
  enabledButtons.find((button) => text(button) === "批准并应用").props.onClick({});
  enabledButtons.find((button) => text(button) === "拒绝修改").props.onClick({});
  assert.deepEqual(enabledEvents, [["approve", "proposal-qa"], ["reject", "proposal-qa"]], "enabled right-panel actions emit real AgentPanel approval and rejection events");

  const reviewingRoot = { type: "root", props: {}, children: [] };
  renderer.createApp({ render: () => proposalPanel(bundle.h, bundle.GlobalSidePanel, bundle.AgentPanel, true, []) }).mount(reviewingRoot);
  const reviewingButtons = walk(reviewingRoot, (node) => node.type === "button" && ["拒绝修改", "提交中…"].includes(text(node)));
  assert.equal(reviewingButtons.length, 2, "reviewing state still renders both proposal actions");
  assert.ok(reviewingButtons.every((button) => button.props.disabled === true), "only an active proposal review disables approval and rejection");

  console.log("production AgentPanel proposal availability and hidden-garden right-panel regression: passed");
} finally {
  await rm(outputDirectory, { recursive: true, force: true });
}
