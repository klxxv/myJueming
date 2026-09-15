// Real browser UI: theme switching, persistence, mode transitions, and readable tokens.
// Uses the read-only browser demo; no native project or canonical data is modified.
import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
const { chromium, webkit } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const browser = await (process.env.THEME_BROWSER === 'webkit' ? webkit : chromium).launch({ headless: true });
const page = await browser.newPage({ locale: 'en-US', viewport: { width: 1440, height: 1000 } });
const output = process.env.THEME_SCREENSHOT_DIR || join(tmpdir(), 'jueming-theme-qa');
await mkdir(output, { recursive: true });
const errors = [];
page.on('pageerror', error => errors.push(error.message));
await page.addInitScript(() => localStorage.setItem('jueming-parallel-tutorial-launched-v2', '1'));
const settings = () => page.locator('.app-toolbar').getByRole('button', { name: 'Settings', exact: true }).click();
const themeSelect = () => page.locator('select').filter({ has: page.locator('option[value="dark"]') });
const shot = async name => { await page.waitForTimeout(350); await page.screenshot({ path: join(output, `${name}.png`) }); };
const luminance = value => {
  const channels = value.match(/[\d.]+/g).slice(0, 3).map(Number).map(v => { v /= 255; return v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4; });
  return channels[0] * .2126 + channels[1] * .7152 + channels[2] * .0722;
};
try {
  await page.goto(process.env.THEME_BASE_URL || 'http://127.0.0.1:1420');
  await settings();
  await page.emulateMedia({ colorScheme: 'light' });
  await themeSelect().selectOption('system');
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'light');
  await page.waitForFunction(() => JSON.parse(localStorage.getItem('jueming-app-settings-v1'))?.device.appearance.theme === 'system');
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark');
  await shot('system-dark-settings');
  await page.reload();
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark');
  await settings();
  assert.equal(await themeSelect().inputValue(), 'system');
  await page.emulateMedia({ colorScheme: 'light' });
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'light');
  await shot('system-light-settings');
  const pairSelect = () => page.locator('select').filter({ has: page.locator('option[value="eye"]') }).filter({ hasNot: page.locator('option[value="system"]') });
  await pairSelect().selectOption('eye');
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'eye');
  await page.waitForFunction(() => JSON.parse(localStorage.getItem('jueming-app-settings-v1'))?.device.appearance.systemLightTheme === 'eye');
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark');
  await page.reload();
  await settings();
  assert.equal(await pairSelect().inputValue(), 'eye');
  await page.emulateMedia({ colorScheme: 'light' });
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'eye');
  await shot('system-eye-settings');
  await pairSelect().selectOption('light');
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'light');

  for (const theme of ['light', 'eye', 'dark']) {
    await themeSelect().selectOption(theme);
    await page.waitForFunction(t => document.documentElement.dataset.theme === t, theme);
    await page.waitForFunction(t => JSON.parse(localStorage.getItem('jueming-app-settings-v1'))?.device.appearance.theme === t, theme);
    await page.emulateMedia({ colorScheme: theme === 'dark' ? 'light' : 'dark' });
    await page.waitForTimeout(100);
    assert.equal(await page.locator('html').getAttribute('data-theme'), theme, 'manual themes ignore system changes');
    const layout = await page.locator('.settings-split').evaluate(el => getComputedStyle(el).display);
    assert.equal(layout, 'grid', 'Tailwind utilities must compile in the actual app');
    await shot(`${theme}-settings`);
  }
  await page.reload();
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'dark');
  await settings();
  await page.locator('.settings-sidebar').getByRole('button', { name: 'Accessibility', exact: true }).click();
  const contrast = page.locator('select').filter({ has: page.locator('option[value="increased"]') });
  await contrast.selectOption('increased');
  assert.equal(await page.locator('html').evaluate(el => getComputedStyle(el).getPropertyValue('--ink-900').trim()), '#ffffff');
  await shot('dark-increased-contrast');
  await contrast.selectOption('system');
  await page.emulateMedia({ contrast: 'more' });
  assert.equal(await page.locator('html').evaluate(el => getComputedStyle(el).getPropertyValue('--ink-900').trim()), '#ffffff');
  await page.emulateMedia({ contrast: 'no-preference' });
  await page.locator('[data-nav-id="parallel"]').click();
  for (const mode of ['review', 'edit', 'order']) {
    await page.locator('.mode-control select').selectOption(mode);
    await page.waitForTimeout(250);
    assert.ok(await page.locator('.segment-card').count() > 0);
    await shot(`dark-${mode}`);
  }
  const tracks = page.locator('.progress-minimap-track');
  const viewport = page.locator('.aligned-workspace-viewport');
  assert.equal(await tracks.count(), 2, 'both language sides expose reading progress');
  await tracks.first().focus();
  await page.keyboard.press('End');
  await page.waitForFunction(() => {
    const el = document.querySelector('.aligned-workspace-viewport');
    return el.scrollTop > 0 && el.scrollHeight - el.clientHeight - el.scrollTop < 2;
  });
  await tracks.nth(1).focus();
  await page.keyboard.press('Home');
  await page.waitForFunction(() => document.querySelector('.aligned-workspace-viewport').scrollTop === 0);
  const track = await tracks.first().boundingBox();
  assert.ok(track);
  await page.mouse.move(track.x + 6, track.y + track.height / 2);
  await page.mouse.down();
  await page.mouse.move(track.x + track.width * .6, track.y + track.height / 2);
  assert.equal(await viewport.evaluate(el => el.scrollTop), 0, 'drag previews until released');
  await page.mouse.up();
  await page.waitForFunction(() => document.querySelector('.aligned-workspace-viewport').scrollTop > 0);
  await tracks.first().focus();
  await page.keyboard.press('Home');
  await page.waitForFunction(() => document.querySelector('.aligned-workspace-viewport').scrollTop === 0);
  const paints = await page.locator('.segment-card__text').first().evaluate(el => {
    const root = getComputedStyle(document.documentElement);
    return { foreground: getComputedStyle(el).color, surface: root.getPropertyValue('--surface-green-selected').trim() };
  });
  const rgb = hex => `rgb(${hex.match(/[\da-f]{2}/gi).map(x => parseInt(x, 16)).join(',')})`;
  assert.ok((luminance(paints.foreground) + .05) / (luminance(rgb(paints.surface)) + .05) >= 4.5, 'reading contrast on selected surface');
  await page.getByRole('button', { name: 'Annotations', exact: true }).click();
  await shot('dark-annotations');
  await page.locator('[data-nav-id="search"]').click();
  await shot('dark-search');
  await page.locator('[data-nav-id="history"]').click();
  await shot('dark-history');
  await page.getByRole('button', { name: 'New', exact: true }).click();
  await shot('dark-new-project');
  await page.keyboard.press('Escape');
  assert.deepEqual(errors, []);
  console.log(`Theme UI regression passed. Screenshots: ${output}`);
} finally { await browser.close(); }
