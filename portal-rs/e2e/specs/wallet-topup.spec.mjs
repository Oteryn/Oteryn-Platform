import {test,expect} from '../fixtures.mjs';
import crypto from 'node:crypto';
import {register,login,submit,pool} from '../helpers.mjs';

test('paid package: browser checkout, signed webhook, worker credit and replay',async({page},info)=>{
 const lang=info.project.metadata.locale;
 const account=await register(page,lang);
 await login(page,lang,account);
 const db=pool();
 try {
  const response=await page.goto(`/${lang}/shop/coins`);
  expect(response.status()).toBe(200);
  await expect(page.locator('main')).toContainText('1234');
  const form=page.locator('main form[action$="/shop/coins"]').first();
  const operation=await form.locator('[name=operation]').inputValue();
  await submit(page,'main form[action$="/shop/coins"]');
  const {rows}=await db.query('SELECT t.*,p.provider_reference FROM store_web_topups t JOIN payment_intents p ON p.id=t.payment_intent_id WHERE t.operation_id=$1',[operation]);
  expect(rows).toHaveLength(1);
  const t=rows[0];
  expect(t.coins).toBe('1234'); expect(t.minor_units).toBe('250');
  await expect(page.locator('main')).toContainText(t.payment_intent_id);
  expect((await db.query('SELECT count(*)::int n FROM wallet_entries WHERE operation_id=$1',[t.payment_intent_id])).rows[0].n).toBe(0);
  const body=JSON.stringify({event_id:crypto.randomUUID(),intent_id:t.payment_intent_id,provider_reference:t.provider_reference,amount:{minor_units:250,currency:'PLN'},state:'succeeded'});
  const signature=crypto.createHmac('sha256',process.env.OTERYN_SHOP_TEST_PAYMENT_KEY).update(body).digest('base64');
  const send=()=>page.request.post('/__test/payments/webhook',{data:body,headers:{'content-type':'application/json','x-test-payment-signature':signature}});
  expect((await page.request.post('/__test/payments/webhook',{data:body,headers:{'x-test-payment-signature':'invalid'}})).status()).toBe(400);
  expect((await send()).status()).toBe(204);
  expect((await send()).status()).toBe(204);
  await expect.poll(async()=>Number((await db.query('SELECT count(*)::int n FROM wallet_entries WHERE operation_id=$1',[t.payment_intent_id])).rows[0].n)).toBe(1);
  const {rows:entries}=await db.query('SELECT o.kind,e.available_delta,e.reserved_delta FROM wallet_entries e JOIN wallet_operations o ON o.id=e.operation_id WHERE o.id=$1',[t.payment_intent_id]);
  expect(entries).toEqual([{kind:'top_up',available_delta:'1234',reserved_delta:'0'}]);
  // Redeliver after the credit but before another acknowledgement.
  await db.query("UPDATE outbox SET state='pending',run_after=now(),leased_until=NULL WHERE kind='wallet.top_up' AND idempotency_key=$1",[t.payment_intent_id]);
  await expect.poll(async()=>(await db.query("SELECT state FROM outbox WHERE kind='wallet.top_up' AND idempotency_key=$1",[t.payment_intent_id])).rows[0].state).toBe('done');
  await page.goto(`/${lang}/shop/history`);
  await expect(page.locator('main dl dd').first()).toHaveText('1234');
  expect((await db.query('SELECT count(*)::int n FROM wallet_entries WHERE operation_id=$1',[t.payment_intent_id])).rows[0].n).toBe(1);
 } finally {await db.end();}
});
