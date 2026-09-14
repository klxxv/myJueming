// Detect untranslated Han text in every production SFC. Comments, styles and
// regular expressions are not UI copy. CSS generated content is checked too.
// Exceptions are exact and documented.
import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
const root = fileURLToPath(new URL('../../', import.meta.url));
const require = createRequire(path.join(root, 'apps/desktop/package.json'));
const { parse } = require('vue/compiler-sfc');
const ts = require('typescript');
const allowed = new Map([
  ['App.vue|2024政府工作报告_中英对齐', 'Bilingual demo project name, not UI copy'],
  ['App.vue|政府工作报告 · 平行视图引导', 'Tutorial project data; presentation label is translated separately'],
  ['App.vue|政府工作报告_中文节选.txt', 'Example source filename'],
  ['components/SettingsWorkspace.vue|中文（简体）', 'Native language name in language selector'],
  ['components/SettingsWorkspace.vue|日本語', 'Native language name in language selector'],
  ['components/ImportSegmentationPreview.vue|文本未产生可用 Segment', 'Exact legacy Kernel warning matched to a translated message'],
  ['components/ResearchWorkspace.vue|省译', 'Canonical omission group/strategy default sent to the research API; translating it would change persisted grouping identity'],
  ['components/ResearchWorkspace.vue|意译', 'Canonical paraphrase strategy default; displayed action labels are localized separately'],
]);
const seen = new Set();
const failures = [];
const han = /\p{Script=Han}/u;
function check(file, text, line) {
  if (!han.test(text)) return;
  const key = `${file}|${text.trim()}`;
  if (allowed.has(key)) seen.add(key);
  else failures.push(`${file}:${line}: ${text.trim()}`);
}
function script(file, source, offset = 0) {
  const ast = ts.createSourceFile(file + '.ts', source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  function visit(node) {
    if (ts.isStringLiteralLike(node) || ts.isTemplateHead(node) || ts.isTemplateMiddle(node) || ts.isTemplateTail(node)) {
      check(file, node.text, offset + ast.getLineAndCharacterOfPosition(node.getStart(ast)).line + 1);
    }
    ts.forEachChild(node, visit);
  }
  visit(ast);
}
async function walk(directory) {
  const result = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const file = path.join(directory, entry.name);
    if (entry.isDirectory()) result.push(...await walk(file));
    else if (entry.name.endsWith('.vue')) result.push(file);
  }
  return result;
}
const sourceRoot = path.join(root, 'apps/desktop/src');
const files = await walk(sourceRoot);
for (const filename of files) {
  const file = path.relative(sourceRoot, filename);
  const { descriptor } = parse(await readFile(filename, 'utf8'), { filename });
  for (const block of [descriptor.script, descriptor.scriptSetup]) if (block) script(file, block.content, block.loc.start.line - 1);
  for (const style of descriptor.styles) {
    const css = style.content.replace(/\/\*[\s\S]*?\*\//g, '');
    for (const match of css.matchAll(/\bcontent\s*:\s*(["'])(.*?)\1/g)) check(file, match[2], style.loc.start.line);
  }
  function visit(node) {
    const line = node.loc?.start.line ?? 1;
    if (node.type === 2) check(file, node.content, line);
    if (node.type === 5) script(file, node.content.content, line - 1);
    for (const prop of node.props ?? []) {
      if (prop.type === 6 && prop.value) check(file, prop.value.content, prop.loc.start.line);
      if (prop.type === 7 && prop.exp) script(file, prop.exp.content, prop.loc.start.line - 1);
    }
    for (const child of node.children ?? []) visit(child);
  }
  if (descriptor.template?.ast) visit(descriptor.template.ast);
}
if (failures.length) {
  console.error(`Untranslated Vue UI strings (${failures.length}):\n${failures.join('\n')}`);
  process.exit(1);
}
assert.deepEqual([...allowed.keys()].filter(key => !seen.has(key)), [], 'Remove stale source-audit exceptions');
console.log(`Vue source audit passed (${files.length} components; ${seen.size} documented non-UI exceptions)`);
