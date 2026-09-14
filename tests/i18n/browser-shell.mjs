// Real app regression: footer labels, existing notifications and side panels
// follow live locale changes. Browser-only demo; no native project is modified.
import assert from 'node:assert/strict';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const baseUrl = process.env.I18N_BASE_URL || 'http://127.0.0.1:1420';
const appSource = await (await fetch(`${baseUrl}/src/App.vue`)).text();
const i18nUrl = appSource.match(/from "([^"]*\/i18n\/index\.ts[^"]*)"/)?.[1];
assert.ok(i18nUrl, 'use the same translation module as the real app, including Vite HMR version');
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1500, height: 1000 } });
const errors = [];
page.on('pageerror', error => errors.push(error.message));
await page.addInitScript(() => {
  localStorage.setItem('jueming-parallel-tutorial-launched-v2', '1');
  localStorage.setItem('jueming-ui-locale', 'zh');
});
const translate = (key) => page.evaluate(async ({ key, url }) => (await import(url)).t(key), { key, url: i18nUrl });
const locale = (value) => page.evaluate(async ({ value, url }) => {
  const { setUiLocale } = await import(url);
  setUiLocale(value);
}, { value, url: i18nUrl });
try {
  await page.goto(baseUrl);
  await page.locator('.bottom-status').waitFor();
  await locale('zh');
  // Produce one notification, then translate that same event without retriggering it.
  await page.keyboard.press('Control+s');
  const notifications = [];
  for (const value of ['zh', 'en', 'ja', 'fr', 'de']) {
    await locale(value);
    const expected = await translate('shellFooterProject');
    await page.waitForFunction(expected => document.querySelector('.footer-context-label')?.textContent === expected, expected);
    for (const key of ['shellFooterProject', 'shellFooterFiles', 'shellFooterAlignment']) {
      assert.ok((await page.locator('.footer-context').innerText()).includes(await translate(key)), `${value}: ${key}`);
    }
    assert.equal(await page.locator('[role="progressbar"]').getAttribute('aria-label'), await translate('shellFooterProgressAria'));
    const notification = await page.locator('.footer-save-state').innerText();
    notifications.push(notification);
    if (['en', 'fr', 'de'].includes(value)) assert.ok(!/\p{Script=Han}/u.test(notification), `${value}: untranslated footer notification`);
  }
  assert.equal(new Set(notifications).size, 5, 'the same saved notification updates in every locale');
  // Open Settings through the actual app, then change language via its control.
  await locale('en');
  await page.getByRole('button', { name: await translate('navSettings'), exact: true }).first().click();
  await page.getByRole('button', { name: 'General', exact: true }).click();
  const language = page.locator('select').filter({ has: page.locator('option[value="zh-CN"]') });
  await language.waitFor();
  for (const value of ['fr', 'de', 'ja', 'en', 'zh-CN']) {
    await language.selectOption(value);
    const expected = await translate('shellFooterProject');
    assert.equal(await page.locator('.footer-context-label').first().innerText(), expected);
  }
  for (const value of ['zh', 'en', 'ja', 'fr', 'de']) {
    await locale(value);
    for (const [id, selector] of [
      ['search', '.search-replace-workspace'], ['bookmarks', '.bookmarks-workspace'],
      ['history', '.history-workspace'], ['pipeline', '.pipeline-workspace'],
    ]) {
      await page.locator(`[data-nav-id="${id}"]`).click();
      const workspace = page.locator(selector);
      await workspace.waitFor({ state: 'visible' });
      if (['en', 'fr', 'de'].includes(value)) {
        const labels = await workspace.locator('button, input[placeholder], select, [aria-label]').evaluateAll(elements => elements.map(element => {
          return [element.tagName === 'BUTTON' ? element.textContent : '', element.getAttribute('placeholder'), element.getAttribute('aria-label')].filter(Boolean).join(' ');
        }).join('\n'));
        assert.ok(!/\p{Script=Han}/u.test(labels), `${value}/${id}: untranslated controls: ${labels}`);
      }
    }
  }
  await page.locator('[data-nav-id="project"]').click();
  await page.getByRole('button', { name: await translate('shellEnterDemo'), exact: true }).click();
  const tutorial = page.locator('.parallel-tutorial__card');
  await tutorial.waitFor();
  const tutorialTitles = [];
  for (const value of ['zh', 'en', 'ja', 'fr', 'de']) {
    await locale(value);
    tutorialTitles.push(await tutorial.locator('h2, h3').innerText());
    if (['en', 'fr', 'de'].includes(value)) assert.ok(!/\p{Script=Han}/u.test(await tutorial.innerText()), `${value}: untranslated tutorial`);
  }
  assert.equal(new Set(tutorialTitles).size, 5, 'tutorial text follows live locale changes');
  assert.deepEqual(errors, [], 'app has no browser runtime errors');
  console.log('Shell i18n browser regression passed: five locales, footer/live notifications and search/bookmarks/history/pipeline navigation');
} finally {
  await browser.close();
}
