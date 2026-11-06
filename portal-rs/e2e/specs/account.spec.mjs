import {test,expect} from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import {register,login,csrfRejected,submit,pool} from '../helpers.mjs';

function previousWeek(){const d=new Date();d.setUTCHours(0,0,0,0);d.setUTCDate(d.getUTCDate()-(d.getUTCDay()+6)%7-7);return Math.floor(d.getTime()/1000);}

const surfaces=[
 ['/account','overview'],['account/privacy','privacy'],['account/sessions','sessions'],['account/passkeys','passkeys'],
 ['account/security/email','email change'],['password/change','password change'],['account/sudo','sudo'],['mfa','MFA settings'],
 ['account/characters/new','character creation'],['shop/history','wallet balance'],['panel/premium','premium'],['panel/payments','payments'],
 ['shop','store'],['shop/coins','top up'],['shop/history','wallet history'],['shop/transfer','transfer'],
 ['bazaar','bazaar'],['account/bazaar','character listings'],['bazaar/items','item listings'],
 ['account/hunts','hunts'],[`account/journal/reports?week=${previousWeek()}`,'journal reports'],['account/game-logins','game login history'],['account/dashboard','dashboard'],
 ['panel/hirelings','hirelings'],['support/tickets','tickets'],['support/reports','player reports'],
 ['party','party finder'],['puzzles','puzzles'],['collection','collection'],['pass','season pass'],['account/notifications','notification preferences'],
 ['account/privacy/export','data export'],['account/privacy/delete','account deletion'],
];
// These startup adapters explicitly deny access until accepted game authority exists.
const gameRefusals={
 'account/dashboard':'closed-session and timer producers are unavailable; runtime defaults to empty 503',
 [`account/journal/reports?week=${previousWeek()}`]:'accepted private journal projection source is unavailable',
 'bazaar/items':'capability and escrow receipt integration is absent',
 'account/game-logins':'accepted ownership and baseline authority is absent',
 'pass':'accepted season source and cosmetic authority is absent',
};
let account, cookies, stateFile;
test.beforeAll(async({browser},info)=>{
 stateFile=path.join(process.env.PORTAL_E2E_AUTH_DIRECTORY,`${info.project.name}.json`);
 if(fs.existsSync(stateFile)){({account,cookies}=JSON.parse(fs.readFileSync(stateFile,'utf8')));return;}
 const context=await browser.newContext({ignoreHTTPSErrors:true,viewport:info.project.use.viewport,locale:info.project.metadata.locale,baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443'});
 const page=await context.newPage();account=await register(page,info.project.metadata.locale);await login(page,info.project.metadata.locale,account);cookies=await context.cookies();fs.writeFileSync(stateFile,JSON.stringify({account,cookies}),{mode:0o600});await context.close();
});
test.beforeEach(async({context})=>{await context.addCookies(cookies);});
test.afterEach(async({context})=>{cookies=await context.cookies();fs.writeFileSync(stateFile,JSON.stringify({account,cookies}),{mode:0o600});});
for(const [path,label] of surfaces) test(gameRefusals[path]?`${label}: documented game-authority refusal`:`${label}: owner page and CSRF on every rendered mutation`,async({page},info)=>{
 const lang=info.project.metadata.locale;

 const target=path.startsWith('/')?path:`/${lang}/${path}`;
 if(gameRefusals[path]) {
   const refused=await page.request.get(target);
   expect(refused.status(),gameRefusals[path]).toBe(503);
   expect(refused.headers()['cache-control']).toContain('no-store');
   expect(await refused.text()).not.toContain(account.email);
   return;
 }
 if(path.endsWith('/export')) {
   await page.goto(`/${lang}/account/sudo`);
   await submit(page,'main form',{current_password:account.password});
   const exported=await page.request.get(target);
   expect(exported.status()).toBe(200);
   expect(exported.headers()['content-type']).toContain('json');
   expect(await exported.text()).toContain(account.email);
   return;
 }
 const response=await page.goto(target);
 expect(response.status(),target).toBe(200);
 await expect(page.locator('html')).toHaveAttribute('lang',lang);
 await expect(page.locator('main')).toBeVisible();
 if(label==='wallet balance') {expect(await page.locator('main dl dd').count()).toBeGreaterThanOrEqual(3);await expect(page.locator('main dl dd').first()).toHaveText('0');}
 const forms=await page.locator('form[method=post],form[method=POST]').evaluateAll(forms=>forms.map(f=>({action:f.getAttribute('action')||location.pathname,fields:Object.fromEntries(new FormData(f))})));
 for(const form of forms) await csrfRejected(page,form.action,form.fields);
});

test('support ticket create, reply and refusal to another account',async({page,browser},info)=>{
 const lang=info.project.metadata.locale;

 await page.goto(`/${lang}/support/tickets/create`);
 await submit(page,'main form',{subject:'Browser qualification',body:'Local browser support ticket body for qualification.'});
 await expect(page).toHaveURL(/support\/tickets\/[a-f0-9-]+$/);
 const target=new URL(page.url()).pathname;
 await submit(page,'main form[action$="/reply"]',{body:'Local browser reply for qualification.'});
 await expect(page.locator('main')).toContainText('Local browser reply for qualification.');
 const context=await browser.newContext({ignoreHTTPSErrors:true,viewport:info.project.use.viewport,locale:info.project.metadata.locale,baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443'});
 const other=await context.newPage();const foreign=await register(other,lang);await login(other,lang,foreign);
 expect((await other.goto(target)).status()).toBe(404);
 await context.close();
});

test('character request reaches pending state without a game result',async({page},info)=>{
 const lang=info.project.metadata.locale;
 const db=pool();let world;
 try{world=(await db.query("INSERT INTO world_status(id,name) VALUES(uuidv7(),'Browser Test World') RETURNING id")).rows[0].id;}finally{await db.end();}
 await page.goto(`/${lang}/account/characters/new`);
 const form=page.locator('main form[method=post]').first();await form.locator('select[name=world]').selectOption(world);
 await submit(page,'main form[method=post]',{name:'Browser Tester'});
 await expect(page).toHaveURL(/characters\/requests\/[a-f0-9-]+/);
 await expect(page.locator('[role=status]')).toHaveText(lang==='pl'
   ?'Trwa przetwarzanie prośby. Bez wyniku z gry rezultat pozostaje nieznany. Odśwież stronę, aby sprawdzić ponownie.'
   :'Your request is being processed. Without a result from Game, its outcome is still unknown. Refresh this page to check again.');
 const statusDb=pool();
 try{
   const operation=new URL(page.url()).pathname.split('/').at(-1);
   const row=(await statusDb.query('SELECT c.state,c.receipt_json,a.email FROM character_commands c JOIN accounts a ON a.id=c.account_id WHERE c.id=$1',[operation])).rows[0];
   expect(row.state).toBe('PENDING');expect(row.receipt_json).toBeNull();expect(row.email).toBe(account.email);
 }finally{await statusDb.end();}
});

test('player report: create a private pending report',async({page},info)=>{
 const lang=info.project.metadata.locale;
 await page.goto(`/${lang}/support/reports/create`);
 await submit(page,'main form',{target_reference:'Local browser qualification target',evidence_summary:'Synthetic local evidence; no real player involved.'});
 await expect(page).toHaveURL(/support\/reports\/[a-f0-9-]+$/);
 await expect(page.locator('main')).toContainText('Local browser qualification target');
 const db=pool();
 try {
  const row=(await db.query('SELECT status FROM player_reports WHERE target_reference=$1 ORDER BY created_at DESC LIMIT 1',['Local browser qualification target'])).rows[0];
  expect(row.status).toBe('submitted');
 } finally {await db.end();}
});

test('party post: unavailable ownership authority refuses the mutation',async({page},info)=>{
 const lang=info.project.metadata.locale;
 expect((await page.goto(`/${lang}/party`)).status()).toBe(200);
 await page.goto(`/${lang}/account/sudo`);
 const token=await page.locator('input[name=_csrf]').first().inputValue();
 const now=Math.floor(Date.now()/1000);
 const form={character:'019a541b-a000-7000-8000-000000000001',purpose:'hunt',min_level:'1',max_level:'100',profession:'any',starts_at:String(now+60),expires_at:String(now+3600),note:'Synthetic local recruitment'};
 await csrfRejected(page,`/${lang}/party`,form);
 const db=pool();
 try {
  const count=async()=>Number((await db.query('SELECT count(*) AS count FROM pd_party_posts')).rows[0].count);
  const before=await count();
  const refused=await page.request.post(`/${lang}/party`,{form:{...form,_csrf:token},headers:{origin:new URL(page.url()).origin},maxRedirects:0});
  expect(refused.status()).toBe(403);expect(await count()).toBe(before);
 } finally {await db.end();}
});
