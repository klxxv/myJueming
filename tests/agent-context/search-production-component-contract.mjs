import assert from "node:assert/strict";
import { build } from "esbuild";
import { createRequire } from "node:module";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const workspace = fileURLToPath(new URL("../..", import.meta.url));
const desktopConfig = path.join(workspace, "apps", "desktop", "vite.config.ts");
const desktopRequire = createRequire(desktopConfig);
const pluginRequire = createRequire(desktopRequire.resolve("@vitejs/plugin-vue"));
const { compileScript, compileTemplate, parse } = pluginRequire("@vue/compiler-sfc");
const entry = path.join(workspace, "tests", "agent-context", "search-production-component-contract-entry.ts");
const outputDirectory = await mkdtemp(path.join(os.tmpdir(), "jueming-search-component-qa-"));

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

function button(root, label) {
  const matched = walk(root, (node) => node.type === "button" && text(node) === label);
  assert.equal(matched.length, 1, `expected exactly one ${label} button`);
  return matched[0];
}

function createMemoryRenderer(createRenderer) {
  const element = (type) => {
    const listeners = new Map();
    const view = {
      setTimeout: globalThis.setTimeout.bind(globalThis),
      clearTimeout: globalThis.clearTimeout.bind(globalThis),
    };
    return {
      type,
      props: {},
      // A viewport surface for the production TanStack virtualizer. These are
      // host layout capabilities, not a replacement range calculation.
      offsetWidth: 1024,
      offsetHeight: 400,
      scrollTop: 0,
      scrollLeft: 0,
      ownerDocument: { defaultView: view },
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
    setElementText(node, value) { node.children = [{ type: "#text", text: value, props: {}, children: [], parent: node }]; },
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

const nativeResult = {
  id: "rust-hit-7",
  label: "S-7",
  sourceId: "source-stable-7",
  targetId: "target-stable-7",
  sourceText: "Rust indexed source hit",
  targetText: "Rust indexed target hit",
  alignmentId: "alignment-stable-7",
};

const nativePreview = {
  options: { query: "server query", side: "both", regex: true, caseSensitive: false },
  replacement: "server replacement",
  resultIds: ["source-stable-7", "target-stable-7"],
  source: [{ resultId: "source-stable-7", before: "Rust source before", after: "Rust source after" }],
  target: [{ resultId: "target-stable-7", before: "Rust target before", after: "Rust target after" }],
};

function stateFor(bundle, overrides = {}) {
  return bundle.reactive({
    results: [nativeResult],
    query: "(",
    side: "both",
    regex: true,
    caseSensitive: false,
    replacement: "live replacement",
    projectLabel: "QA project",
    authoritativeResults: true,
    nativePreview: null,
    previewLoading: false,
    previewError: null,
    ...overrides,
  });
}

function mount(bundle, renderer, state, emitted) {
  const root = { type: "root", props: {}, children: [] };
  renderer.createApp({
    render: () => bundle.h(bundle.SearchReplaceWorkspace, {
      ...state,
      onApplyReplace: (preview) => emitted.push(preview),
    }),
  }).mount(root);
  return root;
}

try {
  await build({
    entryPoints: { search: entry },
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
          contents: compileVue(await readFile(args.path, "utf8"), args.path),
          loader: "ts",
        }));
      },
    }],
  });
  await writeFile(path.join(outputDirectory, "package.json"), '{"type":"commonjs"}');
  const bundle = createRequire(import.meta.url)(path.join(outputDirectory, "search.js"));
  const renderer = createMemoryRenderer(bundle.createRenderer);

  {
    const root = mount(bundle, renderer, stateFor(bundle), []);
    await bundle.nextTick();
    assert.match(text(root), /找到 1 条结果/, "authoritative native results must remain visible when the browser cannot compile the supplied regex");
    assert.match(text(root), /Rust indexed source hit/, "the Rust search result is rendered instead of a JavaScript-filtered empty list");
  }

  {
    const emitted = [];
    const state = stateFor(bundle);
    const root = mount(bundle, renderer, state, emitted);
    button(root, "预览替换").props.onClick({});
    await bundle.nextTick();
    assert.equal(button(root, "确认应用").props.disabled, true, "a null native preview cannot be applied");

    state.previewLoading = true;
    await bundle.nextTick();
    assert.match(text(root), /正在生成替换预览/, "native preview loading state is rendered");
    assert.equal(button(root, "确认应用").props.disabled, true, "a loading native preview cannot be applied");

    state.previewLoading = false;
    state.previewError = "Rust preview failed";
    await bundle.nextTick();
    assert.match(text(root), /Rust preview failed/, "native preview errors are rendered");
    assert.equal(button(root, "确认应用").props.disabled, true, "an errored native preview cannot be applied");

    state.previewError = null;
    state.nativePreview = { ...nativePreview, resultIds: [], source: [], target: [] };
    await bundle.nextTick();
    assert.equal(button(root, "确认应用").props.disabled, true, "a zero-result native preview cannot be applied");
    assert.deepEqual(emitted, [], "disabled preview confirmations cannot emit an apply request");
  }

  {
    const emitted = [];
    const state = stateFor(bundle, { nativePreview });
    const root = mount(bundle, renderer, state, emitted);
    button(root, "预览替换").props.onClick({});
    await bundle.nextTick();
    assert.match(text(root), /Rust source before/, "native source before text is shown in the real preview dialog");
    assert.match(text(root), /Rust source after/, "native source after text is shown in the real preview dialog");
    assert.match(text(root), /Rust target before/, "native target before text is shown in the real preview dialog");
    assert.match(text(root), /Rust target after/, "native target after text is shown in the real preview dialog");
    assert.equal(button(root, "确认应用").props.disabled, false, "a non-empty ready native preview can be applied");

    state.query = "changed live query";
    state.replacement = "changed live replacement";
    await bundle.nextTick();
    button(root, "确认应用").props.onClick({});
    assert.equal(emitted.length, 1, "confirming a native preview emits exactly one application request");
    assert.deepEqual(emitted[0], nativePreview, "application emits the displayed native snapshot content, never live query or replacement props");
    assert.notEqual(emitted[0].options.query, state.query, "application must not substitute the changed live query");
    assert.notEqual(emitted[0].replacement, state.replacement, "application must not substitute the changed live replacement");
  }

  console.log("production SearchReplaceWorkspace authoritative native search and preview contract: passed");
} finally {
  await rm(outputDirectory, { recursive: true, force: true });
}
