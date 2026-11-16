import {test,expect} from '../fixtures.mjs';
import {register,login,submit,pool,csrfRejected} from '../helpers.mjs';
import path from 'node:path';

test('staff premium grant persists entitlement and audit; player cannot grant',async({page,browser},info)=>{
 const lang=info.project.metadata.locale;
 const account=await register(page,lang);
 await login(page,lang,account);
 const db=pool();
 const staff=await browser.newContext({
  baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443',
  ignoreHTTPSErrors:true,
  storageState:path.join(process.env.PORTAL_E2E_AUTH_DIRECTORY,`staff-${info.project.name}.json`),
 });
 try {
  const target=(await db.query('SELECT id FROM accounts WHERE email=$1',[account.email])).rows[0].id;
  const admin=await staff.newPage();
  expect((await admin.goto('/admin/commerce')).status(),'staff commerce page').toBe(200);
  // Selector and fields: crates/admin/src/commerce/page.html; route: commerce/http.rs.
  const selector='form[action="/admin/commerce/grant"]';
  const form=admin.locator(selector);
  const request=await form.locator('[name=request_id]').inputValue();
  await submit(admin,selector,{target,days:'7',reason:'Independent browser premium entitlement qualification'});
  await expect(admin,'successful grant returns to commerce').toHaveURL(/\/admin\/commerce$/);
  const rows=await db.query('SELECT event_type,duration_days,delta_seconds,effective_until > now() AS active FROM premium_ledger WHERE request_id=$1 AND account_id=$2',[request,target]);
  expect(rows.rows,'grant must persist seven days of premium').toEqual([{event_type:'GRANT',duration_days:7,delta_seconds:'604800',active:true}]);
  expect((await db.query('SELECT event_name FROM premium_audit WHERE request_id=$1 AND account_id=$2',[request,target])).rows,'grant has an audit event').toEqual([{event_name:'products.premium_granted'}]);
  // No Game entitlement receipt is seeded: this proves the portal ledger grant only.
  await page.goto(`/${lang}/account`);
  const csrf=await page.locator('input[name="_csrf"]').first().inputValue();
  await csrfRejected(page,'/admin/commerce/grant',{target,days:'7',request_id:request,reason:'Unauthorized player grant attempt'});
  const denied=await page.request.post('/admin/commerce/grant',{form:{_csrf:csrf,target,days:'7',request_id:request,reason:'Unauthorized player grant attempt'},maxRedirects:0});
  expect(denied.status(),'authenticated player lacks staff permission').toBe(403);
  expect((await db.query('SELECT count(*)::int n FROM premium_ledger WHERE account_id=$1',[target])).rows[0].n,'refusal leaves exactly the original grant').toBe(1);
 } finally {await staff.close();await db.end();}
});
