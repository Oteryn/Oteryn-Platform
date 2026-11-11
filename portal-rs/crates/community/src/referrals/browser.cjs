const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { createRequire } = require('node:module');
const modules = createRequire(path.join(process.argv[3], 'package.json'));
const browserModules = createRequire(path.join(process.argv[4] || process.argv[3], 'package.json'));
const { chromium } = browserModules('playwright');
const AxeBuilder = modules('@axe-core/playwright').default;
(async () => {
  const browser = await chromium.launch({ headless: true, args: ['--no-sandbox'] });
  try {
    const context = await browser.newContext({ viewport: { width: 360, height: 800 } });
    const page = await context.newPage();
    await page.route('https://portal.example.test/**', async route => {
      const url = new URL(route.request().url());
      if (url.pathname.startsWith('/static/')) {
        const root = path.resolve(__dirname, '../../../../static');
        const file = path.resolve(root, '.' + url.pathname.slice('/static'.length));
        assert.ok(file.startsWith(root + path.sep));
        if (!fs.existsSync(file)) { await route.fulfill({ status: 404, body: '' }); return; }
        const contentType = file.endsWith('.css') ? 'text/css' : file.endsWith('.woff2') ? 'font/woff2' : 'application/octet-stream';
        await route.fulfill({ contentType, body: fs.readFileSync(file) });
      } else {
        const lang = url.pathname.split('/')[1];
        if (!['pl', 'en'].includes(lang)) { await route.fulfill({ status: 404, body: '' }); return; }
        await route.fulfill({ contentType: 'text/html', body: fs.readFileSync(path.join(process.argv[2], lang + '.html')) });
      }
    });
    for (const lang of ['pl', 'en']) {
      await page.goto('https://portal.example.test/' + lang + '/account/referrals');
      await page.locator('#new-email').waitFor();
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, '360px overflow');
      for (const expected of ['invite-link', 'new-email']) {
        await page.keyboard.press('Tab');
        assert.equal(await page.evaluate(() => document.activeElement.id), expected);
        assert.equal(await page.evaluate(() => document.activeElement.matches(':focus-visible')), true);
      }
      await page.keyboard.press('Tab');
      assert.equal(await page.evaluate(() => document.activeElement.tagName), 'BUTTON');
      await page.keyboard.press('Tab');
      assert.equal(await page.evaluate(() => document.activeElement.id), 'buddy-email');
      const result = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa']).analyze();
      assert.deepEqual(result.violations.map(v => ({ id: v.id, targets: v.nodes.map(n => n.target) })), []);
      await page.screenshot({ path: path.join(process.argv[2], lang + '-360.png'), fullPage: true });
    }
    console.log('360px: 2 real-router referral pages, keyboard focus, labels and contrast; axe 0 violations');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
