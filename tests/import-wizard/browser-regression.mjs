// Run against pnpm dev:web. Real Vue wizard + deterministic Kernel/file-dialog bridge;
// this verifies UI request integrity and lifecycle, not Rust file decoding.
import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
const engines = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const browserName = process.env.IMPORT_WIZARD_BROWSER || 'chromium';
const baseUrl = process.env.IMPORT_WIZARD_BASE_URL || 'http://127.0.0.1:1420';
const screenshotDirectory = process.env.IMPORT_WIZARD_SCREENSHOT_DIR || join(tmpdir(), 'jueming-import-wizard');
await mkdir(screenshotDirectory, { recursive: true });
const componentSource = await (await fetch(`${baseUrl}/src/components/NewProjectDialog.vue`)).text();
const vueUrl = componentSource.match(/from "([^"]*\/vue\.js[^\"]*)"/)?.[1];
const i18nUrl = componentSource.match(/from "(\/src\/i18n\/index\.ts[^\"]*)"/)?.[1];
assert.ok(vueUrl, 'running Vite serves the actual production wizard');
assert.ok(i18nUrl, 'running Vite serves the wizard with the production i18n singleton');
const browser = await engines[browserName].launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 1000 } });
page.setDefaultTimeout(10000);
const errors = [];
page.on('pageerror', error => errors.push(error.message));
await page.addInitScript(() => {
  const audit = window.importWizardAudit = {
    fileChoices: [], previews: [], creates: [], created: [], statuses: [],
    pendingPreviews: [], holdPreview: false, failPreview: false, emptyPreview: false,
    holdCreate: false, pendingCreate: null, beforeCreate: true, busy: false,
  };
  const response = request => {
    const legacy = request.profile.segmentation_mode === 'legacy_tagged_line';
    const sentence = request.profile.segmentation_mode === 'sentence_rules';
    const count = audit.emptyPreview ? 0 : legacy ? 2 : 45;
    const segments = Array.from({ length: count }, (_, ordinal) => ({
      ordinal,
      original_text: legacy ? `<seg>我们/r 学习/v 第${ordinal + 1}段/m 。/w</seg>` : `原始内容 ${ordinal + 1}。`,
      content: legacy ? `我们学习第${ordinal + 1}段。` : `原始内容 ${ordinal + 1}。`,
      boundary: sentence ? 'sentence_punctuation' : 'non_empty_line',
      boundary_marker: sentence ? '。' : null,
      cleanups: legacy ? ['seg_wrappers', 'pos_suffixes_and_whitespace'] : [],
    }));
    const detected = request.input.path.endsWith('source.txt') ? 'gb18030'
      : request.input.path.endsWith('translation-1.txt') ? 'utf-16le'
      : request.input.path.endsWith('translation-2.txt') ? 'windows-1252'
      : request.input.path.endsWith('translation-3.txt') ? 'utf8-bom' : 'utf8';
    const encoding = request.auto_detect_encoding ? detected : request.profile.encoding;
    const method = !request.auto_detect_encoding ? 'manual'
      : ['utf-16le', 'utf8-bom'].includes(encoding) ? 'bom' : encoding === 'utf8' ? 'utf8' : 'statistical';
    return { label: request.input.path.split(/[\\/]/).pop(), profile: { ...request.profile, encoding }, had_bom: method === 'bom', encoding_detection: method,
      sha256: `${request.input.path}|${encoding}|${request.profile.segmentation_mode}`,
      byte_length: 800, preview: { segments, cleaned_text: segments.map(segment => segment.content).join('\n'), warnings: [] } };
  };
  audit.previewImport = async request => {
    const record = { request: structuredClone(request), response: response(request) };
    audit.previews.push(record);
    if (audit.failPreview) { audit.failPreview = false; throw { message: '所选编码无法解码文件' }; }
    if (audit.holdPreview) {
      audit.holdPreview = false;
      return new Promise((resolve, reject) => audit.pendingPreviews.push({ resolve: () => resolve(record.response), reject }));
    }
    return record.response;
  };
  audit.createProject = async request => {
    audit.creates.push(structuredClone(request));
    if (audit.holdCreate) await new Promise((resolve, reject) => { audit.pendingCreate = { resolve, reject }; });
    return { project: { project_id: 'mock-created-project' } };
  };
  window.__TAURI_INTERNALS__ = {
    async invoke(command) {
      if (command === 'plugin:dialog|open') return audit.fileChoices.shift() ?? null;
      throw new Error(`Unexpected native command ${command}`);
    },
  };
});
await page.route('**/__import-wizard-regression', route => route.fulfill({
  contentType: 'text/html',
  body: `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"></head><body><div id="app"></div><script type="module">
import { createApp, h, ref } from ${JSON.stringify(vueUrl)};
import NewProjectDialog from '/src/components/NewProjectDialog.vue';
import { fallbackLanguages } from '/src/domain/languages.ts';
import { i18n, initializeLocale, setUiLocale } from ${JSON.stringify(i18nUrl)};
import '/src/styles.css';
const audit = window.importWizardAudit;
window.setWizardLocale = setUiLocale;
const app = createApp({ setup() {
  const dialog = ref(null), busy = ref(false);
  const kernelClient = { listSupportedLanguages: async () => fallbackLanguages, previewImport: audit.previewImport, createProject: audit.createProject };
  return () => h('div', {}, [
    h('button', { id: 'open-wizard', onClick: () => dialog.value.open() }, '新建工程测试入口'),
    h(NewProjectDialog, { ref: dialog, busy: busy.value, 'onUpdate:busy': value => { busy.value = value; audit.busy = value; },
      kernelClient, beforeCreate: async () => audit.beforeCreate,
      onCreated: (snapshot, path) => audit.created.push({ snapshot, path }), onStatus: message => audit.statuses.push(message) }),
  ]);
}});
app.use(i18n);
setUiLocale('zh');
initializeLocale();
app.mount('#app');
</script></body></html>`,
}));
const dialog = () => page.getByRole('dialog', { name: '逐份导入，一起对照' });
const next = () => dialog().getByRole('button', { name: '下一步', exact: true });
const open = async () => { const trigger = page.getByRole('button', { name: '新建工程测试入口' }); await trigger.focus(); await trigger.press('Enter'); await dialog().waitFor(); };
const choose = async (buttonName, path) => {
  await page.evaluate(path => window.importWizardAudit.fileChoices.push(path), path);
  await dialog().getByRole('button', { name: buttonName, exact: true }).click();
};
const navigation = () => dialog().getByRole('navigation', { name: '新建工程步骤' });
const screenshot = name => page.screenshot({ path: join(screenshotDirectory, `${browserName}-${name}.png`), fullPage: true });

