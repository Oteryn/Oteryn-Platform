import {test,expect} from '@playwright/test';
import {register,login,submit,csrfRejected,pool} from '../helpers.mjs';
import * as OTPAuth from 'otpauth';

test('staff bootstrap, mandatory MFA, admin navigation and non-staff refusal',async({page,browser},info)=>{
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
 await submit(page,'main form',{password:staff.password});
 const secret=await page.locator('main .op-prose').innerText();
 const totp=new OTPAuth.TOTP({secret,algorithm:'SHA1',digits:6,period:30});
 await submit(page,'main form',{password:staff.password,code:totp.generate()});
 const codes=await page.locator('textarea[readonly]').allTextContents();
 expect(codes.length).toBeGreaterThan(0);
 await login(page,lang,staff);
 await expect(page).toHaveURL(/mfa\/challenge/);
 await submit(page,'main form',{code:codes[0]});
 const response=await page.goto(`/admin?lang=${lang}`);
 expect(response.status()).toBe(200);
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
   const forms=await page.locator('form[method=post],form[method=POST]').evaluateAll(forms=>forms.map(f=>({action:f.getAttribute('action')||location.pathname,fields:Object.fromEntries(new FormData(f))})));
   for(const form of forms)await csrfRejected(page,form.action,form.fields);
   } finally {await page.close();}
 });
 const context=await browser.newContext({ignoreHTTPSErrors:true,viewport:info.project.use.viewport,locale:info.project.metadata.locale,baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443'});
 const ordinary=await context.newPage();const account=await register(ordinary,lang);await login(ordinary,lang,account);
 expect((await ordinary.goto('/admin')).status()).toBe(403);
 await context.close();
 await info.attach('admin-findings',{body:JSON.stringify({paths,failures},null,2),contentType:'application/json'});
 expect(failures).toEqual([]);
});
