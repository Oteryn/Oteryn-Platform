/* Static artifact checks; dependencies live outside the Rust workspace. */
const assert = require('node:assert/strict');
const fs = require('node:fs');
const { chromium } = require('/tmp/portal-t39b-browser/node_modules/playwright-core');
const AxeBuilder = require('/tmp/portal-t39b-browser/node_modules/@axe-core/playwright').default;
(async () => {
 const browser = await chromium.launch({executablePath:'/usr/bin/chromium',args:['--no-sandbox']});
 let scans = 0;
 try {
  for (const locale of ['pl','en']) for (const screen of ['home','account','bazaar','atlas','journal','statistics']) {
   for (const width of [360,1280]) {
    const page = await browser.newPage({viewport:{width,height:900}, javaScriptEnabled:false});
    await page.goto(`http://127.0.0.1:8099/portal-rs/templates/mockups/t39b/${screen}-${locale}.html`);
    assert.equal(await page.locator('html').getAttribute('lang'),locale);
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    assert.equal(await page.locator('button:not([disabled])').count(),0);
    assert.equal(await page.locator('form, script').count(),0);
    await page.keyboard.press('Tab');
    await page.keyboard.press('Enter');
    assert.equal(await page.evaluate(() => document.activeElement.id),'main');
    for (const summary of await page.locator('summary').all()) {
     await summary.focus();
     await page.keyboard.press('Enter');
    }
    assert.equal(await page.locator('details[open]').count(),3);
    assert.equal(await page.locator('[aria-invalid="true"]').getAttribute('aria-describedby'),'invalid-help');
    await page.screenshot({path:`docs/design/t39b/${screen}-${locale}-${width}.png`,fullPage:true});
    await page.close();
    // Axe injects its scanner; artifact itself still has no JavaScript.
    const context = await browser.newContext({viewport:{width,height:900}});
    const scan = await context.newPage();
    await scan.goto(`http://127.0.0.1:8099/portal-rs/templates/mockups/t39b/${screen}-${locale}.html`);
    await scan.locator('details').evaluateAll(nodes=>nodes.forEach(node=>node.open=true));
    const results = await new AxeBuilder({page:scan}).withTags(['wcag2a','wcag2aa','wcag21aa','wcag22aa']).analyze();
    assert.deepEqual(results.violations,[],`${screen}-${locale}-${width}`);
    scans++;
    await context.close();
   }
  }
  console.log(`PASS: 12 bilingual documents, 24 viewport checks and screenshots; ${scans} axe scans with zero violations; keyboard disclosures, skip links, disabled actions and invalid-input descriptions passed without artifact JavaScript.`);
 } finally { await browser.close(); }
})().catch(error=>{console.error(error);process.exitCode=1;});