try {
  await page.goto(`${baseUrl}/__import-wizard-regression`);

  // Keep one real wizard instance open while switching through every bundled UI
  // locale. This catches static setup-time labels as well as untranslated template
  // content, and verifies that a language change does not reset the draft.
  await open();
  await dialog().getByLabel('项目名称').fill('Locale retention');
  const localeExpectations = {
    zh: { lang: 'zh-CN', title: '逐份导入，一起对照', setup: '先设置工程信息', projectName: '项目名称', browse: '浏览…', add: '增加译本', cancel: '取消', next: '下一步' },
    en: { lang: 'en-US', title: 'Import each text, then compare them together', setup: 'Set up the project', projectName: 'Project name', browse: 'Browse…', add: 'Add translation', cancel: 'Cancel', next: 'Next' },
    ja: { lang: 'ja-JP', title: 'テキストを順に読み込み、一緒に対照', setup: 'プロジェクト情報を設定', projectName: 'プロジェクト名', browse: '参照…', add: '訳文を追加', cancel: 'キャンセル', next: '次へ' },
    fr: { lang: 'fr-FR', title: 'Importez chaque texte, puis comparez-les ensemble', setup: 'Configurer le projet', projectName: 'Nom du projet', browse: 'Parcourir…', add: 'Ajouter une traduction', cancel: 'Annuler', next: 'Suivant' },
    de: { lang: 'de-DE', title: 'Texte einzeln importieren und gemeinsam vergleichen', setup: 'Projekt einrichten', projectName: 'Projektname', browse: 'Durchsuchen…', add: 'Übersetzung hinzufügen', cancel: 'Abbrechen', next: 'Weiter' },
  };
  for (const [locale, expected] of Object.entries(localeExpectations)) {
    await page.evaluate(locale => window.setWizardLocale(locale), locale);
    const localizedDialog = page.getByRole('dialog', { name: expected.title });
    await localizedDialog.waitFor();
    await localizedDialog.getByRole('heading', { name: expected.setup, exact: true }).waitFor();
    assert.equal(await localizedDialog.getByLabel(expected.projectName, { exact: true }).inputValue(), 'Locale retention', `${locale} switch preserves the project draft`);
    assert.equal(await localizedDialog.getByRole('button', { name: expected.browse, exact: true }).count(), 1);
    assert.equal(await localizedDialog.getByRole('button', { name: expected.add, exact: true }).count(), 1);
    assert.equal(await localizedDialog.getByRole('button', { name: expected.cancel, exact: true }).count(), 1);
    assert.equal(await localizedDialog.getByRole('button', { name: expected.next, exact: true }).count(), 1);
    assert.equal(await page.locator('html').getAttribute('lang'), expected.lang, `${locale} updates the document language`);
    if (locale === 'en' || locale === 'fr' || locale === 'de') {
      const uiCopy = await localizedDialog.evaluate(element => {
        const clone = element.cloneNode(true);
        clone.querySelectorAll('.step-text small, .document-summary h4, input, select, option').forEach(node => node.remove());
        return clone.innerText;
      });
      assert.doesNotMatch(uiCopy, /[\u3400-\u9fff]/u, `${locale} wizard setup contains no Chinese UI copy outside draft and language values`);
    }
  }
  await page.evaluate(() => window.setWizardLocale('zh'));
  await dialog().getByRole('button', { name: '取消', exact: true }).click();

  // Original Chinese lifecycle and request-integrity suite.
  await open();
  assert.equal(await next().isDisabled(), true);
  assert.equal(await navigation().locator('li').count(), 4);
  assert.equal(await navigation().getByRole('button', { name: /^删除译本/ }).count(), 0, 'one required translation is retained');
  assert.equal(await dialog().getByLabel('项目名称').evaluate(element => element === document.activeElement), true);
  await dialog().getByLabel('项目名称').fill('三个译本对照');
  await choose('浏览…', '/Users/test/Projects');
  await navigation().getByRole('button', { name: '增加译本', exact: true }).click();
  await navigation().getByRole('button', { name: '增加译本', exact: true }).click();
  assert.equal(await navigation().locator('li').count(), 6, 'each translation creates exactly one additional step');
  await navigation().getByRole('button', { name: '增加译本', exact: true }).click();
  assert.equal(await navigation().locator('.step-link').filter({ hasText: '译本 3' }).isDisabled(), true, 'unvisited steps can still need prerequisites');
  await navigation().getByRole('button', { name: '删除译本 3', exact: true }).click();
  assert.equal(await navigation().locator('li').count(), 6, 'an accidentally added translation can be removed without visiting it');
  assert.equal(await navigation().locator('.step-link').filter({ hasText: '译本 3' }).locator('small').innerText(), '译本 3', 'untouched default names follow the remaining step numbers');
  assert.equal(await dialog().getByRole('heading', { name: '先设置工程信息' }).count(), 1);
  await next().focus();
  await page.keyboard.press('Tab');
  assert.equal(await dialog().getByRole('button', { name: '关闭导入向导' }).evaluate(element => element === document.activeElement), true, 'Tab wraps inside the modal');
  await page.keyboard.press('Shift+Tab');
  assert.equal(await next().evaluate(element => element === document.activeElement), true, 'Shift+Tab wraps inside the modal');
  await screenshot('01-project');
  await next().click();
  await dialog().getByRole('heading', { name: '导入原文', exact: true }).waitFor();
  assert.equal(await next().isDisabled(), true);
  assert.equal(await dialog().getByLabel('编码', { exact: true }).inputValue(), 'auto');
  await choose('选择文件…', '/fixtures/source.txt');
  await dialog().getByText('共 45 段', { exact: true }).waitFor();
  await dialog().getByLabel('解码结果').getByText('识别编码：GB18030', { exact: true }).waitFor();
  assert.ok((await dialog().innerText()).includes('统计推测'));
  assert.equal(await page.evaluate(() => window.importWizardAudit.previews.at(-1).request.auto_detect_encoding), true);
  await screenshot('00-auto-decoding');
  // A pending automatic result cannot overwrite a subsequent manual choice.
  await page.evaluate(() => { window.importWizardAudit.holdPreview = true; });
  await dialog().locator('.import-settings').getByRole('button', { name: '重新预览' }).click();
  await page.waitForFunction(() => window.importWizardAudit.pendingPreviews.length === 1);
  await dialog().getByLabel('编码', { exact: true }).selectOption('utf8');
  await dialog().getByLabel('解码结果').getByText('使用编码：UTF-8', { exact: true }).waitFor();
  await page.evaluate(() => window.importWizardAudit.pendingPreviews.shift().resolve());
  assert.equal(await dialog().getByLabel('编码', { exact: true }).inputValue(), 'utf8');
  assert.equal(await dialog().getByLabel('解码结果').innerText(), '使用编码：UTF-8\n手动选择');
  assert.equal(await page.evaluate(() => window.importWizardAudit.previews.at(-1).request.auto_detect_encoding), false);
  await dialog().getByLabel('编码', { exact: true }).selectOption('auto');
  await dialog().getByLabel('解码结果').getByText('识别编码：GB18030', { exact: true }).waitFor();
  assert.equal(await dialog().locator('.preview-row').count(), 20);
  await dialog().locator('.preview-rows').evaluate(element => { element.scrollTop = element.scrollHeight; });
  await dialog().getByRole('button', { name: '下一组预览' }).click();
  await dialog().getByText('第 21 段', { exact: true }).waitFor();
  assert.equal(await dialog().locator('.preview-rows').evaluate(element => element.scrollTop), 0, 'new preview pages start at their first segment');
  assert.equal(await dialog().locator('.preview-row').count(), 20);
  await dialog().getByRole('button', { name: '下一组预览' }).click();
  await dialog().getByText('第 45 段', { exact: true }).waitFor();
  assert.equal(await dialog().locator('.preview-row').count(), 5, 'large imports never mount all preview segments');

  // A slow old rule result must not overwrite the newer rule or its SHA.
  await page.evaluate(() => { window.importWizardAudit.holdPreview = true; });
  await dialog().getByLabel('分段方式', { exact: true }).selectOption('sentence_rules');
  await page.waitForFunction(() => window.importWizardAudit.pendingPreviews.length === 1);
  assert.equal(await next().isDisabled(), true);
  await dialog().getByLabel('分段方式', { exact: true }).selectOption('legacy_tagged_line');
  await dialog().getByText('共 2 段', { exact: true }).waitFor();
  await page.evaluate(() => window.importWizardAudit.pendingPreviews.shift().resolve());
  assert.equal(await dialog().getByText('共 45 段', { exact: true }).count(), 0);
  assert.equal(await dialog().getByText('移除 <seg> / </seg> 标签', { exact: true }).count(), 2);
  assert.equal(await dialog().getByText('我们学习第1段。', { exact: true }).count(), 1);
  assert.ok((await dialog().innerText()).includes('标签本身不作为分段边界'));
  assert.equal(await dialog().getByRole('option', { name: /SISU/ }).count(), 0);
  await navigation().getByRole('button', { name: '增加译本', exact: true }).click();
  await navigation().getByRole('button', { name: '删除译本 4', exact: true }).click();
  assert.equal(await dialog().getByRole('heading', { name: '导入原文', exact: true }).count(), 1, 'removing another step keeps the current document and preview');
  assert.equal(await dialog().getByText('共 2 段', { exact: true }).count(), 1);
  await dialog().getByLabel('原文名称', { exact: true }).fill('中文原文');
  await page.evaluate(() => { window.importWizardAudit.failPreview = true; });
  await dialog().getByLabel('编码', { exact: true }).selectOption('gb18030');
  await dialog().getByRole('alert').filter({ hasText: '所选编码无法解码文件' }).waitFor();
  const previewFailures = [];
  for (const locale of ['zh', 'en', 'ja', 'fr', 'de']) {
    await page.evaluate(locale => window.setWizardLocale(locale), locale);
    const error = await page.locator('.preview-message--error p').innerText();
    assert.ok(error.includes('所选编码无法解码文件'), 'unknown diagnostic details remain intact');
    previewFailures.push(error);
  }
  assert.equal(new Set(previewFailures).size, 5, 'existing preview failure changes language without rerunning import');
  await page.evaluate(() => window.setWizardLocale('zh'));
  assert.equal(await next().isDisabled(), true, 'failed new preview invalidates the previous successful result');
  assert.equal(await dialog().locator('.preview-row').count(), 0);
  await dialog().getByRole('region', { name: '分段效果与理由' }).getByRole('button', { name: '重新预览' }).click();
  await dialog().getByText('共 2 段', { exact: true }).waitFor();
  const selectSizes = await dialog().locator('select').evaluateAll(elements => elements.map(element => element.getBoundingClientRect().height));
  assert.ok(selectSizes.every(height => height >= 34), `macOS-styled selects keep readable control height: ${selectSizes}`);
  await screenshot('02-seg-preview');
  await page.evaluate(() => { document.documentElement.dataset.theme = 'eye'; });
  await screenshot('03-eye-preview');
  await page.evaluate(() => { delete document.documentElement.dataset.theme; });

  for (let index = 1; index <= 3; index++) {
    await next().click();
    await dialog().getByRole('heading', { name: `导入译本 ${index}`, exact: true }).waitFor();
    assert.equal(await dialog().getByLabel('编码', { exact: true }).inputValue(), 'auto', 'each translation starts with independent detection');
    await choose('选择文件…', `/fixtures/translation-${index}.txt`);
    await dialog().getByText('共 45 段', { exact: true }).waitFor();
    const label = ['UTF-16 LE', 'Windows-1252', 'UTF-8 BOM'][index - 1];
    await dialog().getByLabel('解码结果').getByText(`识别编码：${label}`, { exact: true }).waitFor();
    await dialog().getByLabel('译本名称', { exact: true }).fill(`译者 ${index}`);
    if (index === 2) await dialog().getByLabel('文本语言', { exact: true }).selectOption('fr');
  }
  await next().click();
  await dialog().getByRole('heading', { name: '确认并创建工程', exact: true }).waitFor();
  assert.equal(await dialog().locator('.document-summary').count(), 4);
  await dialog().getByRole('button', { name: '修改译本 1', exact: true }).click();
  assert.equal(await dialog().getByLabel('译本名称', { exact: true }).inputValue(), '译者 1');
  assert.equal(await dialog().getByText('共 45 段', { exact: true }).count(), 1, 'returning to previous steps preserves the draft and preview');
  await navigation().getByRole('button', { name: /确认创建/ }).click();
  await navigation().getByRole('button', { name: '增加译本', exact: true }).click();
  await dialog().getByRole('heading', { name: '导入译本 4', exact: true }).waitFor();
  assert.equal(await navigation().locator('li').count(), 7);
  assert.equal(await next().isDisabled(), true);
  await page.evaluate(() => { window.importWizardAudit.holdPreview = true; });
  await choose('选择文件…', '/fixtures/accidental-translation.txt');
  await page.waitForFunction(() => window.importWizardAudit.pendingPreviews.length === 1);
  await navigation().getByRole('button', { name: '删除译本 4', exact: true }).click();
  await dialog().getByRole('heading', { name: '导入译本 3', exact: true }).waitFor();
  await page.evaluate(() => window.importWizardAudit.pendingPreviews.shift().resolve());
  assert.equal(await navigation().locator('li').count(), 6, 'late preview results cannot restore a removed translation');
  assert.equal(await dialog().getByLabel('译本名称', { exact: true }).inputValue(), '译者 3');
  await next().click();
  await screenshot('04-confirm-three-translations');

  await page.evaluate(() => { window.importWizardAudit.beforeCreate = false; });
  await dialog().getByRole('button', { name: '创建并打开工程', exact: true }).click();
  assert.equal(await page.evaluate(() => window.importWizardAudit.creates.length), 0, 'beforeCreate cancellation does not write a project');
  await page.evaluate(() => { window.importWizardAudit.beforeCreate = true; window.importWizardAudit.holdCreate = true; });
  await dialog().getByRole('button', { name: '创建并打开工程', exact: true }).click();
  await page.waitForFunction(() => !!window.importWizardAudit.pendingCreate);
  assert.equal(await dialog().getByRole('button', { name: '取消', exact: true }).isDisabled(), true);
  assert.equal(await dialog().getByRole('button', { name: '关闭导入向导' }).isDisabled(), true);
  await page.keyboard.press('Escape');
  assert.equal(await dialog().count(), 1, 'creating cannot be cancelled or dismissed');
  assert.equal(await page.evaluate(() => window.importWizardAudit.creates.length), 1);
  await page.evaluate(() => window.importWizardAudit.pendingCreate.reject({ message: '磁盘写入失败' }));
  await dialog().getByRole('alert').filter({ hasText: '创建失败：磁盘写入失败' }).waitFor();
  assert.equal(await dialog().locator('.document-summary').count(), 4, 'creation errors retain every document draft');
  await page.evaluate(() => { window.importWizardAudit.holdCreate = false; });
  await dialog().getByRole('button', { name: '创建并打开工程', exact: true }).click();
  await dialog().waitFor({ state: 'hidden' });
  const audit = await page.evaluate(() => ({ creates: window.importWizardAudit.creates, created: window.importWizardAudit.created, busy: window.importWizardAudit.busy }));
  assert.equal(audit.creates.length, 2);
  assert.deepEqual(audit.creates[0], audit.creates[1], 'retry submits the same reviewed inputs');
  const request = audit.creates[1];
  assert.equal(request.project_path, '/Users/test/Projects/三个译本对照.jm');
  assert.equal(request.source.title, '中文原文');
  assert.equal(request.source.profile.encoding, 'gb18030');
  assert.equal(request.source.profile.segmentation_mode, 'legacy_tagged_line');
  assert.equal(request.source.expected_sha256, '/fixtures/source.txt|gb18030|legacy_tagged_line');
  assert.equal(request.target.title, '译者 1');
  assert.deepEqual([request.target, ...request.additional_targets].map(side => side.profile.encoding), ['utf-16le', 'windows-1252', 'utf8-bom'], 'creation uses the resolved preview encodings, not the UTF-8 request placeholder');
  assert.ok([request.target, ...request.additional_targets].every(side => !('auto_detect_encoding' in side)));
  assert.deepEqual(request.additional_targets.map(target => [target.title, target.language_id]), [['译者 2', 'fr'], ['译者 3', 'en']]);
  assert.ok([request.source, request.target, ...request.additional_targets].every(side => !!side.expected_sha256));
  assert.equal(audit.created.length, 1);
  assert.equal(audit.created[0].path, request.project_path);
  assert.equal(audit.busy, false);

  // Closing and reopening invalidates asynchronous results from the old session.
  await open();
  assert.equal(await navigation().locator('li').count(), 4);
  assert.ok((await navigation().innerText()).includes('译本 1'), 'reopened default translation is numbered from one');
  await dialog().getByLabel('项目名称').fill('关闭竞态');
  await choose('浏览…', 'C:\\Projects\\');
  assert.ok((await dialog().innerText()).includes('C:\\Projects\\关闭竞态.jm'), 'Windows directory separators remain supported');
  await next().click();
  await page.evaluate(() => { window.importWizardAudit.holdPreview = true; });
  await choose('选择文件…', '/fixtures/old-session.txt');
  await page.waitForFunction(() => window.importWizardAudit.pendingPreviews.length === 1);
  await dialog().getByRole('button', { name: '取消', exact: true }).click();
  await open();
  await page.evaluate(() => window.importWizardAudit.pendingPreviews.shift().resolve());
  assert.equal(await dialog().getByLabel('项目名称').inputValue(), '');
  assert.equal(await next().isDisabled(), true);
  await dialog().getByLabel('项目名称').fill('空文本');
  await choose('浏览…', '/');
  await next().click();
  await page.evaluate(() => { window.importWizardAudit.emptyPreview = true; });
  await choose('选择文件…', '/fixtures/empty.txt');
  await dialog().getByText('没有可用段落。请检查文件内容、编码或分段方式。', { exact: true }).waitFor();
  assert.equal(await next().isDisabled(), true, 'zero-segment previews cannot proceed');
  await page.setViewportSize({ width: 600, height: 760 });
  assert.ok(await dialog().evaluate(element => element.getBoundingClientRect().right <= innerWidth));
  assert.ok(await dialog().locator('.wizard-footer').evaluate(element => element.getBoundingClientRect().bottom <= innerHeight));
  await screenshot('05-small-window');
  await dialog().getByRole('button', { name: '取消', exact: true }).click();
  assert.equal(await page.locator('#open-wizard').evaluate(element => element === document.activeElement), true, 'closing restores focus to the launch control');
  assert.deepEqual(errors, []);
  console.log(JSON.stringify({ browser: browserName, result: 'PASS', requestDocuments: 4, checks: ['five-language live locale switching', 'delete unvisited and loading translations', 'independent automatic decoding', 'manual encoding and stale detection', 'resolved encodings on creation', 'dynamic import steps', 'preview reasons and cleanup', 'paged previews', 'stale and failed preview rejection', 'three translations and SHA submission', 'creation failure/retry/lock', 'session invalidation', 'zero-segment guard', 'theme and focus'], screenshotDirectory }));
} catch (error) {
  if (errors.length) console.error(JSON.stringify({ pageErrors: errors }, null, 2));
  console.error(JSON.stringify({ bodyText: (await page.locator('body').innerText()).slice(0, 2000) }, null, 2));
  throw error;
} finally {
  await browser.close();
}
