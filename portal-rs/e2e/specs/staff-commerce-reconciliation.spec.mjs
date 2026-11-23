import {test,expect} from '../fixtures.mjs';
import {register,login,submit,pool} from '../helpers.mjs';
import {paidTopup} from '../staff-commerce-helpers.mjs';
import {isolatedAdmin} from '../staff-rbac-helpers.mjs';

test('staff reconciles a paid test intent and records the audit; bad intent refused',async({page,browser},info)=>{
 const lang=info.project.metadata.locale;
 const account=await register(page,lang);await login(page,lang,account);
 const {context:staff,page:admin}=await isolatedAdmin(browser,info);
 const db=pool();
 try {
  const paid=await paidTopup(page,lang,db);
  expect((await admin.goto('/admin/commerce')).status()).toBe(200);
  // Commerce has one target-ID form rather than a payment-intent detail page.
  const selector='form[action="/admin/commerce/reconcile"]';
  const request=await admin.locator(`${selector} [name=request_id]`).inputValue();
  const reconciliationResponse=admin.waitForResponse(r=>r.request().method()==='POST'&&new URL(r.url()).pathname==='/admin/commerce/reconcile'&&r.request().postData()?.includes('_csrf='));
  await submit(admin,selector,{target:paid.payment_intent_id,reason:'Review browser test bank payment reconciliation'});
  const reconciled=await reconciliationResponse;
  expect(reconciled.status(),`reconciliation response: ${reconciled.status()===303?'redirect':await reconciled.text()}`).toBe(303);
  await expect(admin,'reconciliation returns to commerce').toHaveURL(/\/admin\/commerce$/);
  expect((await db.query('SELECT after FROM audit_log WHERE request_id=$1 AND target=$2',[request,paid.payment_intent_id])).rows.length,'reconciliation audit persisted').toBeGreaterThan(0);
  const operation=(await db.query('SELECT result FROM commerce_operations WHERE id=$1',[request])).rows[0];
  expect(operation.result,'supported reconciliation commits resolution').toMatchObject({resolution_code:'reviewed_no_payment_state_change'});
  const refusal=admin.waitForNavigation();
  await submit(admin,selector,{target:'00000000-0000-7000-8000-000000000001',reason:'Reject nonexistent payment intent reconciliation'});
  expect((await refusal).status(),'unknown intent is invalid').toBe(422);
  expect((await db.query('SELECT count(*)::int n FROM commerce_operations WHERE target_id=$1 AND result IS NOT NULL',['00000000-0000-7000-8000-000000000001'])).rows[0].n).toBe(0);
 } finally {await staff.close();await db.end();}
});
