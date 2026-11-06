import { expect } from '@playwright/test';
import pg from 'pg';
import crypto from 'node:crypto';
import dns from 'node:dns';
dns.setDefaultResultOrder('ipv4first');
export const password = () => `Local!${crypto.randomBytes(18).toString('hex')}`;
export const pool = () => {
 const connectionString=process.env.PORTAL_E2E_DATABASE_URL || process.env.OTERYN_DATABASE_URL;
 if(!connectionString)throw new Error('Set PORTAL_E2E_DATABASE_URL to the disposable local database');
 const url=new URL(connectionString);
 if(!['localhost','127.0.0.1','[::1]'].includes(url.hostname)||!url.pathname.startsWith('/portal_e2e_'))throw new Error('Fixtures require a local portal_e2e_ database');
 return new pg.Pool({connectionString});
};
export async function csrfRejected(page, action, fields={}) {
  const clean=Object.fromEntries(Object.entries(fields).filter(([key,value])=>key!=='_csrf' && value!==undefined));
  const payload=new URL(action,page.url()).pathname==='/admin/media'?{multipart:{alt_text:'Local test image',image:{name:'local.png',mimeType:'image/png',buffer:Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jD1sAAAAASUVORK5CYII=','base64')}}}:{form:clean};
  const response=await page.request.post(action,{...payload,headers:{origin:new URL(page.url()).origin},maxRedirects:0});
  expect(response.status(), `missing CSRF: ${action}`).toBe(403);
}
export async function submit(page, selector='main form', values={}) {
  const form=page.locator(selector).first();
  const action=await form.getAttribute('action') || new URL(page.url()).pathname;
  const fields=await form.evaluate(f=>Object.fromEntries(new FormData(f)));
  await csrfRejected(page,action,{...fields,...values,_csrf:undefined});
  for(const [name,value] of Object.entries(values)) {
    const input=form.locator(`[name="${name}"]`);
    if(await input.getAttribute('type')==='checkbox') await input.setChecked(Boolean(value));
    else await input.fill(value);
  }
  await form.locator('button[type=submit], button:not([type])').first().click();
}
export async function mailLink(page,email,pattern) {
  let found;
  await expect.poll(async()=>{
    const response=await page.request.get('/__mail');
    const messages=await response.json();
    const message=messages.filter(m=>m.to.includes(email)).reverse().find(m=>pattern.test(m.raw.replace(/=\r?\n/g,'')));
    if(!message)return false;
    const raw=message.raw.replace(/=\r?\n/g,'').replace(/=3D/g,'=');
    found=raw.match(pattern)?.[0];return Boolean(found);
  },{timeout:30_000}).toBe(true);
  return found;
}
export async function seedPasswordRange(value) {
  const db=pool();
  try {
    await expect.poll(async()=>Number((await db.query("SELECT COALESCE(max(count),0) AS count FROM web_rate_limits WHERE limiter_name='identity-registration' AND window_start=date_trunc('minute',clock_timestamp())")).rows[0].count),{timeout:65000,intervals:[1000]}).toBeLessThan(5);
    await db.query('UPDATE registration_abuse_policy SET enabled=true WHERE singleton=true');
    await db.query("INSERT INTO world_status(id,name) SELECT uuidv7(),'Local Browser World' WHERE NOT EXISTS(SELECT 1 FROM world_status)");
    // Synthetic local corpus, matching the identity crate's registration fixtures.
    const prefix=crypto.createHash('sha1').update(value).digest('hex').toUpperCase().slice(0,5);
    await db.query("INSERT INTO registration_password_ranges(prefix,suffixes,expires_at) VALUES($1,ARRAY[]::text[],now()+interval '1 hour') ON CONFLICT(prefix) DO UPDATE SET suffixes=EXCLUDED.suffixes,expires_at=EXCLUDED.expires_at",[prefix]);
  } finally { await db.end(); }
}
export async function register(page,lang) {
  const email=`browser-${crypto.randomUUID()}@local.test`, pass=password();
  await seedPasswordRange(pass);
  const registration=await page.goto(`/${lang}/register`);
  expect(registration.status()).toBe(200);
  await page.locator('[name=work_token]').waitFor({state:'attached'});
  await submit(page,'main form',{email,password:pass,password_confirmation:pass,terms:true,privacy:true});
  await expect(page).toHaveURL(new RegExp(`/${lang}/register/check-inbox`));
  const link=await mailLink(page,email,/https:\/\/localhost:18443\/(?:pl|en)\/verify-email\?token=[a-f0-9]+/);
  await page.goto(link);
  const form=page.locator('main form');
  if(await form.count()) await submit(page);
  return {email,password:pass};
}
export async function logout(page,lang) {
 await page.goto(`/${lang}/`);
 const logout=page.locator('header form[action$="/logout"]');
 if(await logout.count()) await submit(page,'header form[action$="/logout"]');
}
export async function waitForLoginBudget(needed=1) {
 const db=pool();
 try {
  if(needed>=6) await expect.poll(async()=>Number((await db.query("SELECT extract(second FROM clock_timestamp()) AS second")).rows[0].second),{timeout:65000,intervals:[1000]}).toBeLessThan(50);
  await expect.poll(async()=>Number((await db.query("SELECT COALESCE(max(count),0) AS count FROM web_rate_limits WHERE limiter_name='identity-login-source' AND window_start=date_trunc('minute',clock_timestamp())")).rows[0].count),{timeout:65000,intervals:[1000]}).toBeLessThanOrEqual(20-needed);
 } finally {await db.end();}
}
export async function login(page,lang,account) {
 await logout(page,lang);
 await page.goto(`/${lang}/login`);
 await waitForLoginBudget();
 await submit(page,'main form',{email:account.email,password:account.password});
}
