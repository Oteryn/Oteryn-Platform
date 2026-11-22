import {test,expect} from '../fixtures.mjs';
import {register,login,submit,pool,csrfRejected} from '../helpers.mjs';
import path from 'node:path';
import {enrollMfa,loginWithRecoveryCode} from '../staff-rbac-helpers.mjs';

test('staff premium grant persists entitlement and audit; player cannot grant',async({page,browser},info)=>{
 test.setTimeout(300_000);
 const lang=info.project.metadata.locale;
 const account=await register(page,lang);
 await login(page,lang,account);
 const db=pool();
 const staff=await browser.newContext({
  baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443',
  ignoreHTTPSErrors:true,
  storageState:path.join(process.env.PORTAL_E2E_AUTH_DIRECTORY,`staff-${info.project.name}.json`),
 });
 const reviewer=await browser.newContext({
  baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443',
  ignoreHTTPSErrors:true,
  locale:info.project.metadata.locale,
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
  expect((await db.query('SELECT count(*)::int n FROM premium_ledger WHERE request_id=$1',[request])).rows[0].n,'a staff grant waits for a second staff member').toBe(0);
  // Second staff identity: same first-staff SQL bootstrap and MFA path as admin.spec.mjs.
  const second=await reviewer.newPage();
  const approver=await register(second,lang);
  await db.query("INSERT INTO account_roles(account_id,role_id) SELECT a.id,r.id FROM accounts a CROSS JOIN roles r WHERE a.email=$1 AND r.key='platform_admin'",[approver.email]);
  await login(second,lang,approver);
  const codes=await enrollMfa(second,lang,approver);
  // MFA confirmation revokes existing sessions; authenticate again with a recovery code.
  await loginWithRecoveryCode(second,lang,approver,codes[0]);
  await admin.goto('/admin/commerce');
  await expect(admin.locator(`form[action="/admin/commerce/approvals/${request}"] button`),'the proposer cannot approve their own grant').toHaveCount(0);
  expect((await second.goto('/admin/commerce')).status(),'second staff commerce page').toBe(200);
  await submit(second,`form[action="/admin/commerce/approvals/${request}"]`);
  await expect(second,'approval returns to commerce').toHaveURL(/\/admin\/commerce$/);
  const rows=await db.query('SELECT event_type,duration_days,delta_seconds,effective_until > now() AS active FROM premium_ledger WHERE request_id=$1 AND account_id=$2',[request,target]);
  expect(rows.rows,'grant must persist seven days of premium').toEqual([{event_type:'GRANT',duration_days:7,delta_seconds:'604800',active:true}]);
  expect((await db.query('SELECT event_name FROM premium_audit WHERE request_id=$1 AND account_id=$2',[request,target])).rows,'grant has an audit event').toEqual([{event_name:'products.premium_granted'}]);
  await page.goto(`/${lang}/panel/premium`);
  const end=(await db.query('SELECT effective_until FROM premium_ledger WHERE request_id=$1',[request])).rows[0].effective_until;
  await expect(page.locator('main'),'premium panel shows persisted end date').toContainText(end.toISOString().slice(0,10));
  // No Game entitlement receipt is seeded: this proves the portal ledger grant only.
  await page.goto(`/${lang}/account`);
  const csrf=await page.locator('input[name="_csrf"]').first().inputValue();
  await csrfRejected(page,'/admin/commerce/grant',{target,days:'7',request_id:request,reason:'Unauthorized player grant attempt'});
  const denied=await page.request.post('/admin/commerce/grant',{form:{_csrf:csrf,target,days:'7',request_id:request,reason:'Unauthorized player grant attempt'},maxRedirects:0});
  expect(denied.status(),'authenticated player lacks staff permission').toBe(403);
  expect((await db.query('SELECT count(*)::int n FROM premium_ledger WHERE account_id=$1',[target])).rows[0].n,'refusal leaves exactly the original grant').toBe(1);
 } finally {await reviewer.close();await staff.close();await db.end();}
});
