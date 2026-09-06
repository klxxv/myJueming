import { build } from "esbuild";
import { fileURLToPath } from "node:url";
import path from "node:path";

const source = fileURLToPath(new URL("./agent-workspace-wire-contract.test.ts", import.meta.url));
const desktopRoot = fileURLToPath(new URL("../../apps/desktop/", import.meta.url));
const bundled = await build({
  entryPoints: [source],
  bundle: true,
  format: "esm",
  platform: "node",
  target: "node24",
  nodePaths: [path.join(desktopRoot, "node_modules")],
  write: false,
  plugins: [{
    name: "desktop-dependency-resolution",
    setup(build_) {
      build_.onResolve({ filter: /^pinia$/ }, () => ({ path: path.join(desktopRoot, "node_modules/pinia/dist/pinia.esm-browser.prod.js") }));
      build_.onResolve({ filter: /^vue$/ }, () => ({ path: path.join(desktopRoot, "node_modules/vue/index.mjs") }));
    },
  }],
});
const moduleUrl = `data:text/javascript;base64,${Buffer.from(bundled.outputFiles[0].text).toString("base64")}`;
await import(moduleUrl);
