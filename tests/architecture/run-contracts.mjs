import { build } from "esbuild";
import { fileURLToPath } from "node:url";
import path from "node:path";
const root = fileURLToPath(new URL("../../", import.meta.url));
const bundled = await build({
  entryPoints: [path.join(root, "tests/architecture/workspace-contracts.ts")],
  bundle: true, format: "esm", platform: "node", target: "node24", write: false,
  nodePaths: [path.join(root, "apps/desktop/node_modules")],
});
await import(`data:text/javascript;base64,${Buffer.from(bundled.outputFiles[0].text).toString("base64")}`);
