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
const unavailable=new Set(['online','rankings','guilds','houses','deaths','wiki','atlas','delivery','guild-stats','growth-rankings','reports','logins','pass','places']);
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
 const members=inventory.sections.filter(s=>s.group===section.group);
 const expected=members.map(s=>s.path.startsWith('/')?s.path:`/${lang}/${s.path}`);
 const group=page.locator('.hx-nav__item--menu').filter({has:page.locator(`.hx-nav__menu a[href="${path}"]`)});
 await expect(group).toHaveCount(1);
 expect(await group.locator('.hx-nav__menu a').evaluateAll(nodes=>nodes.map(n=>n.getAttribute('href')))).toEqual(expected);
 const catalogue=fs.readFileSync(new URL(`../../crates/ui/i18n/${lang}.ftl`,import.meta.url),'utf8');
 const labels=members.map(s=>catalogue.split('\n').find(line=>line.startsWith(`${s.label} = `))?.slice(s.label.length+3));
 expect(await group.locator('.hx-nav__menu a').allTextContents()).toEqual(labels);
 expect(new Set(labels).size,'distinct labels in the group').toBe(labels.length);
 await expect(group.locator(`.hx-nav__menu a[href="${path}"]`)).toHaveAttribute('aria-current','page');
 await expect(page.locator('.hx-nav__menu a[aria-current=page]')).toHaveCount(1);
 expect(errors).toEqual([]);
});

test('every enabled group opens by keyboard focus and touch',async({page,browser},info)=>{
 const lang=info.project.metadata.locale;
 await page.goto(`/${lang}/`);
 for(const group of await page.locator('.hx-nav__item--menu').all()) {
   const summary=group.locator('summary');
   await summary.focus();
   await expect(group.locator('.hx-nav__menu')).toBeVisible();
   await page.keyboard.press('Enter');
   await expect(group.locator('details')).toHaveAttribute('open','');
   await page.keyboard.press('Tab');
   await expect(group.locator('.hx-nav__menu a').first()).toBeFocused();
 }
 const touch=await browser.newContext({ignoreHTTPSErrors:true,hasTouch:true,viewport:info.project.use.viewport,locale:lang,baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443'});
 await touch.addCookies(cookies);
 const screen=await touch.newPage();await screen.goto(`/${lang}/`);
 for(const group of await screen.locator('.hx-nav__item--menu').all()) {
   await group.locator('summary').tap();
   await expect(group.locator('details')).toHaveAttribute('open','');
   const link=group.locator('.hx-nav__menu a').last();
   const destination=await link.getAttribute('href');
   await link.tap();
   await expect(screen).toHaveURL(new URL(destination,screen.url()).href);
   await screen.goto(`/${lang}/`);
 }
 await touch.close();
});
