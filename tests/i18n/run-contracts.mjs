import { build } from "esbuild";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";

const root = fileURLToPath(new URL("../../", import.meta.url));
const desktopRequire = createRequire(path.join(root, "apps/desktop/package.json"));
const { parse, compileScript } = desktopRequire("vue/compiler-sfc");
const result = await build({
  entryPoints: [path.join(root, "tests/i18n/i18n-contracts.ts")],
  bundle: true,
  format: "esm",
  platform: "node",
  target: "node24",
  write: false,
  nodePaths: [path.join(root, "apps/desktop/node_modules")],
  define: { __REPOSITORY_ROOT__: JSON.stringify(root), "process.env.NODE_ENV": '"production"' },
  plugins: [{
    name: "real-vue-settings-render",
    setup(build) {
      build.onLoad({ filter: /\.(svg|png|gif)$/ }, ({ path: filename }) => ({ contents: `export default ${JSON.stringify(filename)}`, loader: "js" }));
      // Share one installed Vue runtime between the renderer, i18n and SFCs.
      build.onResolve({ filter: /^[^./]/ }, ({ path: specifier }) => {
        if (specifier.startsWith("node:")) return { path: specifier, external: true };
        return { path: pathToFileURL(desktopRequire.resolve(specifier)).href, external: true };
      });
      build.onLoad({ filter: /\.vue$/ }, async ({ path: filename }) => {
        const { descriptor } = parse(await readFile(filename, "utf8"), { filename });
        const script = compileScript(descriptor, { id: filename, inlineTemplate: true });
        return { contents: script.content, loader: "ts", resolveDir: path.dirname(filename) };
      });
    },
  }],
});

try {
  await import(`data:text/javascript;base64,${Buffer.from(result.outputFiles[0].text).toString("base64")}`);
} catch (error) {
  console.error(error instanceof Error ? error.message.replace(/data:text\/javascript;base64,[A-Za-z0-9+/=]+/g, "<test bundle>") : error);
  process.exitCode = 1;
}
