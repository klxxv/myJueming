// Real Vue settings regression against a deterministic Host bridge. This does not test resource downloads or model execution.
import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');

const screenshots = process.env.SETTINGS_SCREENSHOT_DIR || join(tmpdir(), 'jueming-settings-browser');
await mkdir(screenshots, { recursive: true });
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
page.setDefaultTimeout(12000);
const errors = [];
page.on('pageerror', error => errors.push(error.message));

await page.addInitScript(() => {
  const callbacks = new Map();
  let callbackId = 0;
  const projectId = crypto.randomUUID(), sourceDoc = crypto.randomUUID(), targetDoc = crypto.randomUUID();
  const sourceId = crypto.randomUUID(), targetId = crypto.randomUUID();
  const texts = { [sourceId]: '今天有大雨。', [targetId]: 'Heavy rain was forecast.' };
  const view = {
    contract_version: '1.0',
    project: { project_id: projectId, name: 'Settings browser verification', source_language: 'zh', target_language: 'en', document_ids: [sourceDoc, targetDoc], current_revision_id: '1', format_version: '1.0', created_at: '2026-09-13T00:00:00Z', updated_at: '2026-09-13T00:00:00Z' },
    documents: [{ document_id: sourceDoc, project_id: projectId, title: 'source' }, { document_id: targetDoc, project_id: projectId, title: 'target' }],
    segments: [{ segment_id: sourceId, document_id: sourceDoc, content_hash: sourceId, content_length: texts[sourceId].length }, { segment_id: targetId, document_id: targetDoc, content_hash: targetId, content_length: texts[targetId].length }],
    segment_orders: [{ document_id: sourceDoc, entries: [{ segment_id: sourceId, position_key: '0' }] }, { document_id: targetDoc, entries: [{ segment_id: targetId, position_key: '0' }] }],
    alignments: [{ alignment_id: crypto.randomUUID(), source_segment_ids: [sourceId], target_segment_ids: [targetId], producer: 'manual', cardinality: '1:1' }],
    revisions: [{ revision_id: '1', project_id: projectId, created_at: '2026-09-13T00:00:00Z', change_set: { operation: 'create_project', affected_count: 2 }, summary: 'created', state: 'complete' }],
    bookmarks: [], annotations: [],
    summary: { project_id: projectId, name: 'Settings browser verification', source_label: 'source.txt', target_label: 'target.txt', source_count: 1, target_count: 1, alignment_count: 1, source_unlinked_count: 0, target_unlinked_count: 0, revision_id: '1' },
  };
  const feature = { feature_id: 'translation_research', desired_enabled: false, status: 'disabled', stage: null, generation: '0', activation_id: null, completed_bytes: null, total_bytes: null, reason: null, resources_ready: false, worker_state: 'stopped', default_similarity: 'fuzzy.edit_distance', auto_locate: true };
  window.settingsAudit = {
    calls: [], listeners: {}, sequence: 0, feature,
    settings: { schemaVersion: 2, device: { appearance: { theme: 'light' }, privacy: { localOnlyMode: true } } },
    holdPreferences: false, rejectPreferences: false, pendingPreferences: null,
  };
  const audit = window.settingsAudit;
  const changeFeature = patch => {
    Object.assign(feature, patch, { generation: String(BigInt(feature.generation) + 1n) });
    return structuredClone(feature);
  };
  audit.publishFeature = patch => {
    changeFeature(patch);
    const event = { contract_version: '1.0', sequence: String(++audit.sequence), kind: 'feature_changed', binding_id: null, origin: 'native', payload: { feature_id: feature.feature_id, generation: feature.generation } };
    for (const handler of audit.listeners.agent_subscribe || []) callbacks.get(handler)?.({ event: 'agent_subscribe', id: 1, payload: event });
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
    transformCallback(callback) { callbacks.set(++callbackId, callback); return callbackId; },
    unregisterCallback(id) { callbacks.delete(id); },
    async invoke(cmd, args = {}) {
      audit.calls.push({ cmd, args });
      if (cmd === 'load_app_settings') return structuredClone(audit.settings);
      if (cmd === 'save_app_settings') { audit.settings = JSON.parse(JSON.stringify(args.settings)); return null; }
      if (cmd === 'get_current_project') return structuredClone(view);
      if (cmd === 'get_project_summary') return structuredClone(view.summary);
      if (cmd === 'list_bookmarks') return [];
      if (cmd === 'load_parallel_slice') return { project_id: projectId, revision_id: '1', segments: args.request.segment_ids.map(id => ({ segment_id: id, content: texts[id], content_hash: id })) };
      if (cmd === 'agent_projection') return { request_id: 'projection', sequence: String(audit.sequence), data: { project: { project: view.project }, binding_id: null, context: null, search_spec: { query: '', regex: false, case_sensitive: false, language_id: null }, search_results: null, proposals: [], pending_ui_actions: [] } };
      if (cmd === 'agent_runtime_status') return { configured: false, available: false, active_run_ids: [], recovered_interrupted_runs: 0 };
      if (cmd === 'agent_runtime_history') return { session_id: 'settings-session', messages: [] };
      if (cmd === 'plugin:event|listen') { (audit.listeners[args.event] ||= []).push(args.handler); return 1; }
      if (cmd.startsWith('plugin:event|')) return 1;
      if (cmd !== 'agent_call') return null;
      const { method, params } = args.call;
      let data = {};
      if (method === 'app.bind_session') data = { binding_id: 'settings-binding', project_id: projectId, revision_id: '1' };
      else if (method === 'capabilities.get') data = { contract_version: '2', generation: feature.generation, features: [structuredClone(feature)], slots: [], operators: [] };
      else if (['pipeline.list_proposals', 'pipeline.list', 'pipeline.list_runs'].includes(method)) data = [];
      else if (method === 'features.enable' || method === 'features.retry_prepare') data = changeFeature({ desired_enabled: true, status: 'preparing', stage: 'downloading', activation_id: crypto.randomUUID(), completed_bytes: '1048576', total_bytes: '4194304', reason: null });
      else if (method === 'features.cancel_prepare') data = changeFeature({ desired_enabled: false, status: 'cancelling' });
      else if (method === 'features.disable') data = changeFeature({ desired_enabled: false, status: 'disabled', stage: null, worker_state: 'stopped', reason: null });
      else if (method === 'features.update_preferences') {
        if (audit.holdPreferences) await new Promise(resolve => { audit.pendingPreferences = { params: structuredClone(params), complete: resolve }; });
        if (audit.rejectPreferences) throw { code: 'io_error', message: '设置写入失败，请重试' };
        data = changeFeature(params);
        audit.pendingPreferences = null;
      }
      return { request_id: args.call.request_id, sequence: String(audit.sequence), data };
    },
  };
});

