import {test,expect} from '../fixtures.mjs';
import {submit,pool,csrfRejected} from '../helpers.mjs';
import {actor,listing,post,uuid7} from '../game-sim-helpers.mjs';

test('outbid releases the former leader funds into the visible wallet',async({browser},info)=>{
 test.setTimeout(360000);
 const actors=[],lang=info.project.metadata.locale;
 try{
  const seller=await actor(browser,info,{mfa:true});actors.push(seller);
  const first=await actor(browser,info,{funded:true});actors.push(first);
  const second=await actor(browser,info,{funded:true});actors.push(second);
  const lot=await listing(seller,first,lang,'character');
  // The existing producer owns all synthetic Game facts, including this second
  // bidder's destination. Seed a distinct asset so the listed asset is untouched.
  const destination=uuid7();
  expect((await second.page.request.post('/__bazaar_game/seed',{data:{seller:seller.id,buyer:second.id,kind:'character',asset_id:uuid7(),seller_character_id:uuid7(),buyer_character_id:destination}})).status()).toBe(201);
  await first.page.goto(lot.url);await submit(first.page,'main form[action$="/bids"]',{buyer_character_id:lot.destination,maximum:'200'});
  await first.page.goto(`/${lang}/shop/history`);
  await expect(first.page.locator('main dl dd').first(),'first bid must debit available wallet funds').toHaveText('1034');
  await second.page.goto(lot.url);await submit(second.page,'main form[action$="/bids"]',{buyer_character_id:destination,maximum:'300'});
  await first.page.reload();
  await expect(first.page.locator('main dl dd').first(),'outbid leader must recover all held funds').toHaveText('1234');
  const db=pool();try{
   const hold=(await db.query('SELECT amount FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2',[lot.id,first.id])).rows[0];
   expect(Number(hold.amount),'former leader hold must be released').toBe(0);
   expect(Number((await db.query('SELECT amount FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2',[lot.id,second.id])).rows[0].amount)).toBe(300);
   const before=Number((await db.query('SELECT count(*) n FROM bazaar_bids WHERE listing_id=$1',[lot.id])).rows[0].n);
   await second.page.goto(lot.url);
   const refused=await post(second.page,`/${lang}/account/bazaar/${lot.id}/bids`,{request_id:uuid7(),buyer_character_id:destination,maximum:'0'});
   expect(refused.status(),'zero bid must be refused').toBe(422);
   expect(Number((await db.query('SELECT count(*) n FROM bazaar_bids WHERE listing_id=$1',[lot.id])).rows[0].n)).toBe(before);
  }finally{await db.end();}
 }finally{for(const a of actors)await a.context.close();}
});

test('owner cancels an unbid item and Game confirms its return; foreign cancel refused',async({browser},info)=>{
 test.setTimeout(240000);
 const actors=[],lang=info.project.metadata.locale;
 try{
  const seller=await actor(browser,info,{mfa:true});actors.push(seller);
  const viewer=await actor(browser,info);actors.push(viewer);
  const lot=await listing(seller,viewer,lang);
  await viewer.page.goto(lot.url);
  expect((await post(viewer.page,`/${lang}/account/bazaar/${lot.id}/control`,{request_id:uuid7(),action:'cancel'})).status(),'other player cannot cancel seller listing').toBe(403);
  const selector='main form[action$="/control"]';
  await csrfRejected(seller.page,`/${lang}/account/bazaar/${lot.id}/control`,{request_id:uuid7(),action:'cancel'});
  await seller.page.locator(`${selector} button[value=cancel]`).click();
  await expect.poll(async()=>{await seller.page.reload();return seller.page.locator(`[data-listing="${lot.id}"]`).getAttribute('data-state');},{message:'cancellation must wait for Game return receipt',timeout:45000,intervals:[1000]}).toBe('cancelled');
  const game=await (await seller.page.request.get(`/__bazaar_game/state?asset=${lot.asset}`)).json();
  expect(game.asset.owner).toBe(seller.id);expect(game.asset.consumed).toBe(true);
  expect(game.decisions.some(d=>d.command==='ReturnBazaarItem'&&d.outcome==='COMMITTED')).toBe(true);
  await viewer.page.goto(`/${lang}/bazaar/items`);
  await expect(viewer.page.locator(`[data-listing="${lot.id}"]`),'cancelled listing must leave public catalogue').toHaveCount(0);
 }finally{for(const a of actors)await a.context.close();}
});

