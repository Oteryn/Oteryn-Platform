// Platform-owned consumer qualification; no Game producer or resync simulation.
// Test launcher: presentation=true with synthetic authority/privacy; no fence or baseline.
import {test,expect} from '../fixtures.mjs';
import {register,login,submit,pool} from '../helpers.mjs';
import * as OTPAuth from 'otpauth';
import fs from 'node:fs';
import crypto from 'node:crypto';

test('admin analytics pages fail closed without an accepted resync baseline',async({page},info)=>{
 const lang=info.project.metadata.locale;
 const staff=await register(page,lang);
 const db=pool();
 try {
  await db.query("INSERT INTO account_roles(account_id,role_id) SELECT a.id,r.id FROM accounts a CROSS JOIN roles r WHERE a.email=$1 AND r.key='platform_admin'",[staff.email]);
 } finally {await db.end();}
 await login(page,lang,staff);
 expect((await page.goto('/admin/analytics')).status()).toBe(403);
 await page.goto(`/${lang}/mfa`);
 await submit(page,'main form[action$="/mfa/enroll"]',{password:staff.password});
 const secret=await page.locator('main .op-prose').innerText();
 const totp=new OTPAuth.TOTP({secret,algorithm:'SHA1',digits:6,period:30});
 await submit(page,'main form[action$="/mfa/confirm"]',{password:staff.password,code:totp.generate()});
 const codes=await page.locator('textarea[readonly]').allTextContents();
 await login(page,lang,staff);
 await submit(page,'main form[action$="/mfa/challenge"]',{code:codes[0]});
 for(const suffix of ['', '/creatures','/activity','/levels','/health','/economy']) {
  const path=`/admin/analytics${suffix}`;
  const response=await page.goto(path);
  expect(response.status(),path).toBe(200);
  expect(response.headers()['cache-control']).toBe('no-store, private');
  expect(response.headers()['strict-transport-security']).toBe('max-age=63072000; includeSubDomains');
  expect(response.headers()['x-content-type-options']).toBe('nosniff');
  expect(response.headers()['referrer-policy']).toBe('strict-origin-when-cross-origin');
  await expect(page.locator('html')).toHaveAttribute('lang',lang);
  await expect(page.locator('[data-state="unavailable"]')).toHaveCount(suffix==='/economy'?5:1);
  await expect(page.locator('[data-state="unavailable"]').first()).toBeVisible();
  await expect(page.locator('main a[href*="/characters/"]')).toHaveCount(0);
  const json=await page.request.get(`${path}/data`);
  expect(json.status()).toBe(200);
  expect((await json.json()).state).toBe(suffix==='/economy'?'available':'unavailable');
  expect(json.headers()['cache-control']).toBe('no-store, private');
 }
 const audit=pool();
 try {
  const {rows}=await audit.query("SELECT count(*)::int AS n FROM audit_log WHERE action='admin.analytics.read' AND actor=(SELECT id FROM accounts WHERE email=$1)",[staff.email]);
  expect(rows[0].n).toBe(12);
 } finally {await audit.end();}
 // TEST ONLY accepted baseline, never a production resync operation.
 const fixture=JSON.parse(fs.readFileSync(new URL('../../crates/admin/tests/fixtures/admin_analytics/economy/market.json',import.meta.url),'utf8'));
 const circulation=JSON.parse(fs.readFileSync(new URL('../../crates/admin/tests/fixtures/admin_analytics/economy/circulation.json',import.meta.url),'utf8'));
 const authority=process.env.OTERYN_ADMIN_ANALYTICS_AUTHORITY;
 const privacy=process.env.OTERYN_ADMIN_ANALYTICS_PRIVACY;
 const generation=crypto.randomUUID();
 const seed=pool();
 try {
  await seed.query('BEGIN');
  await seed.query("INSERT INTO admin_analytics.fence(authority,epoch,privacy_floor) VALUES($1,1,$2) ON CONFLICT(authority) DO NOTHING",[authority,privacy]);
  await seed.query("INSERT INTO admin_analytics.generations(id,authority,epoch,baseline_digest,baseline_cut,accepted_at) VALUES($1,$2,1,$3,1,now())",[generation,authority,'a'.repeat(64)]);
  await seed.query("UPDATE admin_analytics.fence SET active_generation=$2 WHERE authority=$1",[authority,generation]);
  await seed.query("DELETE FROM admin_analytics.rows WHERE authority=$1",[authority]);
  const pages=[circulation,fixture,
   {...fixture,family:'economy_flows_daily',rows:['creature_loot','house_rent','market_fee','npc_service','npc_trade','refund'].map(key=>({key,state:'incomplete',values:[]}))},
   {...fixture,family:'economy_trades_daily',rows:[{key:'world',state:'low_n',values:['2','900']}]},
   {...fixture,family:'economy_store_daily',rows:[{key:'refund/coin',state:'incomplete',values:[]},{key:'spent_in_game/coin',state:'low_n',values:['1','35']}]},
  ];
  for(const p of pages){
   await seed.query("INSERT INTO admin_analytics.subjects(authority,family,world,bucket_start,bucket_end,epoch,revision,privacy,observed,commit_digest,erased,generation) VALUES($1,$2,$3,$4,$5,1,1,$6,$7,$8,false,$9) ON CONFLICT(authority,family,world,bucket_start) DO UPDATE SET generation=excluded.generation",[authority,p.family,p.world_id,p.bucket_start,p.bucket_end,privacy,p.source_observed_at,'b'.repeat(64),generation]);
   await seed.query("INSERT INTO admin_analytics.watermarks(authority,family,world,epoch,privacy,cut,observed,digest) VALUES($1,$2,$3,1,$4,$5,$5,$6) ON CONFLICT(authority,family,world) DO UPDATE SET cut=excluded.cut,observed=excluded.observed",[authority,p.family,p.world_id,privacy,String(Math.floor(Date.now()/1000)),'c'.repeat(64)]);
   for(const r of p.rows)await seed.query("INSERT INTO admin_analytics.rows(generation,authority,family,world,bucket_start,bucket_end,key,state,values,epoch,revision,privacy,observed) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,1,1,$10,$11)",[generation,authority,p.family,p.world_id,p.bucket_start,p.bucket_end,r.key,r.state,JSON.stringify(r.values),privacy,p.source_observed_at]);
  }
  await seed.query('COMMIT');
 } finally {await seed.end();}
 const response=await page.goto(`/admin/analytics/economy?world=${fixture.world_id}&start=${fixture.bucket_start}&end=${fixture.bucket_end}`);
 expect(response.status()).toBe(200);
 for(const text of ['48210377','4500','low-n (below five)','depot source coverage not yet delivered by Game','spent_in_game/coin','Platform wallet ledger'])await expect(page.locator('main')).toContainText(text);
 await expect(page.locator('main a[href*="/characters/"]')).toHaveCount(0);
 expect((await page.request.get('/admin/analytics/export')).status()).toBe(404);
});
