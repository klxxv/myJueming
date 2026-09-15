import { build } from 'esbuild';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('../../', import.meta.url));
const result = await build({ entryPoints: [`${root}tests/alignment-layout/batching-contracts.ts`], bundle: true, format: 'esm', platform: 'node', target: 'node24', write: false, nodePaths: [`${root}apps/desktop/node_modules`], define: { 'import.meta.env.DEV': 'false' } });
await import(`data:text/javascript;base64,${Buffer.from(result.outputFiles[0].text).toString('base64')}`);
