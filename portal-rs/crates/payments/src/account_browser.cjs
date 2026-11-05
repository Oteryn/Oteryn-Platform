/* Optional real-browser check, invoked by the ignored account panel test. */
const assert = require('node:assert/strict');
const { createRequire } = require('node:module');
const modules = createRequire(process.env.PORTAL_BROWSER_MODULES + '/package.json');
const { chromium } = modules('playwright-core');
const AxeBuilder = modules('@axe-core/playwright').default;
(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || '/usr/bin/chromium', headless: true, args: ['--no-sandbox'] });
  try {
    const context = await browser.newContext({ viewport: { width: 360, height: 800 }, extraHTTPHeaders: { Cookie: process.env.PORTAL_BROWSER_COOKIE } });
    const page = await context.newPage();
    for (const lang of ['pl', 'en']) {
      for (const path of ['premium', 'payments', 'payments/receipts/' + process.env.PORTAL_BROWSER_RECEIPT]) {
        const response = await page.goto(process.env.PORTAL_BROWSER_URL + '/' + lang + '/panel/' + path);
        assert.equal(response.status(), 200);
        assert.equal(await page.locator('html').getAttribute('lang'), lang);
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, '360px overflow');
        assert.equal(await page.locator('h1').count(), 1);
        await page.keyboard.press('Tab');
        assert.equal(await page.evaluate(() => document.activeElement.tagName), 'A');
        const result = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa']).analyze();
        assert.deepEqual(result.violations.map(v => ({ id:v.id, targets:v.nodes.map(n => n.target) })), []);
        if (path.includes('receipts')) {
          await page.emulateMedia({ media: 'print' });
          assert.equal(await page.locator('article').isVisible(), true);
          await page.emulateMedia({ media: 'screen' });
        }
      }
    }
    console.log('360px: 6 authenticated pages, keyboard links, print receipt, axe 0 violations');
    await context.close();
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
