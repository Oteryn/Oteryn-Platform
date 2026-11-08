import {test,expect} from '../fixtures.mjs';
import crypto from 'node:crypto';
import AxeBuilder from '@axe-core/playwright';
import {register,login,submit,csrfRejected,mailLink,pool} from '../helpers.mjs';

test('e-mail change: recovery proof, mail confirmation, revert and replay refusal',async({page},info)=>{
 test.setTimeout(180_000); // Delivery uses the existing once-per-minute worker schedule.
 const lang=info.project.metadata.locale, account=await register(page,lang);
 await login(page,lang,account);
 await page.goto(`/${lang}/account/recovery-key`);
 await submit(page,'main form',{current_password:account.password});
 const key=(await page.locator('.op-code').innerText()).trim();
 await submit(page,'main form',{current_password:account.password,recovery_key:key});
 const email=`changed-${crypto.randomUUID()}@local.test`;
 await page.goto(`/${lang}/account/security/email`);
 const values={email,email_confirmation:email,current_password:account.password,proof:key,recovery:true};
 const db=pool();
 try{
  const state=async()=>(await db.query('SELECT id,email,native_security_generation FROM accounts WHERE email=$1 OR email=$2',[account.email,email])).rows[0];
  const before=await state();
  await submit(page,'main form',{...values,current_password:'Incorrect local password'});
  expect((await state()).email).toBe(account.email);
  expect(Number((await db.query('SELECT count(*) AS n FROM account_email_changes WHERE account_id=$1',[before.id])).rows[0].n)).toBe(0);
  await submit(page,'main form',values);
  const confirm=await mailLink(page,email,/https:\/\/localhost:18443\/email-change\/confirm\/[a-f0-9]{64}/,75_000);
  const revert=await mailLink(page,account.email,/https:\/\/localhost:18443\/email-change\/revert\/[a-f0-9]{64}/,75_000);
  expect((await state()).email).toBe(account.email);
  await page.goto(confirm);await submit(page);
  await expect(page).toHaveURL(/login/);
  expect((await state()).email).toBe(email);
  expect(Number((await state()).native_security_generation)).toBe(Number(before.native_security_generation)+1);
  await login(page,lang,{...account,email});await expect(page).not.toHaveURL(/login/);
  await page.goto(revert);await submit(page);
  await expect(page).toHaveURL(/login/);
  expect((await state()).email).toBe(account.email);
  await page.goto(confirm);await submit(page);
  expect((await state()).email).toBe(account.email);
  await page.goto(revert);await submit(page);
  expect((await state()).email).toBe(account.email);
  await login(page,lang,account);await expect(page).not.toHaveURL(/login/);
 }finally{await db.end();}
});

test('notification preferences persist; signed unsubscribe is one-click, bounded and idempotent',async({page},info)=>{
 const lang=info.project.metadata.locale,account=await register(page,lang);await login(page,lang,account);
 await page.goto(`/${lang}/account/notifications`);
 await submit(page,'main form',{news_email:true,news_portal:true,support_email:true,support_portal:false});
 await page.reload();
 await expect(page.locator('[name=news_email]')).toBeChecked();await expect(page.locator('[name=support_portal]')).not.toBeChecked();
 const token=await page.locator('main [name=_csrf]').inputValue();
 const bad=await page.request.post(`/${lang}/account/notifications`,{form:{_csrf:token,news_email:'invalid'},headers:{origin:new URL(page.url()).origin}});
 expect(bad.status()).toBe(422);
 const db=pool();
 try{
  const id=(await db.query('SELECT id FROM accounts WHERE email=$1',[account.email])).rows[0].id;
  const payload=`unsubscribe:${id}:news:${Math.floor(Date.now()/1000)+3600}`;
  // The local sender's signing key supplies a valid delivery link, never an account session.
  const signature=crypto.createHmac('sha256',Buffer.from(process.env.OTERYN_NOTIFY_UNSUBSCRIBE_KEY)).update(payload).digest('hex');
  const link=`/${lang}/unsubscribe?token=${encodeURIComponent(`${payload}:${signature}`)}`;
  const enabled=async()=>(await db.query("SELECT email_enabled FROM account_notification_prefs WHERE account_id=$1 AND category='news'",[id])).rows[0].email_enabled;
  expect((await page.goto(link)).status()).toBe(200);expect(await enabled()).toBe(true);
  const forged=await page.request.post(`/${lang}/unsubscribe?token=invalid`,{form:{'List-Unsubscribe':'One-Click'}});expect(forged.status()).toBe(422);expect(await enabled()).toBe(true);
  // RFC 8058 uses the signed capability and no browser session/CSRF token.
  for(let i=0;i<2;i++){expect((await page.request.post(link,{form:{'List-Unsubscribe':'One-Click'}})).status()).toBe(200);}
  expect(await enabled()).toBe(false);
  await page.goto(`/${lang}/account/notifications`);await expect(page.locator('[name=news_email]')).not.toBeChecked();await expect(page.locator('[name=support_email]')).toBeChecked();
 }finally{await db.end();}
});

