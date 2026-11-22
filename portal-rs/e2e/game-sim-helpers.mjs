import crypto from 'node:crypto';
import path from 'node:path';
import {TOTP,Secret} from 'otpauth';
import {expect} from './fixtures.mjs';
import {register,login,submit,pool,csrfRejected} from './helpers.mjs';
import {uuid7} from './support/bazaar-game.mjs';
export {uuid7};
export async function actor(browser,info,{staff=false,mfa=false,funded=false}={}) {
 const lang=info.project.metadata.locale;
 const context=await browser.newContext({baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443',ignoreHTTPSErrors:true,locale:lang,viewport:info.project.use.viewport,...(staff?{storageState:path.join(process.env.PORTAL_E2E_AUTH_DIRECTORY,`staff-${info.project.name}.json`)}:{})});
 const page=await context.newPage();
 try {
  if(staff)return {context,page};
  const account=await register(page,lang);await login(page,lang,account);
  const db=pool();let id;
  try{id=(await db.query('SELECT id FROM accounts WHERE email=$1',[account.email])).rows[0].id;}finally{await db.end();}
  if(mfa){
   await page.goto(`/${lang}/mfa`);await submit(page,'main form[action$="/mfa/enroll"]',{password:account.password});
   const secret=(await page.locator('main p.op-prose').innerText()).trim();
   const totp=new TOTP({secret:Secret.fromBase32(secret),algorithm:'SHA1',digits:6,period:30});
   await submit(page,'main form[action$="/mfa/confirm"]',{password:account.password,code:totp.generate()});
   const recoveryCode=page.locator('textarea[readonly]').first();
   await expect(recoveryCode).toBeVisible();
   const recovery=await recoveryCode.inputValue();
   await login(page,lang,account);await expect(page,'seller must complete MFA').toHaveURL(/mfa\/challenge/);
   await submit(page,'main form[action$="/mfa/challenge"]',{code:recovery});
   await page.goto(`/${lang}/account/sudo`);await submit(page,'main form',{current_password:account.password});
  }
  if(funded){
   await page.goto(`/${lang}/shop/coins`);const operation=await page.locator('main form[action$="/shop/coins"] [name=operation]').inputValue();
   await submit(page,'main form[action$="/shop/coins"]');
   const db=pool();try{
    const t=(await db.query('SELECT t.*,p.provider_reference FROM store_web_topups t JOIN payment_intents p ON p.id=t.payment_intent_id WHERE t.operation_id=$1',[operation])).rows[0];
    const body=JSON.stringify({event_id:crypto.randomUUID(),intent_id:t.payment_intent_id,provider_reference:t.provider_reference,amount:{minor_units:250,currency:'PLN'},state:'succeeded'});
    const signature=crypto.createHmac('sha256',process.env.OTERYN_SHOP_TEST_PAYMENT_KEY).update(body).digest('base64');
    expect((await page.request.post('/__test/payments/webhook',{data:body,headers:{'content-type':'application/json','x-test-payment-signature':signature}})).status()).toBe(204);
    await expect.poll(async()=>Number((await db.query('SELECT count(*) n FROM wallet_entries WHERE operation_id=$1',[t.payment_intent_id])).rows[0].n),{message:'checkout must credit bidder through the worker',timeout:30000}).toBe(1);
   }finally{await db.end();}
  }
  return {context,page,account,id};
 }catch(error){await context.close();throw error;}
}
export async function listing(seller,buyer,lang,kind='item') {
 const character=uuid7(),asset=kind==='character'?character:uuid7(),destination=uuid7();
 const seeded=await seller.page.request.post('/__bazaar_game/seed',{data:{seller:seller.id,buyer:buyer.id,kind,asset_id:asset,seller_character_id:character,buyer_character_id:destination}});
 expect(seeded.status(),'existing fake Game must seed the asset').toBe(201);
 await seller.page.goto(`/${lang}/account/bazaar/sell`);
 await seller.page.locator('[name=kind]').selectOption(kind);
 await submit(seller.page,'main form',{world_id:'0192a000-0000-7000-8000-000000000012',seller_character_id:character,asset_id:asset,expected_revision:'7',quantity:'1',definition_key:'fixture-sword',definition_revision:'v1',starting_bid:'100',duration_days:'1',publish_snapshot:true});
 await expect(seller.page,'listing must be persisted before activation').toHaveURL(/\/bazaar\/[a-f0-9-]+$/);
 const id=new URL(seller.page.url()).pathname.split('/').at(-1);
 await expect.poll(async()=>{await seller.page.reload();return seller.page.locator(`[data-listing="${id}"]`).getAttribute('data-state');},{message:'Game receipt must activate listing',timeout:45000,intervals:[1000]}).toBe('active');
 return {id,asset,destination,url:`/${lang}/bazaar/${id}`};
}
export async function post(page,target,fields) {
 await csrfRejected(page,target,fields);
 const token=await page.locator('input[name=_csrf]').first().inputValue();
 return page.request.post(target,{form:{...fields,_csrf:token},headers:{origin:new URL(page.url()).origin},maxRedirects:0});
}
