import {test,expect} from '@playwright/test';
import {register,login,submit,pool} from '../helpers.mjs';
test.setTimeout(240_000);
async function sudo(page,lang,account){await page.goto(`/${lang}/account/sudo`);await submit(page,'main form',{current_password:account.password});}
async function fault(account_id,mode){const response=await fetch(`${process.env.PORTAL_E2E_GAME_CLOSURE_CONTROL_URL}/fault`,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({account_id,mode})});expect(response.status).toBe(200);}
async function requestDeletion(page,lang,account){await login(page,lang,account);await sudo(page,lang,account);await page.goto(`/${lang}/account/privacy/delete`);await submit(page,'main form',{current_password:account.password,confirmation:'TERMINATE'});await expect(page).toHaveURL(/login/);}
// Test clock advancement only: no command, receipt, proof or eligibility is seeded.
async function advanceGrace(db,account){await db.query("UPDATE account_terminations SET created_at=now()-interval '15 days',scheduled_for=now()-interval '1 day' WHERE account_id=$1 AND cancelled_at IS NULL AND finalized_at IS NULL",[account]);}
test('verified Game receipt finalizes deletion after grace; finalized accounts cannot log in',async({page},info)=>{
 const lang=info.project.metadata.locale,account=await register(page,lang),db=pool();
 try{
  const id=(await db.query('SELECT id FROM accounts WHERE email=$1',[account.email])).rows[0].id;
  await requestDeletion(page,lang,account);
  let state=await db.query('SELECT finalized_at,scheduled_for>now() AS grace FROM account_terminations WHERE account_id=$1',[id]);expect(state.rows[0].finalized_at).toBeNull();expect(state.rows[0].grace).toBe(true);
  expect(Number((await db.query('SELECT count(*) AS n FROM plat3_closure_commands c JOIN plat3_closure_work w USING(termination_id) WHERE w.account_id=$1',[id])).rows[0].n)).toBe(0);
  await advanceGrace(db,id);
  await expect.poll(async()=>(await db.query('SELECT status FROM accounts WHERE id=$1',[id])).rows[0].status,{timeout:90_000}).toBe('closed');
  expect((await db.query('SELECT finalized_at FROM account_terminations WHERE account_id=$1',[id])).rows[0].finalized_at).not.toBeNull();
  expect((await db.query('SELECT resolution FROM plat3_closure_work WHERE account_id=$1',[id])).rows[0].resolution).toBe('FINALIZE');
  const evidence=await db.query("SELECT r.receipt_sha256,c.intent_sha256 FROM plat3_closure_receipts r JOIN plat3_closure_commands c USING(operation_id) JOIN plat3_closure_work w USING(termination_id) WHERE w.account_id=$1 AND c.command='CloseAccount'",[id]);expect(evidence.rows).toHaveLength(1);expect(evidence.rows[0].receipt_sha256.length).toBe(32);
  await login(page,lang,account);await expect(page).toHaveURL(/login/);await expect(page.locator('[role=alert]')).toBeVisible();
 }finally{await db.end();}
});
test('invalid Game signature stays pending; cancellation remains available after grace',async({page},info)=>{
 const lang=info.project.metadata.locale,account=await register(page,lang),db=pool();
 try{
  const id=(await db.query('SELECT id FROM accounts WHERE email=$1',[account.email])).rows[0].id;await fault(id,'invalid_signature');
  await requestDeletion(page,lang,account);await advanceGrace(db,id);
  await expect.poll(async()=>Number((await db.query("SELECT count(*) AS n FROM plat3_closure_commands c JOIN plat3_closure_work w USING(termination_id) WHERE w.account_id=$1 AND c.command='CloseAccount'",[id])).rows[0].n),{timeout:90_000}).toBe(1);
  await page.waitForTimeout(6500); // At least one real producer publication/worker cycle.
  expect((await db.query('SELECT finalized_at FROM account_terminations WHERE account_id=$1',[id])).rows[0].finalized_at).toBeNull();
  expect(Number((await db.query('SELECT count(*) AS n FROM plat3_closure_receipts r JOIN plat3_closure_commands c USING(operation_id) JOIN plat3_closure_work w USING(termination_id) WHERE w.account_id=$1',[id])).rows[0].n)).toBe(0);
  await login(page,lang,account);await sudo(page,lang,account);await page.goto(`/${lang}/account/privacy/delete`);await submit(page,'form[action$="/cancel"]',{current_password:account.password});
  const state=(await db.query('SELECT cancelled_at,finalized_at FROM account_terminations WHERE account_id=$1',[id])).rows[0];expect(state.cancelled_at).not.toBeNull();expect(state.finalized_at).toBeNull();
  expect((await db.query('SELECT resolution FROM plat3_closure_work WHERE account_id=$1',[id])).rows[0].resolution).toBe('CANCEL');
  const restoring=lang==='pl'?'Przywracanie konta w grze trwa':'Game restoration is pending';
  await expect(page.locator('main [role=status]').first()).toContainText(restoring);
  await fault(id,'valid');
  await expect.poll(async()=>Number((await db.query("SELECT count(*) AS n FROM plat3_closure_receipts r JOIN plat3_closure_commands c USING(operation_id) JOIN plat3_closure_work w USING(termination_id) WHERE w.account_id=$1 AND c.command='ResolveAccountClosure' AND convert_from(r.receipt,'UTF8')::jsonb->'result_data'->>'closure_state'='CANCELLED'",[id])).rows[0].n),{timeout:90_000}).toBe(1);
  await expect.poll(async()=>(await db.query("SELECT convert_from(snapshot,'UTF8')::jsonb->'closures'->0->>'closure_state' AS state FROM plat3_closure_snapshots WHERE account_id=$1",[id])).rows[0]?.state,{timeout:90_000}).toBe('CANCELLED');
  await page.goto(`/${lang}/account/privacy/delete`);
  await expect(page.locator('main [role=status]').first()).not.toContainText(restoring);
 }finally{await db.end();}
});
