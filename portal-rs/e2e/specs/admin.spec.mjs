import {test,expect} from '../fixtures.mjs';
import path from 'node:path';
import fs from 'node:fs';
import {register,login,submit,csrfRejected,pool} from '../helpers.mjs';
import {assertShell} from '../shell.mjs';
import * as OTPAuth from 'otpauth';

test('staff bootstrap, mandatory MFA, admin navigation and non-staff refusal',async({page,browser},info)=>{
 test.setTimeout(300_000);
 const lang=info.project.metadata.locale;
 const staff=await register(page,lang);
 const db=pool();
 try{
   // Same first-staff SQL bootstrap used by the owning admin crate's tests.
   await db.query("INSERT INTO account_roles(account_id,role_id) SELECT a.id,r.id FROM accounts a CROSS JOIN roles r WHERE a.email=$1 AND r.key='platform_admin'",[staff.email]);
 }finally{await db.end();}
 await login(page,lang,staff);
 expect((await page.goto(`/admin?lang=${lang}`)).status()).toBe(403);
 await page.goto(`/${lang}/mfa`);
 await submit(page,'main form[action$="/mfa/enroll"]',{password:staff.password});
 const secret=await page.locator('main .op-prose').innerText();
 const totp=new OTPAuth.TOTP({secret,algorithm:'SHA1',digits:6,period:30});
 await submit(page,'main form[action$="/mfa/confirm"]',{password:staff.password,code:totp.generate()});
 const codes=await page.locator('textarea[readonly]').allTextContents();
 expect(codes.length).toBeGreaterThan(0);
 await login(page,lang,staff);
 await expect(page).toHaveURL(/mfa\/challenge/);
 await submit(page,'main form[action$="/mfa/challenge"]',{code:codes[0]});
 const response=await page.goto(`/admin?lang=${lang}`);
 expect(response.status()).toBe(200);
 fs.writeFileSync(path.join(process.env.PORTAL_E2E_AUTH_DIRECTORY,`staff-${info.project.name}.json`),JSON.stringify(await page.context().storageState()),{mode:0o600});
 const linked=await page.locator('nav a[href]').evaluateAll(nodes=>nodes.map(n=>n.getAttribute('href')).filter(h=>/^\/(?:pl\/|en\/)?admin(?:\/|$)|^\/console(?:\/|$)/.test(h)));
 const paths=[...new Set([...linked,'/admin/players','/admin/roles','/admin/console/audit','/admin/news','/admin/pages','/admin/events','/admin/media','/admin/wiki','/admin/world','/admin/commerce','/admin/support/tickets',`/${lang}/admin/support/tickets`,'/admin/moderation/reports','/admin/moderation/enforcement',`/admin/${lang}/bazaar`,'/admin/control/step-up','/console/dashboard','/admin/console/access','/admin/console/players','/admin/console/approvals'])];
 const failures=[];
 const staffContext=page.context();
 for(const path of paths) await test.step(path,async()=>{
   const page=await staffContext.newPage();
   try {
   let response;try{response=await page.goto(['/admin/world','/admin/support/tickets','/admin/moderation/reports','/admin/moderation/enforcement','/admin/control/step-up'].includes(path)||path.startsWith(`/${lang}/admin/`)?path:`${path}?lang=${lang}`);}catch(error){failures.push(`${path}: ${error.message}`);return;}
   if(response.status()!==200)failures.push(`${path}: HTTP ${response.status()}`);
   if(response.status()===200 && await page.locator('html').getAttribute('lang')!==lang)failures.push(`${path}: wrong page language`);
   if(response.status()===200) await assertShell(page);
   const forms=await page.locator('form[method=post],form[method=POST]').evaluateAll(forms=>forms.map(f=>({action:f.getAttribute('action')||location.pathname,fields:Object.fromEntries(new FormData(f))})));
   for(const form of forms)await csrfRejected(page,form.action,form.fields);
   } finally {await page.close();}
 });
 // Opening navigation must not grant Game read permission to the bootstrap role.
 expect((await page.request.get('/admin/console/players?q=Local')).status()).toBe(403);
 expect((await page.request.get('/console/dashboard?agent=unavailable')).status()).toBe(403);
 const context=await browser.newContext({ignoreHTTPSErrors:true,viewport:info.project.use.viewport,locale:info.project.metadata.locale,baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443'});
 const ordinary=await context.newPage();const account=await register(ordinary,lang);await login(ordinary,lang,account);
 await test.step('four-eyes boundary: an MFA staff member cannot grant a privileged role alone',async()=>{
   const db=pool();
   try {
     const target=(await db.query('SELECT id FROM accounts WHERE email=$1',[account.email])).rows[0].id;
     const role=(await db.query("SELECT id FROM roles WHERE key='platform_admin'")).rows[0].id;
     await page.goto('/admin/roles');
     const token=await page.locator('input[name=_csrf]').first().inputValue();
     const fields={account:target,role,action:'grant'};
     await csrfRejected(page,'/admin/roles',fields);
     const refused=await page.request.post('/admin/roles',{form:{...fields,_csrf:token},headers:{origin:new URL(page.url()).origin},maxRedirects:0});
     expect(refused.status()).toBe(403);
     expect(Number((await db.query('SELECT count(*) AS n FROM account_roles WHERE account_id=$1 AND role_id=$2',[target,role])).rows[0].n)).toBe(0);
   }finally{await db.end();}
 });
 expect((await ordinary.goto('/admin')).status()).toBe(403);
 for(const path of ['/admin/support/tickets','/admin/moderation/reports','/admin/moderation/enforcement','/admin/control/step-up','/console/dashboard','/admin/console/players']) expect((await ordinary.goto(path)).status(),path).toBe(403);
 await context.close();
 await info.attach('admin-findings',{body:JSON.stringify({paths,failures},null,2),contentType:'application/json'});
 expect(failures).toEqual([]);
});
