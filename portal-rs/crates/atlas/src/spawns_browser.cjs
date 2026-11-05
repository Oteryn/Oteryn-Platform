'use strict';
// Optional PostgreSQL-backed browser qualification; dependencies are supplied by the caller.
const {chromium} = require(process.env.PORTAL_ATLAS_PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
(async()=>{
  const browser=await chromium.launch({headless:true,executablePath:process.env.PORTAL_ATLAS_CHROMIUM});
  try {
    const context=await browser.newContext({viewport:{width:360,height:800}});
    const [origin,token]=process.argv.slice(2);
    await context.addCookies([{name:'__Host-oteryn_session',value:token,domain:new URL(origin).hostname,path:'/',secure:true,httpOnly:true,sameSite:'Lax'}]);
    const page=await context.newPage(), errors=[];
    page.on('pageerror',error=>errors.push(error.message));
    for(const lang of ['en','pl']) {
      const response=await page.goto(`${origin}/${lang}/atlas?x=100&y=200&z=7&zoom=0`);
      assert.equal(response.status(),200);
      assert.match(response.headers()['content-security-policy'],/connect-src 'self'/);
      await page.waitForFunction(()=>document.querySelector('.spawn-panel>p')?.textContent.includes(': 2;'));
      assert.equal(await page.locator('.spawn-panel>ul').first().locator(':scope >li').count(),1);
      assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
      const place=page.locator('.spawn-panel nav a');
      assert.match(await place.getAttribute('href'),/x=100.*y=200.*z=7.*zoom=0/);
      await page.locator('#in').click();
      await page.waitForFunction(()=>document.querySelector('.spawn-panel>ul').children.length===2);
      await page.locator('#atlas').focus(); await page.keyboard.press('ArrowRight');
      assert.match(page.url(),/x=164/);
      await page.keyboard.press('ArrowLeft');
    }
    await page.goto(`${origin}/en/atlas?x=100&y=200&z=7&zoom=0`);
    await page.getByLabel('Marker label (1–80 characters)').fill('Browser marker');
    await page.getByRole('button',{name:'Save marker at map centre'}).click();
    await page.getByRole('link',{name:'Browser marker',exact:true}).waitFor();
    await page.reload();
    await page.getByRole('link',{name:'Browser marker',exact:true}).waitFor();
    await page.getByRole('button',{name:'Delete marker'}).click();
    await page.getByRole('link',{name:'Browser marker',exact:true}).waitFor({state:'detached'});
    const refused=await page.evaluate(async()=> (await fetch('/atlas/map-reports',{method:'POST',headers:{'Content-Type':'application/json'},body:'{}'})).status);
    assert.equal(refused,403);
    // The fixture has already reached its five-report limit: the UI must surface refusal.
    await page.getByLabel('Describe the error (10–2000 characters)').fill('The map floor is incorrect.');
    await page.getByRole('button',{name:'Send report'}).click();
    await page.getByText('Unavailable or invalid input. Please try again.',{exact:true}).waitFor();
    assert.deepEqual(errors,[]);
    console.log('Browser: 360 px, PL/EN, grouping, viewport counts, place link, keyboard, private marker persistence/deletion, CSRF and report refusal passed');
  } finally { await browser.close(); }
})().catch(error=>{console.error(error);process.exitCode=1;});
