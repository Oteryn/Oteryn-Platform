import crypto from 'node:crypto';
import {test,expect} from '../fixtures.mjs';
import {submit,pool,csrfRejected} from '../helpers.mjs';
import {actor,post,uuid7} from '../game-sim-helpers.mjs';

test('character submission persists pending; another account cannot read the request',async({browser},info)=>{
 test.setTimeout(240000);
 const actors=[],lang=info.project.metadata.locale;
 try{
  const owner=await actor(browser,info);actors.push(owner);
  const viewer=await actor(browser,info);actors.push(viewer);
  // Existing closure producer only processes CloseAccount/ResolveAccountClosure;
  // no CreateCharacter receipt producer runs in local.mjs. Do not mint a result.
  const name='Sim '+Array.from(crypto.randomBytes(10),b=>String.fromCharCode(97+b%26)).join('');
  await owner.page.goto(`/${lang}/account/characters/new`);
  const world=await owner.page.locator('select[name=world] option').evaluateAll(options=>options.find(o=>o.value)?.value);
  expect(world,'registration/harness must provide an available world').toBeTruthy();
  await owner.page.locator('select[name=world]').selectOption(world);
  await submit(owner.page,'main form[method=post]',{name});
  await expect(owner.page,'valid character form must create a request').toHaveURL(/\/characters\/requests\/[a-f0-9-]+$/);
  const target=new URL(owner.page.url()).pathname,operation=target.split('/').at(-1);
  await expect(owner.page.locator('[role=status]'),'without CreateCharacter receipts result must remain unknown').toHaveText(lang==='pl'?'Trwa przetwarzanie prośby. Bez wyniku z gry rezultat pozostaje nieznany. Odśwież stronę, aby sprawdzić ponownie.':'Your request is being processed. Without a result from Game, its outcome is still unknown. Refresh this page to check again.');
  const db=pool();try{
   expect((await db.query('SELECT account_id,state,receipt_json FROM character_commands WHERE id=$1',[operation])).rows).toEqual([{account_id:owner.id,state:'PENDING',receipt_json:null}]);
  }finally{await db.end();}
  expect((await viewer.page.goto(target)).status(),'request ownership must be private').toBe(404);
  await owner.page.goto(target);await expect(owner.page.locator('[role=status]')).toContainText(lang==='pl'?'Bez wyniku z gry':'Without a result from Game');
 }finally{for(const a of actors)await a.context.close();}
});

test('minigame catalogue and hirelings expose documented unavailable state without Game results',async({browser},info)=>{
 test.setTimeout(180000);
 const player=await actor(browser,info),lang=info.project.metadata.locale;
 try{
  // Puzzles require an accepted WorldStore catalogue. local.mjs only supplies
  // an unsupported negative catalogue; no puzzle publisher exists. Collection
  // config injects Unavailable ownership; pass uses unavailable Game adapters.
  const db=pool();let published;
  try{published=(await db.query("SELECT EXISTS(SELECT 1 FROM puzzles_daily WHERE day=(clock_timestamp() AT TIME ZONE 'UTC')::date) present")).rows[0].present;}finally{await db.end();}
  // Do not assume an empty table: existing definitions also need the absent
  // accepted catalogue, and therefore give 503 rather than an empty 200 page.
  const puzzles=await player.page.goto(`/${lang}/puzzles`);
  expect(puzzles.status(),'puzzles cannot serve definitions without an accepted catalogue').toBe(published?503:200);
  expect(puzzles.headers()['cache-control']).toContain('no-store');
  if(!published)await expect(player.page.locator('main')).toContainText(lang==='pl'?'Dzisiejsze zagadki są niedostępne.':"Today's puzzles are unavailable.");
  await expect(player.page.locator('main form[method=post]'),'no answer form may invent an unpublished puzzle').toHaveCount(0);
  await player.page.goto(`/${lang}/account/sudo`);
  expect((await post(player.page,`/${lang}/puzzles/${uuid7()}/answer`,{choice:'0'})).status(),'unknown puzzle refuses an authorized answer').toBe(404);
  const collection=await player.page.goto(`/${lang}/collection/${uuid7()}`);expect(collection.status(),'collection ownership adapter is unavailable').toBe(503);expect(collection.headers()['cache-control']).toContain('no-store');
  const pass=await player.page.goto(`/${lang}/pass`);expect(pass.status(),'season source adapter is unavailable').toBe(503);expect(pass.headers()['cache-control']).toContain('no-store');
  // hirelings::web only mounts GET; command_outbox=false. There is no browser
  // hire/dismiss route to exercise without changing the product.
  const hirelings=await player.page.goto(`/${lang}/panel/hirelings`);expect(hirelings.status()).toBe(200);expect(hirelings.headers()['cache-control']).toContain('no-store');
  await expect(player.page.locator('main')).toContainText(lang==='pl'?'Wybór wypraw i zakupy czekają na zweryfikowane kontrakty sklepu i gry. Tutaj nie pobieramy coinów.':'Expedition selection and purchases await verified store and game contracts. No coins will be charged here.');
  await expect(player.page.locator('main form[method=post]'),'read-only hirelings must not offer unsupported commands').toHaveCount(0);
 }finally{await player.context.close();}
});

