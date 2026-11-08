import {test,expect} from '../fixtures.mjs';
import fs from 'node:fs';
import path from 'node:path';
import {register,login} from '../helpers.mjs';
import {assertShell} from '../shell.mjs';
const rows=JSON.parse(fs.readFileSync(new URL('../route-expectations.json',import.meta.url),'utf8'));
export const sample=(route,lang)=>route.replaceAll('{lang}',lang).replaceAll('{kind}',route.includes('stats/rankings')?'yesterday':route.includes('rankings')?'experience':'items').replaceAll('{tab}','experience').replaceAll('{slug}',route.startsWith('/admin/pages/')?'rules':'missing-e2e').replaceAll('{action}','approve').replace(/\{(?:floor|zoom|x|y)\}/g,'0').replace(/\{(?:name|slug|key|family|channel|filename|\*target|\*rest)\}/g,'missing-e2e').replace(/\{[^}]+\}/g,'019a541b-a000-7000-8000-000000000001');
let cookies;
test.beforeAll(async({browser},info)=>{
 const context=await browser.newContext({ignoreHTTPSErrors:true,baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443',locale:info.project.metadata.locale,viewport:info.project.use.viewport});
 const page=await context.newPage();const account=await register(page,info.project.metadata.locale);await login(page,info.project.metadata.locale,account);cookies=await context.cookies();await context.close();
});
for(const row of rows)test(`route ${row.crate} ${row.route}`,async({page,context},info)=>{
 const lang=info.project.metadata.locale;
 if(row.actor==='staff') {
   const staff=JSON.parse(fs.readFileSync(path.join(process.env.PORTAL_E2E_AUTH_DIRECTORY,`staff-${info.project.name}.json`),'utf8'));
   await context.addCookies(staff.cookies);
 }else await context.addCookies(cookies);
 const target=sample(row.route,lang);
 const errors=[];
 let documentPath=new URL(target,'https://localhost:18443').pathname;
 page.on('pageerror',e=>errors.push(e.message));
 page.on('console',m=>{
   if(m.type()!=='error')return;
   const expected=documentPath;
   if(new URL(m.location().url||'https://localhost/').pathname===expected&&m.text()===`Failed to load resource: the server responded with a status of ${row.status} (${({400:'Bad Request',401:'Unauthorized',403:'Forbidden',404:'Not Found',422:'Unprocessable Entity',503:'Service Unavailable'})[row.status]})`)return;
   errors.push(m.text());
 });
 const response=await page.request.fetch(target,{method:row.method,headers:{accept:'text/html',origin:'https://localhost:18443','accept-language':lang},...(row.method==='GET'?{}:{form:row.route==='/oauth/token'?{client_id:'e2e-local'}:{}})});
 expect(response.status(),`${target}: ${row.reason}`).toBe(row.status);
 if(row.status===503)expect(response.headers()['cache-control']).toContain('no-store');
 documentPath=new URL(response.url()).pathname;
 const text=await response.text();
 if(row.method==='GET'&&response.headers()['content-type']?.startsWith('text/html')&&/<html[\s>]/i.test(text)) {
   expect((await page.goto(target)).status()).toBe(row.status);
   expect(new URL(page.url()).pathname).toBe(documentPath);
   await assertShell(page,{status:row.status});
 }
 if(row.route==='/oauth/token') {
   const anonymousClient=await page.request.post(target,{form:{}});
   expect(anonymousClient.status()).toBe(401);
   expect((await anonymousClient.json()).error).toBe('invalid_client');
 }
 if(row.route==='/{lang}/account/journal/reports/email') {
   expect((await page.goto(`/${lang}/account/sudo`)).status()).toBe(200);
   const token=await page.locator('input[name=_csrf]').first().inputValue();
   const unavailable=await page.request.post(target,{form:{_csrf:token,enabled:'true'},headers:{origin:'https://localhost:18443'},maxRedirects:0});
   expect(unavailable.status(),'weekly mail stays closed without an authoritative JournalSource').toBe(503);
   expect(unavailable.headers()['cache-control']).toContain('no-store');
 }
 expect(errors).toEqual([]);
 await info.attach('route-coverage',{body:JSON.stringify({route:row.route,crate:row.crate,actor:row.actor,method:row.method,status:response.status(),reason:row.reason}),contentType:'application/json'});
});
