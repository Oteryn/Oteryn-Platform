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
    async function focused(locator) {
      assert.equal(await locator.evaluate(node => node === document.activeElement), true);
    }
    for (const width of [390, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      for (const lang of ['en', 'pl']) {
        const response = await page.goto(`${origin}/${lang}/atlas?x=100&y=200&z=7&zoom=0`);
        assert.equal(response.status(), 200);
        assert.match(response.headers()['content-security-policy'], /connect-src 'self'/);
        const labels = await (await page.request.get(`${origin}/atlas/spawns-text/${lang}`)).json();
        await page.waitForFunction(() => document.querySelector('.spawn-panel>p')?.textContent.includes(': 2;'));
        const groups = page.locator('.spawn-panel>ul').first().locator(':scope >li');
        assert.equal(await groups.count(), 1);
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
        const place = page.getByRole('link', { name: labels.place });
        assert.match(await place.getAttribute('href'), /x=100.*y=200.*z=7.*zoom=0/);

        await page.locator('#out').focus();
        await page.keyboard.press('Tab');
        const zoom = page.locator('#in');
        await focused(zoom);
        await page.keyboard.press('Enter');
        await page.waitForFunction(() => document.querySelector('.spawn-panel>ul').children.length === 2);
        assert.equal(await zoom.getAttribute('aria-disabled'), 'true');
        await focused(zoom);
        await page.keyboard.press('Tab');
        await focused(page.locator('#north'));
        await page.locator('#markers').focus();
        await page.keyboard.press('Tab');
        await focused(page.locator('#atlas'));
        await page.keyboard.press('ArrowRight');
        await page.waitForURL(url => url.searchParams.get('x') === '164');
        assert.match(await page.locator('#status').ariaSnapshot(), /X: 164/);
        await page.keyboard.press('ArrowLeft');
        await page.keyboard.press('Tab');
        const fullscreen = page.getByRole('button', { name: labels.fullscreen, exact: true });
        await focused(fullscreen);
        await page.keyboard.press('Enter');
        const exit = page.getByRole('button', { name: labels['exit-fullscreen'], exact: true });
        await exit.waitFor();
        await focused(exit);
        assert.equal(await page.evaluate(() => document.fullscreenElement.id), 'atlas-viewer');
        await page.keyboard.press('Shift+Tab');
        await focused(page.locator('#atlas'));
        await page.keyboard.press('Tab');
        await page.keyboard.press('Enter');
        await fullscreen.waitFor();
        await focused(fullscreen);
        const spawnLayer = page.getByRole('checkbox', { name: labels.spawns });
        await page.keyboard.press('Tab');
        await focused(spawnLayer);
        await page.keyboard.press('Space');
        assert.equal(await spawnLayer.isChecked(), false);
        assert.equal(await page.locator('.spawn-panel>ul').first().locator(':scope >li').count(), 0);
        await page.locator('#atlas').focus();
        await page.keyboard.press('ArrowRight');
        assert.equal(await page.locator('.spawn-panel').getByRole('status').nth(1).textContent(), '');
        await spawnLayer.focus();
        await page.keyboard.press('Space');
        assert.equal(await spawnLayer.isChecked(), true);
        await page.locator('#atlas').focus();
        await page.keyboard.press('ArrowLeft');
        await page.locator('.spawn-panel>ul').first().locator(':scope >li').first().waitFor();
      }
    }
    await page.setViewportSize({ width: 390, height: 900 });
    await page.goto(`${origin}/en/atlas?x=100&y=200&z=7&zoom=0`);
    const markerInput = page.getByRole('textbox', {name:'Marker label (1–80 characters)'});
    await markerInput.focus();
    await page.keyboard.type('Browser marker');
    await page.keyboard.press('Tab');
    const save = page.getByRole('button',{name:'Save marker at map centre'});
    await focused(save);
    await page.keyboard.press('Enter');
    await page.getByRole('link',{name:'Browser marker',exact:true}).waitFor();
    await focused(save);
    // Hold the spawns load behind the report refusal: a late load must not erase the refusal.
    let release; const held=new Promise(resolve=>{release=resolve;});
    await page.route('**/atlas/spawns?*',async route=>{await held; await route.continue();});
    await page.reload();
    await page.getByRole('link',{name:'Browser marker',exact:true}).waitFor();
    const item = page.getByRole('listitem').filter({has:page.getByRole('link',{name:'Browser marker',exact:true})});
    const remove = item.getByRole('button',{name:'Delete marker'});
    assert.match(await remove.ariaSnapshot(), /Delete marker/);
    assert.equal(await remove.getAttribute('aria-describedby'), await item.getByRole('link').getAttribute('id'));
    await remove.focus();
    await page.keyboard.press('Enter');
    await page.getByRole('link',{name:'Browser marker',exact:true}).waitFor({state:'detached'});
    assert.equal(await page.evaluate(() => {
      const active=document.activeElement;
      return active.id==='private-markers-title' || active.matches('#private-markers-title + ul button');
    }), true);
    const refused=await page.evaluate(async()=> (await fetch('/atlas/map-reports',{method:'POST',headers:{'Content-Type':'application/json'},body:'{}'})).status);
    assert.equal(refused,403);
    // The fixture has already reached its five-report limit: the UI must surface refusal.
    await page.getByLabel('Describe the error (10–2000 characters)').fill('The map floor is incorrect.');
    await page.getByRole('button',{name:'Send report'}).click();
    const refusal=page.getByRole('status').filter({hasText:'Unavailable or invalid input. Please try again.'});
    await refusal.waitFor();
    release(); // the route stays: a later spawns load passes straight through it
    await page.waitForFunction(()=>document.querySelector('.spawn-panel>p')?.textContent.includes(': 2;'));
    await refusal.waitFor();
    await page.route('**/atlas/spawns?*', async route => {
      const response = await route.fetch();
      const points = await response.json();
      await route.fulfill({response, json: points.map(point => ({...point, respawn_ms: null}))});
    });
    for (const [lang, label] of [['en', 'No ordinary respawn'], ['pl', 'Brak zwykłego odrodzenia']]) {
      await page.goto(`${origin}/${lang}/atlas?x=100&y=200&z=7&zoom=0`);
      const details = page.locator('.spawn-panel>ul details').first();
      await details.locator('summary').click();
      assert.ok((await details.locator('p').first().textContent()).includes(label),
        `fixed-at-reset spawn must show "${label}"`);
    }
    assert.deepEqual(errors,[]);
    console.log('Browser: 390/1440 px, PL/EN, grouping, viewport counts, place link, keyboard, private marker persistence/deletion, CSRF and report refusal passed');
  } finally { await browser.close(); }
})().catch(error=>{console.error(error);process.exitCode=1;});
