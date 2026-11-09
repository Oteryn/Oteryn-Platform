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
    for (const [index, lang] of ['pl', 'en'].entries()) {
      await page.goto(`${process.env.PORTAL_BROWSER_URL}/${lang}/shop/coins`);
      assert.ok((await page.locator('main form').textContent()).includes(lang === 'pl' ? '2,50 zł' : 'PLN 2.50'));
      assert.equal(await page.locator('main form button').textContent(), lang === 'pl' ? 'Kup' : 'Buy');
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
      await page.locator('main form button').click();
      await page.waitForURL(`**/${lang}/shop/test-checkout/*`);
      assert.ok((await page.locator('main').textContent()).includes(lang === 'pl' ? 'TRYB TESTOWY – żadne prawdziwe pieniądze nie są pobierane' : 'TEST MODE – no real money is charged'));
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
      await page.keyboard.press('Tab');
      assert.equal(await page.evaluate(() => document.activeElement.tagName), 'A');
      const result = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa']).analyze();
      assert.deepEqual(result.violations.map(v => ({ id: v.id, targets: v.nodes.map(n => n.target) })), []);
      await page.locator('button[value=pay]').focus();
      await page.keyboard.press('Enter');
      await page.waitForURL(`**/${lang}/shop/coins?checkout=*`);
      const expected = String((index + 1) * 100);
      for (let attempt = 0; attempt < 30; attempt++) {
        if (await page.locator('main section dd').first().textContent() === expected) break;
        await page.waitForTimeout(100);
        await page.reload();
      }
      assert.equal(await page.locator('main section dd').first().textContent(), expected);
    }
    await page.goto(`${process.env.PORTAL_BROWSER_URL}/en/shop/transfer`);
    const ownId = page.getByLabel('Your transfer ID', { exact: true });
    assert.equal(await ownId.getAttribute('readonly'), '');
    assert.match(await ownId.inputValue(), /^[0-9a-f-]{36}$/);
    await page.locator('[name=recipient]').fill('invalid');
    assert.equal(await page.locator('[name=recipient]').evaluate(input => input.checkValidity()), false);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
    const transferAxe = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa']).analyze();
    assert.deepEqual(transferAxe.violations.map(v => v.id), []);
    await page.locator('[name=recipient]').fill(process.env.PORTAL_BROWSER_RECIPIENT);
    await page.locator('[name=amount]').fill('20');
    await page.locator('main button').click();
    await page.waitForURL('**/en/shop/history');
    assert.ok((await page.locator('main').textContent()).includes('-20 / 0'));
    console.log('360px: PL+EN top-up -> test bank -> keyboard pay -> real worker credit -> transfer; axe 0 violations. Purchase remains blocked on authoritative ownership.');
    await context.close();
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
