import {test,expect} from '../fixtures.mjs';
import crypto from 'node:crypto';
import {register,login,submit,csrfRejected,pool} from '../helpers.mjs';
const v7=()=>{const s=crypto.randomUUID().split('');s[14]='7';return s.join('');};

// Disposable local read-model fixtures only. The Rust privacy_authority test
// separately proves authenticated Game push ingestion; no production seeding exists.
test('T45 owner settings control live online and public profile privacy',async({page},info)=>{
 test.setTimeout(180_000);
 const lang=info.project.metadata.locale,account=await register(page,lang);await login(page,lang,account);
 const db=pool(),character=v7(),world=v7(),foreign=v7(),foreignCharacter=v7();
 const name=`Privacy${crypto.randomBytes(4).toString('hex')}`,foreignName=`Foreign${crypto.randomBytes(4).toString('hex')}`;
 const authority=process.env.OTERYN_LCFA_SOURCE_AUTHORITY,publicAuthority=process.env.PORTAL_PUBLIC_PROJECTIONS_AUTHORITY;
 let timer,baseline,publicState,watermarks=[];
 try {
  const owner=(await db.query('SELECT id FROM accounts WHERE email=$1',[account.email])).rows[0].id;
  baseline=(await db.query('SELECT * FROM account_character_feed WHERE id=1')).rows[0];
  const epoch=baseline.highest_epoch||'1';
  publicState=(await db.query('SELECT highest_epoch::text FROM public_projection_state WHERE id=1')).rows[0].highest_epoch;
  const publicEpoch=publicState||'1';
  watermarks=(await db.query("SELECT * FROM public_snapshots WHERE family='character_profile' AND is_watermark")).rows;
  await db.query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'non-loginable-test','active','en',now())",[foreign,`${foreign}@example.invalid`]);
  const stamp=()=>Math.floor(Date.now()/1000);
  for(const id of [owner,foreign])await db.query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,$2,$3,'1',$4,$5)",[id,authority,epoch,stamp(),Buffer.alloc(32)]);
  for(const [id,c,n] of [[owner,character,name],[foreign,foreignCharacter,foreignName]])await db.query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,$4,'AVAILABLE')",[id,c,world,n]);
  await db.query('UPDATE public_projection_state SET highest_epoch=$1 WHERE id=1',[publicEpoch]);
  const profile={contract_version:1,operation:'PublishCharacterProfileV1',source_authority:publicAuthority,character_id:character,projection_epoch:publicEpoch,projection_revision:'1',source_observed_at:String(stamp()),profile:[{world_id:world,name,vocation:4,level:'412',guild:[],guild_visible:false}]};
  const online={contract_version:1,operation:'PublishWorldOnlineV1',source_authority:publicAuthority,world_id:world,projection_epoch:publicEpoch,projection_revision:'1',source_observed_at:String(stamp()),characters:[{name,vocation:4,level:'412'},{name:'UnknownPrivacySubject',vocation:4,level:'500'}]};
  await db.query("INSERT INTO public_snapshots(family,subject,epoch,version,watermark,payload) VALUES('character_profile',$1,$2,1,$3,$4),('world_online',$5,$2,1,$3,$6)",[character,publicEpoch,stamp(),profile,world,online]);
  const heartbeat=async()=>{
   await db.query('UPDATE account_character_feed SET highest_epoch=$1,watermark_epoch=$1,source_authority=$2,complete_through=$3,observed_at=$3 WHERE id=1',[epoch,authority,stamp()]);
   await db.query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES('character_profile','',true,$1,1,$2,$3) ON CONFLICT(family,subject,is_watermark) DO UPDATE SET epoch=$1,watermark=$2,payload=$3",[publicEpoch,stamp(),{source_authority:publicAuthority}]);
  };
  await heartbeat();timer=setInterval(()=>heartbeat().catch(()=>{}),5000);
  const target=`/${lang}/account/characters/${name}/profile`;
  expect((await page.goto(target)).status()).toBe(200);
  await expect(page.locator('[name=hidden]')).toBeChecked();
  for(const key of ['show_guild','show_account','show_status','show_house','show_skills','show_deaths','show_kills','show_online_time'])await expect(page.locator(`[name=${key}]`)).not.toBeChecked();
  const onlineBody=async()=>{const r=await page.request.get(`/${lang}/online`);expect(r.status()).toBe(200);expect(r.headers()['cache-control']).toContain('no-store');return r.text();};
  expect(await onlineBody()).not.toContain(name);
  await csrfRejected(page,target,{hidden:'0',show_status:'1'});
  await submit(page,'main form',{hidden:false,show_status:true,comment:'<script>private comment</script>'});
  await expect(page.locator('[name=comment]')).toHaveValue('<script>private comment</script>');
  expect(await onlineBody()).not.toContain(name); // account status remains the upper bound
  await page.goto(`/${lang}/account/privacy`);await submit(page,'main form',{public_status_visible:true});
  let body=await onlineBody();expect(body).toContain(name);expect(body).not.toContain('UnknownPrivacySubject');expect(body).not.toContain(owner);
  const profileResponse=await page.request.get(`/${lang}/characters/${name}`);expect(profileResponse.status()).toBe(200);const html=await profileResponse.text();expect(html).not.toContain('<script>private comment</script>');expect(html).toContain('private comment');
  await page.goto(target);await submit(page,'main form',{show_status:false});expect(await onlineBody()).not.toContain(name);
  await submit(page,'main form',{show_status:true,hidden:true});expect(await onlineBody()).not.toContain(name);
  expect((await page.request.get(`/${lang}/characters/${name}`)).status()).toBe(404);
  await submit(page,'main form',{hidden:false,hide_all_characters:true});expect(await onlineBody()).not.toContain(name);
  await submit(page,'main form',{hide_all_characters:false});expect(await onlineBody()).toContain(name);
  const csrf=await page.locator('main [name=_csrf]').inputValue();
  expect((await page.request.get(`/${lang}/account/characters/${foreignName}/profile`)).status()).toBe(404);
  const refused=await page.request.post(`/${lang}/account/characters/${foreignName}/profile`,{form:{_csrf:csrf,show_status:'1'},headers:{origin:new URL(page.url()).origin},maxRedirects:0});expect(refused.status()).toBe(404);
  const invalid=await page.request.post(target,{form:{_csrf:csrf,comment:'a'.repeat(501)},headers:{origin:new URL(page.url()).origin},maxRedirects:0});expect(invalid.status()).toBe(422);
  expect(Number((await db.query('SELECT count(*) AS n FROM character_profile_events WHERE account_id=$1',[owner])).rows[0].n)).toBeGreaterThan(0);
  expect(Number((await db.query('SELECT count(*) AS n FROM character_profile_events WHERE account_id=$1',[foreign])).rows[0].n)).toBe(0);
  for(const path of ['deaths','guilds','houses','stats/guilds','stats/world','wiki/places']){const response=await page.request.get(`/${lang}/${path}`);expect(response.status(),`${path}: Game contract gate`).toBe(503);expect(response.headers()['cache-control']).toContain('no-store');}
 } finally {
  clearInterval(timer);
  await db.query("DELETE FROM public_snapshots WHERE NOT is_watermark AND subject=ANY($1)",[[character,world]]);
  await db.query("DELETE FROM public_snapshots WHERE family='character_profile' AND is_watermark AND subject=''");
  for(const w of watermarks)await db.query('INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload,previous_snapshot) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(family,subject,is_watermark) DO UPDATE SET epoch=$4,version=$5,watermark=$6,payload=$7,previous_snapshot=$8',[w.family,w.subject,w.is_watermark,w.epoch,w.version,w.watermark,w.payload,w.previous_snapshot]);
  await db.query('DELETE FROM accounts WHERE id=$1',[foreign]);
  if(baseline)await db.query('UPDATE account_character_feed SET highest_epoch=$1,watermark_epoch=$2,source_authority=$3,complete_through=$4,observed_at=$5 WHERE id=1',[baseline.highest_epoch,baseline.watermark_epoch,baseline.source_authority,baseline.complete_through,baseline.observed_at]);
  if(publicState!==undefined)await db.query('UPDATE public_projection_state SET highest_epoch=$1 WHERE id=1',[publicState]);
  await db.end();
 }
});