test('party ownership gap refuses creation and guilds refuse missing public authority',async({browser},info)=>{
 test.setTimeout(180000);
 const player=await actor(browser,info),lang=info.project.metadata.locale;
 try{
  // Party requires both LCFA ownership AND a fresh character_profile. None of
  // the three local Game producers publishes that public profile. Guilds are
  // read-only Game projections: no create/edit/delete post routes exist.
  await player.page.goto(`/${lang}/party`);
  const now=Math.floor(Date.now()/1000),fields={character:uuid7(),purpose:'hunt',min_level:'1',max_level:'100',profession:'any',starts_at:String(now+60),expires_at:String(now+3600),note:`Game sim ${crypto.randomUUID()}`};
  await player.page.goto(`/${lang}/account/sudo`);
  expect((await post(player.page,`/${lang}/party`,fields)).status(),'missing ownership/profile authority must deny creation').toBe(403);
  const db=pool();try{expect((await db.query('SELECT id FROM pd_party_posts WHERE account_id=$1',[player.id])).rows).toEqual([]);}finally{await db.end();}
  const guilds=await player.page.goto(`/${lang}/guilds`);expect(guilds.status(),'no guild projection publisher runs locally').toBe(503);expect(guilds.headers()['cache-control']).toContain('no-store');
 }finally{await player.context.close();}
});

test('account privacy persists while absent public Game projections remain unavailable to another viewer',async({browser},info)=>{
 test.setTimeout(240000);
 const actors=[],lang=info.project.metadata.locale;
 try{
  const owner=await actor(browser,info);actors.push(owner);
  const viewer=await actor(browser,info);actors.push(viewer);
  // Complements privacy.spec: no SQL Game snapshots or heartbeat replacement.
  // Closure publishes zero-character portfolios; Bazaar/league do not publish
  // world_online/character_profile. A visible character toggle needs that gap filled.
  await owner.page.goto(`/${lang}/account/privacy`);
  await submit(owner.page,'main form',{public_status_visible:true});
  await owner.page.reload();await expect(owner.page.locator('[name=public_status_visible]'),'privacy opt-in must persist').toBeChecked();
  await submit(owner.page,'main form',{public_status_visible:false});
  await owner.page.reload();await expect(owner.page.locator('[name=public_status_visible]'),'privacy opt-out must persist').not.toBeChecked();
  await csrfRejected(owner.page,`/${lang}/account/privacy`,{public_status_visible:'1'});
  await owner.page.reload();await expect(owner.page.locator('[name=public_status_visible]'),'CSRF refusal cannot change saved preference').not.toBeChecked();
  const online=await viewer.page.goto(`/${lang}/online`);expect(online.status(),'no fake producer publishes world_online').toBe(503);expect(online.headers()['cache-control']).toContain('no-store');
  await expect(viewer.page.locator('main'),'unavailable online view must not disclose owner identity').not.toContainText(owner.account.email);
 }finally{for(const a of actors)await a.context.close();}
});
