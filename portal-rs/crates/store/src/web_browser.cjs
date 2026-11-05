const assert = require('node:assert/strict');
const { createRequire } = require('node:module');
const modules = createRequire(process.env.PORTAL_BROWSER_MODULES + '/package.json');
const { chromium } = modules('playwright-core');
const AxeBuilder = modules('@axe-core/playwright').default;
(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || '/usr/bin/chromium', headless: true, args: ['--no-sandbox'] });
  try {
    const context = await browser.newContext({ viewport: { width: 360, height: 800 } });
    const [name, value] = process.env.PORTAL_BROWSER_COOKIE.split('=');
    await context.addCookies([{ name, value, url: process.env.PORTAL_BROWSER_URL.replace('http:', 'https:'), secure: true, httpOnly: true, sameSite: 'Lax' }]);
    const page = await context.newPage();
    for (const lang of ['pl', 'en']) {
      for (const path of ['shop', 'shop/coins', 'shop/history', 'shop/transfer']) {
        const response = await page.goto(process.env.PORTAL_BROWSER_URL + '/' + lang + '/' + path);
        assert.equal(response.status(), 200);
        assert.equal(await page.locator('html').getAttribute('lang'), lang);
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, '360px overflow');
        await page.keyboard.press('Tab');
        assert.equal(await page.evaluate(() => document.activeElement.tagName), 'A');
        const result = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa']).analyze();
        assert.deepEqual(result.violations.map(v => ({ id:v.id, targets:v.nodes.map(n => n.target) })), []);
      }
    }
    await page.goto(process.env.PORTAL_BROWSER_URL + '/en/shop/transfer');
    await page.locator('[name=recipient]').fill(process.env.PORTAL_BROWSER_RECIPIENT);
    await page.locator('[name=amount]').fill('20');
    const [transferResponse] = await Promise.all([page.waitForResponse(r => r.request().method() === 'POST'), page.locator('button').click()]);
    assert.equal(transferResponse.status(), 303);
    await page.waitForURL('**/en/shop/history');
    assert.ok((await page.locator('main').textContent()).includes('-20 / 0'));
    await page.goto(process.env.PORTAL_BROWSER_URL + '/en/shop/transfer');
    await page.locator('[name=recipient]').fill('00000000-0000-0000-0000-000000000000');
    await page.locator('[name=amount]').fill('1');
    const [response] = await Promise.all([page.waitForResponse(r => r.request().method() === 'POST'), page.locator('button').click()]);
    assert.equal(response.status(), 422);
    await page.locator('[role=status]').waitFor();
    assert.equal(await page.locator('[role=status]').textContent(), 'Check the submitted values.');
    console.log('360px: 8 authenticated shop pages, keyboard links, successful transfer and refusal, axe 0 violations');
    await context.close();
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
