import {test,expect} from '../fixtures.mjs';
import {register,login,pool} from '../helpers.mjs';
import {isolatedSeededAdmin} from '../staff-rbac-helpers.mjs';
import {paidTopup} from '../staff-commerce-helpers.mjs';

test('staff reconciles a paid test intent and records the audit; bad intent refused',async({page,browser},info)=>{
 const lang=info.project.metadata.locale;
 const account=await register(page,lang);await login(page,lang,account);
 const {context:staff,page:admin}=await isolatedSeededAdmin(browser,info);
 const db=pool();
 try {
  const paid=await paidTopup(page,lang,db);
  expect((await admin.goto('/admin/commerce')).status()).toBe(200);
  const path='/admin/commerce/reconcile';
  await expect(admin.locator(`a[href^="${path}?"]`)).toBeVisible();
  expect((await admin.goto(`${path}?lang=${lang}&id=${paid.payment_intent_id}`)).status()).toBe(200);
  const form=admin.locator(`form[action^="${path}?"]`);
  const fields=await form.evaluate(node=>Object.fromEntries(new FormData(node)));
  await form.locator('[name=reason]').selectOption('evidence_reviewed');
  const [response]=await Promise.all([
    admin.waitForResponse(r=>r.request().method()==='POST'&&new URL(r.url()).pathname===path),
    form.getByRole('button').click(),
  ]);
  expect(response.status(),'reconciliation committed').toBe(303);
  await expect(admin).toHaveURL(new RegExp(`${path}.*id=${paid.payment_intent_id}`));
  expect((await db.query('SELECT after FROM audit_log WHERE request_id=$1 AND target=$2',[fields.request_id,paid.payment_intent_id])).rows.length,'reconciliation audit persisted').toBeGreaterThan(0);
  const operation=(await db.query('SELECT result FROM commerce_operations WHERE id=$1',[fields.request_id])).rows[0];
  expect(operation.result,'supported reconciliation commits resolution').toMatchObject({resolution_code:'reviewed_no_payment_state_change',review_version:fields.version});
  const beforeBad=await form.evaluate(node=>Object.fromEntries(new FormData(node)));
  const refusal=await admin.request.post(path,{headers:{origin:new URL(admin.url()).origin},maxRedirects:0,
    form:{...beforeBad,id:'00000000-0000-7000-8000-000000000001',reason:'evidence_reviewed'}});
  expect(refusal.status(),'unknown intent is invalid').toBe(422);
  expect((await db.query('SELECT count(*)::int n FROM commerce_operations WHERE target_id=$1 AND result IS NOT NULL',['00000000-0000-7000-8000-000000000001'])).rows[0].n).toBe(0);
 } finally {await staff.close();await db.end();}
});