test('staff suspension persists and removes an active listing from player views',async({browser},info)=>{
 test.setTimeout(240000);
 const actors=[],lang=info.project.metadata.locale;
 try{
  const seller=await actor(browser,info,{mfa:true});actors.push(seller);
  const viewer=await actor(browser,info);actors.push(viewer);
  const staff=await actor(browser,info,{staff:true});actors.push(staff);
  const lot=await listing(seller,viewer,lang);
  await viewer.page.goto(lot.url);
  expect((await post(viewer.page,`/admin/${lang}/bazaar`,{action:'suspend',listing:lot.id,reason:'Synthetic moderation evidence'})).status(),'ordinary player cannot moderate').toBe(403);
  expect((await staff.page.goto(`/admin/${lang}/bazaar`)).status()).toBe(200);
  await submit(staff.page,'main form:has([name=action][value=suspend])',{listing:lot.id,reason:'Synthetic moderation evidence'});
  await expect(staff.page,'successful moderation returns to queue').toHaveURL(new RegExp(`/admin/${lang}/bazaar$`));
  const db=pool();try{
   expect((await db.query('SELECT reason FROM bazaar_moderation_holds WHERE listing_id=$1',[lot.id])).rows).toEqual([{reason:'Synthetic moderation evidence'}]);
   expect(Number((await db.query("SELECT count(*) n FROM audit_log WHERE action='bazaar.suspend' AND target=$1",[lot.id])).rows[0].n),'staff action must have an audit record').toBe(1);
   expect((await db.query('SELECT state FROM bazaar_listings WHERE id=$1',[lot.id])).rows[0].state,'suspension retains custody while hiding the listing').toBe('active');
   expect((await db.query('SELECT id FROM bazaar_settlements WHERE listing_id=$1',[lot.id])).rows,'suspension must not create a seller payout').toEqual([]);
  }finally{await db.end();}
  // Suspension is a reversible custody hold, not a sale/cancellation: no seller
  // payout/refund or restore command exists in this UI. The Game still holds it.
  const game=await (await seller.page.request.get(`/__bazaar_game/state?asset=${lot.asset}`)).json();expect(game.asset.owner).toBe(seller.id);
  expect((await viewer.page.goto(lot.url)).status(),'suspended listing must not remain visible to players').toBe(404);
  await viewer.page.goto(`/${lang}/bazaar/items`);
  await expect(viewer.page.locator(`[data-listing="${lot.id}"]`),'moderation hold must remove public catalogue card').toHaveCount(0);
 }finally{for(const a of actors)await a.context.close();}
});

test('reporting documents unavailable moderation account authority without inventing Game facts',async({browser},info)=>{
 test.setTimeout(240000);
 const actors=[],lang=info.project.metadata.locale;
 try{
  const seller=await actor(browser,info,{mfa:true});actors.push(seller);
  const reporter=await actor(browser,info);actors.push(reporter);
  const lot=await listing(seller,reporter,lang);
  // moderation_web::configured_router injects PendingIntegration for Accounts
  // and PublicViews. Bazaar transport does not implement those adapters.
  await reporter.page.goto(`/${lang}/account/bazaar/${lot.id}/report`);
  const form=reporter.page.locator('main form');
  const fields=await form.evaluate(f=>Object.fromEntries(new FormData(f)));
  expect((await post(reporter.page,new URL(reporter.page.url()).pathname,{...fields,category:'fraud',detail:'Synthetic report with sufficient evidence'})).status(),'PendingIntegration must refuse report authority').toBe(403);
  const db=pool();try{expect((await db.query('SELECT id FROM bazaar_reports WHERE listing_id=$1',[lot.id])).rows).toEqual([]);}finally{await db.end();}
 }finally{for(const a of actors)await a.context.close();}
});
