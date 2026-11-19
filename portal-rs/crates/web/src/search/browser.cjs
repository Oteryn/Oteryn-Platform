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
        for (const [fixture, expectedStatus] of [['browser-shell', 503], ['browser-headless', 200]]) {
          const response = await page.goto(`${process.env.PORTAL_BROWSER_URL}/${fixture}/${lang}`);
          assert.equal(response.status(), expectedStatus);
          assert.equal(await page.locator('script[src="/static/thais/home.js"]').count(), 0);
          assert.equal(await page.locator('link[href="/static/css/tokens.css"]').count(), 1);
          assert.notEqual(await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--op-text').trim()), '');
          assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, 'closed navigation overflow');
          await page.keyboard.press('Tab');
          assert.equal(await page.evaluate(() => document.activeElement.matches(':focus-visible') && getComputedStyle(document.activeElement).outlineStyle !== 'none'), true);
          const menu = page.locator('details.om-fallback-menu');
          const summary = menu.locator(':scope > summary');
          const firstLink = menu.locator('a[href$="#profesje"]');
          assert.equal(await firstLink.isVisible(), false);
          await summary.focus();
          await page.keyboard.press('Enter');
          assert.equal(await firstLink.isVisible(), true);
          if (fixture === 'browser-shell') {
            assert.equal(await menu.locator('#portal-community-panel a[href$="/online"]').isVisible(), true);
          }
          const bounds = await menu.locator('nav').boundingBox();
          assert.ok(bounds && bounds.x >= 0 && bounds.x + bounds.width <= width, 'open navigation fits viewport');
          await page.keyboard.press('Tab');
          assert.equal(await firstLink.evaluate(n => n === document.activeElement), true);
          await summary.focus();
          await page.keyboard.press('Enter');
          assert.equal(await firstLink.isVisible(), false);
        }

      }
      await context.close();
      console.log(`${width}px ${colorScheme}: PL/EN keyboard search, escaped result, no overflow; axe 0 violations`);
    }
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
