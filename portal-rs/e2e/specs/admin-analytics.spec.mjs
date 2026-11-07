// Platform-owned consumer qualification; no Game producer or resync simulation.
import {test,expect} from '@playwright/test';
import {register,login,submit,pool} from '../helpers.mjs';
import * as OTPAuth from 'otpauth';

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
 for(const suffix of ['', '/creatures','/activity','/levels','/health']) {
  const path=`/admin/analytics${suffix}`;
  const response=await page.goto(path);
  expect(response.status(),path).toBe(200);
  expect(response.headers()['cache-control']).toBe('no-store, private');
  await expect(page.locator('html')).toHaveAttribute('lang',lang);
  await expect(page.locator('[data-state="unavailable"]')).toBeVisible();
  await expect(page.locator('main a[href*="/characters/"]')).toHaveCount(0);
  const json=await page.request.get(`${path}/data`);
  expect(json.status()).toBe(200);
  expect((await json.json()).state).toBe('unavailable');
  expect(json.headers()['cache-control']).toBe('no-store, private');
 }
 const audit=pool();
 try {
  const {rows}=await audit.query("SELECT count(*)::int AS n FROM audit_log WHERE action='admin.analytics.read' AND actor=(SELECT id FROM accounts WHERE email=$1)",[staff.email]);
  expect(rows[0].n).toBe(10);
 } finally {await audit.end();}
 expect((await page.request.get('/admin/analytics/export')).status()).toBe(404);
});
