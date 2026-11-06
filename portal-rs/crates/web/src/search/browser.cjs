/* Optional browser validation uses externally installed development dependencies. */
const assert = require('node:assert/strict');
const { createRequire } = require('node:module');
const modules = createRequire(process.env.PORTAL_BROWSER_MODULES + '/package.json');
const { chromium } = modules('playwright-core');
const AxeBuilder = modules('@axe-core/playwright').default;
(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || '/usr/bin/chromium', headless: true, args: ['--no-sandbox'] });
  try {
    for (const width of [360, 1280]) for (const colorScheme of ['light', 'dark']) {
      const context = await browser.newContext({ viewport: { width, height: 800 }, colorScheme });
      const page = await context.newPage();
      for (const lang of ['en', 'pl']) {
        assert.equal((await page.goto(`${process.env.PORTAL_BROWSER_URL}/${lang}/search`)).status(), 200);
        await page.locator('#q').focus();
        await page.keyboard.type('dragon');
        await page.keyboard.press('Tab');
        assert.equal(await page.evaluate(() => document.activeElement.id), 'kind');
        await page.keyboard.press('Tab');
        await Promise.all([page.waitForURL('**/search?q=dragon&kind='), page.keyboard.press('Enter')]);
        assert.equal(await page.locator('article a').count(), 1);
        assert.equal(await page.locator('article a').textContent(), 'Dragon <test>');
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, `${width}px overflow`);
        const result = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa']).analyze();
        assert.deepEqual(result.violations.map(v => ({ id: v.id, targets: v.nodes.map(n => n.target) })), []);
      }
      await context.close();
      console.log(`${width}px ${colorScheme}: PL/EN keyboard search, escaped result, no overflow; axe 0 violations`);
    }
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