const settings = () => page.locator('.research-settings');
const toggle = () => settings().locator('input[type="checkbox"]').first();
const preferences = () => settings().locator('select');
const openSettings = async () => {
  await page.locator('[data-nav-id="settings"]').click();
  await page.getByRole('navigation', { name: '设置分类' }).getByRole('button', { name: '译法研究', exact: true }).click();
  await settings().waitFor();
};
const publish = patch => page.evaluate(value => window.settingsAudit.publishFeature(value), patch);
const waitStatus = text => settings().locator('#research-feature-status').filter({ hasText: text }).waitFor();
const screenshot = name => page.screenshot({ path: join(screenshots, name), fullPage: true });

try {
  await page.goto(process.env.RESEARCH_BASE_URL || 'http://127.0.0.1:1420');
  await page.getByText('今天有大雨。', { exact: true }).waitFor();
  await openSettings();
  await waitStatus('尚未开启');
  assert.equal(await toggle().isChecked(), false, 'disabled feature still has an actionable management entry');
  assert.equal(await toggle().isEnabled(), true);
  assert.equal(await page.evaluate(() => window.settingsAudit.settings.device.privacy.localOnlyMode), true);

  // Native keyboard activation reaches the Host even with the legacy local data preference.
  await toggle().focus();
  await page.keyboard.press('Space');
  await waitStatus('正在下载所需资源');
  await page.waitForFunction(() => !document.querySelector('.research-settings input[type="checkbox"]').disabled);
  assert.equal(await toggle().isChecked(), true);
  assert.equal(await settings().getByRole('progressbar').getAttribute('value'), '25');
  assert.equal(await settings().getByRole('progressbar').getAttribute('aria-valuetext'), '1 MB / 4 MB');
  await screenshot('01-light-preparing.png');
  await settings().getByRole('button', { name: '取消开启', exact: true }).click();
  await waitStatus('正在取消开启');
  assert.equal(await toggle().isDisabled(), true);
  await publish({ status: 'disabled', desired_enabled: false, stage: null, completed_bytes: null, total_bytes: null });
  await waitStatus('尚未开启');

  await toggle().click();
  await waitStatus('正在下载所需资源');
  await publish({ status: 'failed', reason: { code: 'network_unavailable', message: '网络暂不可用，已下载内容会保留。', retryable: true } });
  await waitStatus('准备失败');
  assert.equal(await toggle().isChecked(), true, 'failed preparation must not fake a ready result or erase the requested state');
  await screenshot('02-failed-retry.png');
  await settings().getByRole('button', { name: '重试准备', exact: true }).click();
  await waitStatus('正在下载所需资源');

  // Finishing after the user leaves settings must not steal focus/navigation.
  await page.locator('[data-nav-id="parallel"]').click();
  await publish({ status: 'ready', desired_enabled: true, resources_ready: true, worker_state: 'ready', stage: null, reason: null });
  await page.waitForFunction(() => document.querySelector('[data-nav-id="research"]'));
  assert.equal(await page.locator('[data-nav-id="parallel"]').evaluate(element => element.classList.contains('nav-item--active')), true);
  await openSettings();
  await waitStatus('已就绪');
  await settings().getByRole('button', { name: '打开译法研究', exact: true }).click();
  await page.locator('.research-workspace:visible').waitFor();
  await openSettings();

  // Default controls retain the acknowledged Host values throughout an outstanding write.
  await settings().locator('summary').focus();
  await page.keyboard.press('Enter');
  await preferences().waitFor();
  assert.equal(await settings().getByText('已加载', { exact: true }).isVisible(), true, 'Host ready worker state is shown as loaded');
  await page.evaluate(() => { window.settingsAudit.holdPreferences = true; });
  await preferences().selectOption('fuzzy.char_ngram');
  await page.waitForFunction(() => window.settingsAudit.pendingPreferences !== null);
  assert.equal(await preferences().inputValue(), 'fuzzy.edit_distance');
  assert.equal(await preferences().isDisabled(), true);
  assert.equal(await page.evaluate(() => window.settingsAudit.feature.default_similarity), 'fuzzy.edit_distance');
  await page.evaluate(() => window.settingsAudit.pendingPreferences.complete());
  await page.waitForFunction(() => document.querySelector('.research-settings select')?.value === 'fuzzy.char_ngram');
  await page.waitForFunction(() => !document.querySelector('.research-settings select')?.disabled);
  await page.evaluate(() => { window.settingsAudit.rejectPreferences = true; });
  await preferences().selectOption('fuzzy.edit_distance');
  await page.waitForFunction(() => window.settingsAudit.pendingPreferences !== null);
  await page.evaluate(() => window.settingsAudit.pendingPreferences.complete());
  await page.waitForFunction(() => !document.querySelector('.research-settings select')?.disabled);
  assert.equal(await preferences().inputValue(), 'fuzzy.char_ngram', 'failed saves retain the last acknowledged value');
  await page.evaluate(() => { window.settingsAudit.pendingPreferences = null; window.settingsAudit.rejectPreferences = false; });
  const autoLocate = settings().locator('input[type="checkbox"]').nth(1);
  await autoLocate.focus();
  await page.keyboard.press('Space');
  await page.waitForFunction(() => window.settingsAudit.pendingPreferences !== null);
  assert.equal(await autoLocate.isChecked(), true);
  await page.evaluate(() => window.settingsAudit.pendingPreferences.complete());
  await page.waitForFunction(() => !document.querySelectorAll('.research-settings input[type="checkbox"]')[1].checked);
  await page.waitForFunction(() => !document.querySelectorAll('.research-settings input[type="checkbox"]')[1].disabled);

  // Reading or typing in settings must not call research confirmation commands.
  await page.getByRole('searchbox', { name: '搜索设置' }).fill('XLM-R');
  assert.equal(await page.getByRole('searchbox', { name: '搜索设置' }).evaluate(element => document.activeElement === element), true);
  await page.keyboard.type(' op');
  assert.equal(await page.evaluate(() => window.settingsAudit.calls.filter(call => call.args?.call?.method === 'research.confirm').length), 0);
  await page.getByRole('searchbox', { name: '搜索设置' }).fill('');

  // The application supports light/eye themes. System dark preference must preserve the chosen eye theme.
  await page.getByRole('navigation', { name: '设置分类' }).getByRole('button', { name: '外观与阅读', exact: true }).click();
  await page.locator('.settings-pane select').first().selectOption('eye');
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'eye');
  await openSettings();
  await waitStatus('已就绪');
  await settings().locator('summary').click();
  await screenshot('03-eye-ready.png');
  assert.notEqual(await settings().locator('.research-card').first().evaluate(element => getComputedStyle(element).backgroundColor), 'rgb(255, 255, 255)');
  await page.emulateMedia({ colorScheme: 'dark', reducedMotion: 'reduce' });
  assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), 'eye');
  await settings().locator('summary').focus();
  assert.equal(await settings().locator('summary').evaluate(element => document.activeElement === element), true);
  await screenshot('04-system-dark-eye-keyboard.png');

  await toggle().click();
  await waitStatus('已关闭 · 资源已准备');
  assert.equal(await toggle().isChecked(), false);
  assert.equal(await settings().getByText('关闭功能会保留已准备的资源和研究记录，不会删除工程或历史。', { exact: true }).isVisible(), true);
  assert.equal(await page.evaluate(() => window.settingsAudit.feature.resources_ready), true);
  assert.equal(await settings().getByText('已停止', { exact: true }).isVisible(), true, 'disabled worker is shown as stopped');
  await screenshot('05-disabled-resources-retained.png');

  await publish({ desired_enabled: true, status: 'blocked', resources_ready: false, reason: { code: 'network_disabled', message: '当前策略禁止联网下载，本地资源不足。', retryable: false } });
  await waitStatus('暂时无法启用');
  assert.equal(await settings().getByRole('button', { name: '重试准备', exact: true }).count(), 0);
  await toggle().click();
  await waitStatus('尚未开启');

  assert.deepEqual(errors, [], 'no browser runtime errors');
  console.log(JSON.stringify({ passed: ['disabled-entry', 'keyboard-enable', 'legacy-local-mode-not-a-download-gate', 'preparation-progress', 'cancel', 'failure-retry', 'no-navigation-steal', 'ready-open', 'preference-host-acknowledgement', 'preference-write-failure', 'input-focus', 'eye-theme', 'system-dark-preference', 'disable-retains-resources', 'blocked-policy'], screenshots }, null, 2));
} catch (error) {
  await screenshot('failure.png');
  console.error(await page.locator('body').innerText());
  throw error;
} finally {
  await browser.close();
}
