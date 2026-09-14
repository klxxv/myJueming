import { build } from 'esbuild';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('../../', import.meta.url));
const result = await build({ entryPoints: [path.join(root, 'tests/comparison/projection-contracts.ts')], bundle: true, format: 'esm', platform: 'node', target: 'node24', write: false, nodePaths: [path.join(root, 'apps/desktop/node_modules')] });
await import(`data:text/javascript;base64,${Buffer.from(result.outputFiles[0].text).toString('base64')}`);
