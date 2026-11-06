import {test,expect} from '@playwright/test';
import crypto from 'node:crypto';
import {TOTP,Secret} from 'otpauth';
import {register,login,submit,pool} from '../helpers.mjs';
import {uuid7} from '../support/bazaar-game.mjs';
const world='0192a000-0000-7000-8000-000000000012';
let seller,buyer,sellerContext,buyerContext,sellerID,buyerID;
let listingTimes=[];
async function accountID(email){const db=pool();try{return (await db.query('SELECT id FROM accounts WHERE email=$1',[email])).rows[0].id;}finally{await db.end();}}
async function enroll(page,lang,account){
 await page.goto(`/${lang}/mfa`);await submit(page,'main form[action$="/mfa/enroll"]',{password:account.password});
 const encoded=(await page.locator('main p.op-prose').innerText()).trim();const totp=new TOTP({secret:Secret.fromBase32(encoded),algorithm:'SHA1',digits:6,period:30});
 const enrollmentStep=Math.floor(Date.now()/30000);
 await submit(page,'main form[action$="/mfa/confirm"]',{password:account.password,code:totp.generate()});
 await expect(page.locator('main textarea')).not.toHaveCount(0);
 await expect.poll(()=>Math.floor(Date.now()/30000),{timeout:35000,intervals:[500]}).toBeGreaterThan(enrollmentStep);
 await login(page,lang,account);await expect(page).toHaveURL(/mfa\/challenge/);
 await submit(page,'main form[action$="/mfa/challenge"]',{code:totp.generate()});
 await page.goto(`/${lang}/account/sudo`);await submit(page,'main form',{current_password:account.password});
}
test.beforeAll(async({browser},info)=>{
 expect(process.env.OTERYN_BAZAAR_ENABLED).toBe('true');
 const settings={ignoreHTTPSErrors:true,viewport:info.project.use.viewport,locale:info.project.metadata.locale,baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443'};
 sellerContext=await browser.newContext(settings);buyerContext=await browser.newContext(settings);
 const sellerPage=await sellerContext.newPage(),buyerPage=await buyerContext.newPage(),lang=info.project.metadata.locale;
 seller=await register(sellerPage,lang);await login(sellerPage,lang,seller);await enroll(sellerPage,lang,seller);
 buyer=await register(buyerPage,lang);await login(buyerPage,lang,buyer);
 sellerID=await accountID(seller.email);buyerID=await accountID(buyer.email);
 const db=pool();try{
  await db.query('INSERT INTO wallet_accounts(account_id) VALUES($1) ON CONFLICT DO NOTHING',[buyerID]);
  const operation=uuid7();await db.query("INSERT INTO wallet_operations(id,kind,account_id,amount,reason) VALUES($1,'adjustment',$2,50000,'disposable Bazaar E2E fixture')",[operation,buyerID]);
  await db.query('INSERT INTO wallet_entries(id,operation_id,account_id,available_delta,reserved_delta) VALUES($1,$2,$3,50000,0)',[uuid7(),operation,buyerID]);
 }finally{await db.end();}
 const response=await sellerPage.request.get('/__bazaar_game/fixtures');expect(response.headers()['x-fixture-source']).toBe('3b49a20feec3ba29700bb4b2fa28fa9b539b79a2');
 for(const vector of (await response.json()).vectors)expect(crypto.createHash('sha256').update(vector.wire).digest('hex')).toBe(vector.sha256);
 await sellerPage.close();await buyerPage.close();
});
test.afterAll(async()=>{await sellerContext?.close();await buyerContext?.close();});
async function list(page,lang,kind,refuse=false,currency='COINS'){
 listingTimes=listingTimes.filter(t=>Date.now()-t<61000);
 if(listingTimes.length>=3)await new Promise(resolve=>setTimeout(resolve,61000-(Date.now()-listingTimes[0])));
 listingTimes.push(Date.now());
 const character=uuid7(),asset=kind==='character'?character:uuid7(),destination=uuid7();
 const seeded=await page.request.post('/__bazaar_game/seed',{data:{seller:sellerID,buyer:buyerID,kind,asset_id:asset,seller_character_id:character,buyer_character_id:destination,refuse}});expect(seeded.status()).toBe(201);
 await page.goto(`/${lang}/account/bazaar/sell`);
 await page.locator('[name=kind]').selectOption(kind);await page.locator('[name=currency]').selectOption(currency);
 await submit(page,'main form',{world_id:world,seller_character_id:character,asset_id:asset,expected_revision:'7',quantity:'1',definition_key:'fixture-sword',definition_revision:'v1',starting_bid:'10000',duration_days:'1',publish_snapshot:true});
 await expect(page).toHaveURL(/\/bazaar\/[a-f0-9-]+$/);
 return {id:new URL(page.url()).pathname.split('/').at(-1),asset,destination};
}
for(const kind of ['character','item'])test(`${kind}: listing, funded bid, Game receipt, delivery and seller credit`,async({},info)=>{
 const lang=info.project.metadata.locale,sellerPage=await sellerContext.newPage(),buyerPage=await buyerContext.newPage();
 try{
  const {id,asset,destination}=await list(sellerPage,lang,kind);
  await expect.poll(async()=>{await sellerPage.reload();return await sellerPage.locator(`[data-listing="${id}"]`).getAttribute('data-state');},{timeout:45000,intervals:[1000]}).toBe('active');
  await buyerPage.goto(`/${lang}/bazaar/${kind==='item'?'items/':''}${id}`);
  await submit(buyerPage,'main form',{buyer_character_id:destination,maximum:'15000'});
  const db=pool();try{
   const held=await db.query('SELECT amount,completed FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2',[id,buyerID]);expect(Number(held.rows[0].amount)).toBe(15000);expect(held.rows[0].completed).toBe(false);
   const pendingCredit=await db.query('SELECT COALESCE(sum(available_delta),0)::bigint AS balance FROM wallet_entries WHERE account_id=$1',[sellerID]);
   const before=Number(pendingCredit.rows[0].balance);
   // Fixture time advancement only. The browser submitted an actual funded bid;
   // production waits for the real 1..14 day auction deadline.
   await db.query("UPDATE bazaar_listings SET ends_at=clock_timestamp()-interval '1 second' WHERE id=$1",[id]);
   await expect.poll(async()=> (await db.query('SELECT state::text FROM bazaar_listings WHERE id=$1',[id])).rows[0].state,{timeout:45000,intervals:[1000]}).toBe('completed');
   const after=Number((await db.query('SELECT COALESCE(sum(available_delta),0)::bigint AS balance FROM wallet_entries WHERE account_id=$1',[sellerID])).rows[0].balance);expect(after-before).toBe(9000);
   const hold=(await db.query('SELECT amount,completed FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2',[id,buyerID])).rows[0];expect(Number(hold.amount)).toBe(0);expect(hold.completed).toBe(true);
   const sale=(await db.query('SELECT id FROM bazaar_settlements WHERE listing_id=$1',[id])).rows[0];
   expect(Number((await db.query("SELECT count(*) FROM wallet_operations WHERE reference_id=$1 AND kind='capture'",[sale.id])).rows[0].count)).toBe(1);
   await sellerPage.request.post('/__bazaar_game/replay',{data:{listing_id:id}});
   await expect.poll(async()=>{const state=await (await sellerPage.request.get(`/__bazaar_game/state?asset=${asset}`)).json();return state.asset?.owner;},{timeout:15000}).toBe(buyerID);
   if(kind==='item')expect((await (await sellerPage.request.get(`/__bazaar_game/state?asset=${asset}`)).json()).asset.inbox).toBe(buyerID);
   if(kind==='character')await expect.poll(async()=> (await (await sellerPage.request.get(`/__bazaar_game/state?asset=${asset}`)).json()).asset?.locked,{timeout:30000,intervals:[1000]}).toBe(false);
   expect(Number((await db.query("SELECT count(*) FROM wallet_operations WHERE reference_id=$1 AND kind='capture'",[sale.id])).rows[0].count)).toBe(1);
  }finally{await db.end();}
  await buyerPage.reload();await expect(buyerPage.locator('main')).toContainText(lang==='pl'?'Sprzedaż zakończona':'Sale completed');await expect(buyerPage.locator('html')).toHaveAttribute('lang',lang);
 }finally{await sellerPage.close();await buyerPage.close();}
});
test('Game refusal stays closed: no listing activation, transfer or credit',async({},info)=>{
 const lang=info.project.metadata.locale,page=await sellerContext.newPage();
 try{
  const {id,asset}=await list(page,lang,'item',true);
  await expect.poll(async()=>{await page.reload();return await page.locator('main').innerText();},{timeout:30000,intervals:[1000]}).toContain(lang==='pl'?'Gra odrzuciła':'Game authority refused');
  const db=pool();try{
   expect((await db.query('SELECT state::text FROM bazaar_listings WHERE id=$1',[id])).rows[0].state).toBe('draft');
   expect(Number((await db.query('SELECT count(*) FROM bazaar_settlements WHERE listing_id=$1',[id])).rows[0].count)).toBe(0);
   expect(Number((await db.query('SELECT count(*) FROM bazaar_coin_holds WHERE id=$1',[id])).rows[0].count)).toBe(0);
  }finally{await db.end();}
  const game=await (await page.request.get(`/__bazaar_game/state?asset=${asset}`)).json();expect(game.asset.owner).toBe(sellerID);expect(game.asset.playable).toBe(true);expect(game.asset.escrow).toBeUndefined();
  const publicView=await buyerContext.request.get(`/${lang}/bazaar/${id}`);expect(publicView.status()).toBe(404);
 }finally{await page.close();}
});

for(const kind of ['item','character'])test(`${kind}: Game Gold hold, delivery, payout and excess release`,async({},info)=>{
 const lang=info.project.metadata.locale,page=await sellerContext.newPage(),bidder=await buyerContext.newPage();
 try{
  const {id,asset,destination}=await list(page,lang,kind,false,'GOLD');
  await expect.poll(async()=>{await page.reload();return await page.locator(`[data-listing="${id}"]`).getAttribute('data-state');},{timeout:45000,intervals:[1000]}).toBe('active');
  const initial=(await (await page.request.get(`/__bazaar_game/state?asset=${asset}`)).json()).banks;
  await bidder.goto(`/${lang}/bazaar/${kind==='item'?'items/':''}${id}`);await submit(bidder,'main form',{buyer_character_id:destination,maximum:'15000'});
  const db=pool();try{
   await expect.poll(async()=>Number((await db.query('SELECT count(*) FROM bazaar_bids WHERE listing_id=$1',[id])).rows[0].count),{timeout:30000,intervals:[1000]}).toBe(1);
   await db.query("UPDATE bazaar_listings SET ends_at=clock_timestamp()-interval '1 second' WHERE id=$1",[id]);
   await expect.poll(async()=>(await db.query('SELECT state::text FROM bazaar_listings WHERE id=$1',[id])).rows[0].state,{timeout:45000,intervals:[1000]}).toBe('completed');
   const sale=(await db.query('SELECT gross_amount,fee_amount,seller_amount FROM bazaar_authority_gold_sales WHERE listing_id=$1 AND completed_at IS NOT NULL',[id])).rows[0];
   expect(Number(sale.gross_amount)).toBe(10000);expect(Number(sale.fee_amount)).toBe(250);expect(Number(sale.seller_amount)).toBe(9750);
   expect(Number((await db.query('SELECT count(*) FROM bazaar_coin_holds WHERE id=$1',[id])).rows[0].count)).toBe(0);
   expect(Number((await db.query('SELECT count(*) FROM bazaar_settlements WHERE listing_id=$1',[id])).rows[0].count)).toBe(0);
  }finally{await db.end();}
  const game=(await (await page.request.get(`/__bazaar_game/state?asset=${asset}`)).json());
  expect(game.asset.owner).toBe(buyerID);if(kind==='item')expect(game.asset.inbox).toBe(buyerID);expect(game.banks[buyerID].balance).toBe(initial[buyerID].balance-10000);expect(game.banks[buyerID].held).toBe(0);expect(game.banks[sellerID].balance).toBe(initial[sellerID].balance+9750);expect(game.banks[sellerID].incoming).toBe(0);
  if(kind==='character')await expect.poll(async()=>(await (await page.request.get(`/__bazaar_game/state?asset=${asset}`)).json()).asset.locked,{timeout:30000}).toBe(false);
 }finally{await page.close();await bidder.close();}
});

test('unsold item cancellation waits for explicit Game return',async({},info)=>{
 const lang=info.project.metadata.locale,page=await sellerContext.newPage();
 try{
  const {id,asset}=await list(page,lang,'item');
  await expect.poll(async()=>{await page.reload();return await page.locator(`[data-listing="${id}"]`).getAttribute('data-state');},{timeout:45000,intervals:[1000]}).toBe('active');
  // Browser CSRF-protected action records a request; it never declares custody returned.
  const form=page.locator('main form[action$="/control"]');
  await form.locator('button[value="cancel"]').click();
  await expect.poll(async()=>{await page.reload();return await page.locator(`[data-listing="${id}"]`).getAttribute('data-state');},{timeout:45000,intervals:[1000]}).toBe('cancelled');
  const game=await (await page.request.get(`/__bazaar_game/state?asset=${asset}`)).json();expect(game.asset.owner).toBe(sellerID);expect(game.asset.consumed).toBe(true);
  expect(game.decisions.some(d=>d.command==='ReturnBazaarItem'&&d.outcome==='COMMITTED')).toBe(true);
 }finally{await page.close();}
});
