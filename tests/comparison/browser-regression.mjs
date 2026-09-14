// Real App + deterministic native Host bridge. Kernel persistence is covered by comparison.rs.
import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
const engines = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const engine = process.env.COMPARISON_BROWSER || 'chromium';
const browser = await engines[engine].launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1560, height: 1000 } });
page.setDefaultTimeout(12000);
const errors = []; page.on('pageerror', error => errors.push(error.message));
const directory = process.env.COMPARISON_SCREENSHOT_DIR || join(tmpdir(), 'jueming-comparison'); await mkdir(directory, { recursive: true });
await page.addInitScript(() => {
  const calls = [], callbacks = new Map(), texts = {}; let callbackId = 0;
  const projectId = crypto.randomUUID();
  const documents = ['原文', '译本一', '译本二', '译本三'].map((title, index) => ({ document_id: crypto.randomUUID(), project_id: projectId, language_id: index ? 'en' : 'zh', title, source_asset_id: crypto.randomUUID(), segment_order_id: crypto.randomUUID(), created_revision_id: '1' }));
  const project = { project_id: projectId, name: '三份译本对照验证', source_language: 'zh', target_language: 'en', document_ids: documents.map(doc => doc.document_id), current_revision_id: '1', format_version: '2.0', created_at: '2026-09-13T00:00:00Z', updated_at: '2026-09-13T00:00:00Z', comparison: { source_document_id: documents[0].document_id, target_document_ids: documents.slice(1).map(doc => doc.document_id) } };
  const view = { contract_version: '1.0', project, documents, segments: [], segment_orders: documents.map(doc => ({ document_id: doc.document_id, segment_order_id: doc.segment_order_id, entries: [], updated_revision_id: '1' })), alignments: [], revisions: [{ revision_id: '1', created_at: project.created_at, change_set: { operation: 'create_project', affected_count: 2400 }, summary: 'created', state: 'complete' }], bookmarks: [], annotations: [], summary: { project_id: projectId, name: project.name, source_label: '原文', target_label: '译本一', source_count: 600, target_count: 600, alignment_count: 1800, source_unlinked_count: 0, target_unlinked_count: 0, revision_id: '1' } };
  for (let row = 0; row < 600; row++) for (let col = 0; col < 4; col++) {
    const id = crypto.randomUUID(); const content = col ? `${documents[col].title} sentence ${row}. ` + (col === 2 && row === 2 ? 'A long translation wraps across several lines. '.repeat(12) : '') : `原文第 ${row} 段。`;
    texts[id] = content; view.segments.push({ segment_id: id, document_id: documents[col].document_id, content_hash: id, content_length: content.length }); view.segment_orders[col].entries.push({ segment_id: id, position_key: String(row) });
  }
  for (let col = 1; col < 4; col++) for (let row = 0; row < 600; row++) view.alignments.push({ alignment_id: crypto.randomUUID(), project_id: projectId, source_segment_ids: [view.segment_orders[0].entries[row].segment_id], target_segment_ids: [view.segment_orders[col].entries[row].segment_id], producer: 'manual', cardinality: '1:1', created_revision_id: '1', updated_revision_id: '1' });
  // Overlapping n:m groups across translations must share a band without conflating relations.
  for (const [col, start, count] of [[1, 4, 2], [2, 5, 3]]) {
    const originals = view.alignments.filter(relation => view.segment_orders[col].entries.slice(start, start + count).some(entry => relation.target_segment_ids.includes(entry.segment_id)));
    view.alignments = view.alignments.filter(relation => !originals.includes(relation));
    view.alignments.push({ ...originals[0], source_segment_ids: originals.flatMap(relation => relation.source_segment_ids), target_segment_ids: originals.flatMap(relation => relation.target_segment_ids), cardinality: 'n:m' });
  }
  if (new URL(location.href).searchParams.has('large-band')) {
    const targetIds = new Set(view.segment_orders[1].entries.map(entry => entry.segment_id));
    view.alignments = view.alignments.filter(relation => !targetIds.has(relation.target_segment_ids[0]));
    view.alignments.push({ alignment_id: crypto.randomUUID(), project_id: projectId, source_segment_ids: view.segment_orders[0].entries.map(entry => entry.segment_id), target_segment_ids: [...targetIds], producer: 'manual', cardinality: 'n:m', created_revision_id: '1', updated_revision_id: '1' });
  }
  window.audit = { calls, texts, view };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
  window.__TAURI_INTERNALS__ = { metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } }, transformCallback(fn) { callbacks.set(++callbackId, fn); return callbackId; }, unregisterCallback(id) { callbacks.delete(id); }, async invoke(cmd, args = {}) {
    calls.push({ cmd, args });
    if (cmd === 'get_current_project' || cmd === 'open_project') return structuredClone(view);
    if (cmd === 'get_project_summary') return structuredClone(view.summary);
    if (cmd === 'list_bookmarks') return [];
    if (cmd === 'agent_projection') return { request_id: 'projection', sequence: '0', data: { project: { project }, binding_id: null, context: null, search_spec: { query: '', regex: false, case_sensitive: false, language_id: null }, search_results: null, proposals: [], pending_ui_actions: [] } };
    if (cmd === 'agent_call') {
      const method = args.call.method; let data = {};
      if (method === 'app.bind_session') data = { binding_id: 'binding', project_id: projectId, revision_id: project.current_revision_id };
      if (method === 'pipeline.list_proposals' || method === 'pipeline.list' || method === 'pipeline.list_runs') data = [];
      if (method === 'capabilities.get') data = { contract_version: '2', generation: '1', features: [], slots: [], operators: [] };
      return { request_id: args.call.request_id, sequence: '0', data };
    }
    if (cmd === 'agent_runtime_status') return { configured: false, available: false, active_run_ids: [], recovered_interrupted_runs: 0 };
    if (cmd === 'agent_runtime_history') return { session_id: 'session', messages: [] };
    if (cmd.startsWith('plugin:event|')) return 1;
    if (cmd === 'load_parallel_slice') return { project_id: projectId, revision_id: project.current_revision_id, segments: args.request.segment_ids.map(id => ({ segment_id: id, content: texts[id], content_hash: view.segments.find(segment => segment.segment_id === id).content_hash })) };
    if (cmd === 'execute_command') {
      const command = args.command; if (command.kind === 'unlink_alignment') view.alignments = view.alignments.filter(relation => relation.alignment_id !== command.payload.alignment_id);
      if (command.kind === 'update_segment') { texts[command.payload.segment_id] = command.payload.content; const segment = view.segments.find(segment => segment.segment_id === command.payload.segment_id); segment.content_hash = crypto.randomUUID(); segment.content_length = command.payload.content.length; }
      const revision = String(BigInt(project.current_revision_id) + 1n); project.current_revision_id = revision; view.summary.revision_id = revision; view.revisions.push({ revision_id: revision, created_at: project.created_at, change_set: { operation: command.kind, affected_count: 1 }, summary: 'changed', state: 'complete' });
      return { project_id: projectId, command_id: command.command_id, committed_revision_id: revision, status: 'committed' };
    }
    if (cmd === 'plugin:dialog|message') return 'Ok';
    return null;
  } };
});
const canonical = () => page.evaluate(() => JSON.stringify({ documents: audit.view.documents, orders: audit.view.segment_orders, alignments: audit.view.alignments, revision: audit.view.project.current_revision_id }));
const assertAligned = async (minimumBands = 3) => {
  await page.waitForFunction(minimumBands => {
    const bands = new Map();
    for (const element of document.querySelectorAll('.aligned-document-list__band')) {
      const height = element.getBoundingClientRect().height;
      const previous = bands.get(element.dataset.bandId);
      if (previous !== undefined && Math.abs(previous - height) >= 1) return false;
      bands.set(element.dataset.bandId, height);
    }
    return bands.size > minimumBands;
  }, minimumBands);
  const bands = await page.locator('.aligned-document-list__band').evaluateAll(elements => {
    const groups = {};
    for (const element of elements) { const rect = element.getBoundingClientRect(); (groups[element.dataset.bandId] ??= []).push({ top: rect.top, height: rect.height }); }
    return Object.values(groups).filter(group => group.length > 1);
  });
  assert.ok(bands.length > minimumBands);
  const anchors = await page.evaluate(() => {
    let count = 0;
    for (const relation of audit.view.alignments) {
      const source = document.querySelector(`[data-comparison-segment-id="${relation.source_segment_ids[0]}"]`);
      const target = document.querySelector(`[data-comparison-segment-id="${relation.target_segment_ids[0]}"]`);
      if (source && target) { if (Math.abs(source.getBoundingClientRect().top - target.getBoundingClientRect().top) >= 1) return -1; count++; }
    }
    return count;
  });
  assert.ok(anchors > 0, 'every rendered relation starts at the same height on its source and target');
  for (const group of bands) { assert.ok(Math.max(...group.map(band => band.top)) - Math.min(...group.map(band => band.top)) < 1); assert.ok(Math.max(...group.map(band => band.height)) - Math.min(...group.map(band => band.height)) < 1); }
};
try {
  await page.goto(process.env.COMPARISON_BASE_URL || 'http://127.0.0.1:1420');
  await page.getByText('原文第 0 段。', { exact: true }).waitFor();
  assert.equal(await page.locator('.workspace-panel').count(), 4);
  await page.waitForFunction(() => document.querySelectorAll('.comparison-cell .segment-card').length > 8);
  await assertAligned();
  const original = await canonical();
  const cards = await page.locator('.comparison-cell .segment-card').count(); assert.ok(cards < 200, `virtualized ${cards}/2400 cards`);
  const relation = await page.locator('.comparison-binding').first().getAttribute('data-relation-id');
  await page.locator(`.comparison-binding[data-relation-id="${relation}"] button`).first().click();
  assert.equal(await page.locator(`.comparison-binding--active[data-relation-id="${relation}"]`).count(), 2);
  assert.equal(await page.locator('.comparison-cell .segment-card--selected').count(), 2);
  await page.getByRole('button', { name: '译本三 左移', exact: true }).click();
  assert.deepEqual(await page.locator('.workspace-panel__title strong').allTextContents(), ['原文', '译本一', '译本三', '译本二']);
  const handle = page.locator('.workspace-panel').nth(2).locator('[data-panel-handle]');
  const dragSource = await handle.boundingBox(); const dragTarget = await page.locator('.workspace-panel__header').first().boundingBox();
  await page.mouse.move(dragSource.x + dragSource.width / 2, dragSource.y + dragSource.height / 2);
  await page.mouse.down();
  await page.mouse.move(dragSource.x + 12, dragSource.y + 18, { steps: 4 });
  await page.mouse.move(dragTarget.x + 90, dragTarget.y + 25, { steps: 12 });
  await page.mouse.move(dragTarget.x + 95, dragTarget.y + 25);
  await page.mouse.up();
  assert.deepEqual(await page.locator('.workspace-panel__title strong').allTextContents(), ['译本三', '原文', '译本一', '译本二']);
  const separator = page.getByRole('separator', { name: '调整 原文 列宽', exact: true });
  await separator.focus(); await separator.press('ArrowRight'); await separator.press('ArrowRight');
  assert.equal(await separator.getAttribute('aria-valuenow'), '400'); await assertAligned();
  assert.equal(await canonical(), original, 'panel moves/resizes never write canonical state');
  assert.equal(await page.evaluate(() => audit.calls.filter(call => call.cmd === 'execute_command').length), 0);
  await page.screenshot({ path: join(directory, `${engine}-multi-columns.png`) });
  await page.evaluate(() => { document.documentElement.dataset.theme = 'eye'; });
  await page.screenshot({ path: join(directory, `${engine}-eye.png`) });
  assert.notEqual(await page.locator('.workspace-panel').first().evaluate(element => getComputedStyle(element).backgroundColor), 'rgb(255, 255, 255)');
  await page.evaluate(() => { delete document.documentElement.dataset.theme; });
  await page.locator('.comparison-viewport').evaluate(element => { element.scrollTop = 42000; });
  await page.waitForFunction(() => [...document.querySelectorAll('.comparison-cell .segment-card')].some(card => /sentence (2|3|4)\d\d/.test(card.textContent)));
  assert.ok(await page.locator('.comparison-cell .segment-card').count() < 240);
  await assertAligned();
  await page.locator('.comparison-viewport').evaluate(element => { element.scrollTop = 0; });
  await page.getByRole('button', { name: '查找多译本视图', exact: true }).click();
  await page.getByRole('textbox', { name: '查找所有可见译本', exact: true }).fill('译本三 sentence 450.');
  await page.getByText('译本三 sentence 450.', { exact: true }).waitFor();
  assert.equal(await page.locator('.comparison-cell .segment-card--jump-highlighted').count(), 1);
  await page.getByRole('button', { name: '关闭多译本查找', exact: true }).click();
  await page.getByRole('button', { name: '译本二', exact: true }).click();
  await page.locator('.column-headings').getByText('译本二', { exact: false }).waitFor();
  await page.getByText('译本二 sentence 0.', { exact: true }).waitFor();
  assert.equal(await page.getByText('译本一 sentence 0.', { exact: true }).count(), 0);
  await page.getByRole('button', { name: '返回多译本对比', exact: true }).click();
  await page.locator('.workspace-panel').first().waitFor();
  assert.deepEqual(await page.locator('.workspace-panel__title strong').allTextContents(), ['译本三', '原文', '译本一', '译本二'], 'panel order survives leaving and returning');
  assert.equal(await page.getByRole('separator', { name: '调整 原文 列宽', exact: true }).getAttribute('aria-valuenow'), '400');
  const selectedId = await page.locator('.comparison-binding').first().getAttribute('data-relation-id');
  await page.locator(`.comparison-binding[data-relation-id="${selectedId}"] button`).nth(1).click();
  await page.waitForFunction(id => !audit.view.alignments.some(relation => relation.alignment_id === id), selectedId);
  await page.waitForFunction(id => !document.querySelector(`.comparison-binding[data-relation-id="${id}"]`), selectedId);
  await page.locator('[data-nav-id="settings"]').click();
  await page.locator('[data-nav-id="parallel"]').click(); await page.locator('.workspace-panel').first().waitFor();
  await assertAligned();
  await page.goto((process.env.COMPARISON_BASE_URL || 'http://127.0.0.1:1420') + '/?large-band');
  await page.getByText('原文第 0 段。', { exact: true }).waitFor();
  await assertAligned(0);
  assert.ok(await page.locator('.comparison-cell .segment-card').count() < 200, 'a single 600:600 group is also virtualized internally');
  await page.locator('.comparison-viewport').evaluate(element => { element.scrollTop = 42000; });
  await page.waitForFunction(() => [...document.querySelectorAll('.comparison-cell .segment-card')].some(card => /sentence (2|3|4)\d\d/.test(card.textContent)));
  await assertAligned(0);
  assert.ok(await page.locator('.comparison-cell .segment-card').count() < 240);
  assert.deepEqual(errors, []);
  console.log(`${engine}: multi-column alignment, binding icons, virtualization, column DnD/resize, pair switching, unlink and mode navigation PASS`);
} catch (error) { await page.screenshot({ path: join(directory, `${engine}-failure.png`) }); console.error(errors); throw error; } finally { await browser.close(); }
