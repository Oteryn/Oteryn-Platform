import {test,expect} from '../fixtures.mjs';
import fs from 'node:fs';
import {register,login} from '../helpers.mjs';
import {assertShell} from '../shell.mjs';
const inventory=JSON.parse(fs.readFileSync(new URL('../inventory.json',import.meta.url),'utf8'));
let cookies;
test.beforeAll(async({browser},info)=>{
 const context=await browser.newContext({ignoreHTTPSErrors:true,baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443',locale:info.project.metadata.locale,viewport:info.project.use.viewport});
 const page=await context.newPage();const account=await register(page,info.project.metadata.locale);await login(page,info.project.metadata.locale,account);cookies=await context.cookies();await context.close();
});
test.beforeEach(async({context})=>context.addCookies(cookies));
const unavailable=new Set(['online','rankings','guilds','houses','deaths','wiki','creatures','atlas','delivery','guild-stats','growth-rankings','reports','logins','pass','places']);
for(const section of inventory.sections) test(`registry ${section.section}: destination and shared shell`,async({page},info)=>{
 const lang=info.project.metadata.locale;
 const path=section.path.startsWith('/')?section.path:`/${lang}/${section.path}`;
 const errors=[];
 page.on('pageerror',e=>errors.push(e.message));
 page.on('console',m=>{if(m.type()==='error'&&!/^Failed to load resource: the server responded with a status of 503 \(Service Unavailable\)$/.test(m.text()))errors.push(m.text());});
 const response=await page.goto(path);
 expect(response.status(),path).toBe(unavailable.has(section.section)?503:200);
 if(response.status()===503)expect(response.headers()['cache-control']).toContain('no-store');
 await assertShell(page);
 const currentLinks = page.locator(`header a[href="${path}"], #podglad-konta a[href="${path}"], footer.foot a[href="${path}"]`);
 expect(await currentLinks.count(), `published destination ${path}`).toBeGreaterThan(0);
 const labels = await currentLinks.allTextContents();
 expect(labels.every(text => text.trim().length > 0), 'each route has a readable label').toBe(true);
 const states = await currentLinks.evaluateAll(links => links.map(link => link.getAttribute('aria-current')));
 expect(states, 'an active link identifies the current route').toContain('page');
 expect(errors).toEqual([]);
});

test('module navigation opens every enabled group by keyboard focus and touch',async({page,browser},info)=>{
 const lang = info.project.metadata.locale;
 const url = `/${lang}/news`;
 await page.goto(url);
 const menu = page.locator('#portal-menu-toggle');
 if (await menu.isVisible()) await menu.click();
 const groups = page.locator('#portal-header [data-nav-section]');
 expect(await groups.count()).toBeGreaterThan(0);
 for (const group of await groups.all()) {
   const panel = page.locator('#' + await group.getAttribute('aria-controls'));
   await group.focus();
   await page.keyboard.press('Enter');
   await expect(panel).toBeVisible();
   const bounds = await panel.boundingBox();
   expect(bounds.x).toBeGreaterThanOrEqual(0);
   expect(bounds.x + bounds.width).toBeLessThanOrEqual(info.project.use.viewport.width);
   await page.keyboard.press('Tab');
   await expect(panel.locator('a').first()).toBeFocused();
   await page.keyboard.press('Escape');
   await expect(group).toBeFocused();
   await expect(panel).toBeHidden();
 }
 const context = await browser.newContext({ignoreHTTPSErrors:true,hasTouch:true,viewport:info.project.use.viewport,locale:lang,baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443'});
 try {
   await context.addCookies(cookies);
   const screen = await context.newPage();
   const buttons = await groups.evaluateAll(nodes => nodes.map(node => node.id));
   for (const id of buttons) {
     await screen.goto(url);
     const toggle = screen.locator('#portal-menu-toggle');
     if (await toggle.isVisible()) await toggle.tap();
     const group = screen.locator('#' + id);
     await group.tap();
     const panel = screen.locator('#' + await group.getAttribute('aria-controls'));
     await expect(panel).toBeVisible();
     const link = panel.locator('a').last();
     const destination = await link.getAttribute('href');
     await link.tap();
     await expect(screen).toHaveURL(new URL(destination,screen.url()).href);
   }
 } finally {
   await context.close();
 }
});

test('module navigation remains usable when the world route disallows scripts', async ({page}, info) => {
 const lang = info.project.metadata.locale;
 const response = await page.goto(`/${lang}/worlds`);
 expect(response.headers()['content-security-policy']).toContain("default-src 'none'");
 await expect(page.locator('script[src$="/thais/home.js"]')).toHaveCount(0);
 const menu = page.locator('details.om-fallback-menu');
 const summary = menu.locator(':scope > summary');
 await expect(summary).toBeVisible();
 await expect(menu.locator(':scope > #portal-navigation')).toBeHidden();
 await summary.focus();
 await page.keyboard.press('Enter');
 await expect(menu.locator(':scope > #portal-navigation')).toBeVisible();
 const wiki = menu.locator(`a[href="/${lang}/wiki"]`);
 await expect(wiki).toBeVisible();
 await wiki.click();
 await expect(page).toHaveURL(new URL(`/${lang}/wiki`, page.url()).href);
});
