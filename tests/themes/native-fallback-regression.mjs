import assert from 'node:assert/strict';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  const base = process.env.THEME_BASE_URL || 'http://127.0.0.1:1420';
  await page.route('**/theme-test', route => route.fulfill({ contentType: 'text/html', body: '<div id="app"></div>' }));
  await page.goto(`${base}/theme-test`);
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.evaluate(async () => {
    const { createPinia } = await import('/node_modules/.vite/deps/pinia.js');
    const { useAppSettingsStore } = await import('/src/stores/app-settings.ts');
    const { createDefaultAppSettings } = await import('/src/settings/schema.ts');
    const saved = createDefaultAppSettings(false);
    saved.device.appearance.theme = 'system';
    saved.device.appearance.systemLightTheme = 'eye';
    window.store = useAppSettingsStore(createPinia());
    window.store.configure({ detectedMacOS: false, onStatus() {}, kernelClient: {
      loadAppSettings: async () => saved,
      saveAppSettings: async () => {},
      observeSystemTheme: async notify => { window.nativeNotify = notify; notify('dark'); return () => {}; },
    }});
    await window.store.initializeSettings();
  });
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
  await page.emulateMedia({ colorScheme: 'light' });
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'eye');
  await page.evaluate(() => window.nativeNotify('dark'));
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
  await page.evaluate(() => {
    window.store.settings.device.appearance.theme = 'light';
    window.store.applyUiSettings();
    window.nativeNotify('dark');
  });
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'light');
  await page.evaluate(() => window.store.disposeRuntimeListeners());
  await page.evaluate(() => window.nativeNotify('eye'));
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'light');
  console.log('Cached native theme fallback, native events, manual override and listener disposal passed.');
} finally { await browser.close(); }