test('account deletion: password and confirmation refusal, request revokes sessions, then cancel',async({page},info)=>{
 const lang=info.project.metadata.locale,account=await register(page,lang);await login(page,lang,account);
 await page.goto(`/${lang}/account/sudo`);await submit(page,'main form',{current_password:account.password});
 await page.goto(`/${lang}/account/privacy/delete`);
 const db=pool();
 try{
  const pending=async()=>Number((await db.query('SELECT count(*) AS n FROM account_terminations t JOIN accounts a ON a.id=t.account_id WHERE a.email=$1 AND t.cancelled_at IS NULL',[account.email])).rows[0].n);
  await csrfRejected(page,`/${lang}/account/privacy/delete`,{current_password:account.password,confirmation:'TERMINATE'});
  const token=await page.locator('main [name=_csrf]').inputValue();
  for(const [password,confirmation,status] of [['Incorrect local password','TERMINATE',403],[account.password,'invalid',422]]){
   const refused=await page.request.post(`/${lang}/account/privacy/delete`,{form:{_csrf:token,current_password:password,confirmation},headers:{origin:new URL(page.url()).origin},maxRedirects:0});expect(refused.status()).toBe(status);expect(await pending()).toBe(0);
  }
  await submit(page,'main form',{current_password:account.password,confirmation:'TERMINATE'});
  await expect(page).toHaveURL(/login/);expect(await pending()).toBe(1);
  await login(page,lang,account);
  await page.goto(`/${lang}/account/sudo`);await submit(page,'main form',{current_password:account.password});
  await page.goto(`/${lang}/account/privacy/delete`);
  await expect(page.locator('form[action$="/cancel"]')).toBeVisible();
  await submit(page,'main form',{current_password:account.password});
  expect(await pending()).toBe(0);
 }finally{await db.end();}
});


test('account lifecycle pages: shared shell, keyboard and accessibility at 360 pixels',async({page},info)=>{
 const lang=info.project.metadata.locale,account=await register(page,lang);await login(page,lang,account);
 await page.setViewportSize({width:360,height:900});
 for(const path of ['account/notifications','account/privacy/delete','account/security/email']){
  expect((await page.goto(`/${lang}/${path}`)).status()).toBe(200);
  await expect(page.locator('body.op-shell > .hx > header.hx-head')).toHaveCount(1);
  await expect(page.locator('body.op-shell > .hx > footer.hx-foot')).toHaveCount(1);
  expect(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth)).toBe(false);
  await page.keyboard.press('Tab');
  expect(await page.evaluate(()=>{const s=getComputedStyle(document.activeElement);return document.activeElement.matches(':focus-visible')&&(s.outlineStyle!=='none'||s.boxShadow!=='none');})).toBe(true);
  const axe=await new AxeBuilder({page}).analyze();expect(axe.violations.filter(v=>['serious','critical'].includes(v.impact))).toEqual([]);
 }
});
