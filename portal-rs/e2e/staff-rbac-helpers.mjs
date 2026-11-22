import {expect} from '@playwright/test';
import * as OTPAuth from 'otpauth';
import path from 'node:path';
import crypto from 'node:crypto';
import {pool,csrfRejected,submit,register,login} from './helpers.mjs';
export const unique=()=>`staff_rbac_${crypto.randomBytes(10).toString('hex')}`;
export async function staff(browser,info){const context=await browser.newContext({storageState:path.join(process.env.PORTAL_E2E_AUTH_DIRECTORY,`staff-${info.project.name}.json`),baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443',ignoreHTTPSErrors:true,locale:info.project.metadata.locale});return {context,page:await context.newPage()};}
export async function query(sql,args=[],databaseURL){const db=pool(databaseURL);try{return (await db.query(sql,args)).rows;}finally{await db.end();}}
export async function player(page,lang){const account=await register(page,lang);await login(page,lang,account);return {...account,id:(await query('SELECT id FROM accounts WHERE email=$1',[account.email]))[0].id};}
// roles.html has select controls and named submit buttons; submit() cannot fill selects.
// support_web.rs renders hidden action controls and select controls in SSR forms.
export async function action(page,selector,values={},button){const form=page.locator(selector).first();const url=await form.getAttribute('action');const fields=await form.evaluate(f=>Object.fromEntries(new FormData(f)));await csrfRejected(page,url,{...fields,...values});for(const [name,value] of Object.entries(values)){const field=form.locator(`[name="${name}"]`);if(await field.evaluate(e=>e.tagName)==='SELECT')await field.selectOption(value);else await field.fill(value);}await form.locator(button?`button[value="${button}"]`:'button').first().click();}
export async function role(page,permissions=''){const key=unique();await page.goto('/admin/roles');await submit(page,'form:has(input[name=key])',{key,permissions});const id=(await query('SELECT id FROM roles WHERE key=$1',[key]))[0].id;await rolePage(page,id);await expect(page.locator('main'),'created role is visible').toContainText(key);return id;}
export async function membership(page,account,role,op){await rolePage(page,role);await action(page,'form:has(input[name=account])',{account,role,...(op==='grant'?{reason:'Assign role for staff workflow'}:{})},op);await expect.poll(async()=>Number((await query('SELECT count(*) n FROM account_roles WHERE account_id=$1 AND role_id=$2',[account,role]))[0].n),{message:`${op} persists`}).toBe(op==='grant'?1:0);}
export async function audit(page,operation,target){const rows=await query('SELECT actor,request_id FROM audit_log WHERE action=$1 AND target=$2 ORDER BY created_at DESC',[operation,target]);expect(rows.length,`${operation} emits audit evidence`).toBeGreaterThan(0);for(const row of rows){const actor=row.actor;expect((await page.goto(`/admin/console/audit?${new URLSearchParams({admin:actor,operation,target,request_id:row.request_id})}`)).status(),'audit console opens').toBe(200);const main=page.locator('main');await expect(main,'audit exposes action').toContainText(operation);await expect(main,'audit exposes actor').toContainText(actor);await expect(main,'audit exposes target').toContainText(target);}}

export async function rolePage(page,id){await page.goto('/admin/roles');for(let n=0;n<10000;n++){if(await page.locator(`select[name=role] option[value="${id}"]`).count())return;const next=page.locator('a[href^="/admin/roles?after="]');expect(await next.count(),'role remains discoverable through keyset pagination').toBe(1);await next.click();}throw new Error('role pagination exceeded bound');}

export async function recoveryCodes(page){
 const codes=page.locator('main textarea[readonly]');
 await expect(codes.first()).toBeVisible();
 return codes.allTextContents();
}

export async function enrollMfa(page,lang,account){
 await page.goto(`/${lang}/mfa`);
 await submit(page,'form[action$="/mfa/enroll"]',{password:account.password});
 const totp=new OTPAuth.TOTP({secret:await page.locator('main .op-prose').innerText(),algorithm:'SHA1',digits:6,period:30});
 await submit(page,'form[action$="/mfa/confirm"]',{password:account.password,code:totp.generate()});
 return recoveryCodes(page);
}

export async function loginWithRecoveryCode(page,lang,account,code){
 await login(page,lang,account);
 await expect(page).toHaveURL(/mfa\/challenge/);
 await submit(page,'form[action$="/mfa/challenge"]',{code});
}

export async function expectLoginRedirect(page,path){
 const response=await page.request.get(path,{maxRedirects:0});
 expect(response.status(),`revoked session accessing ${path}`).toBe(303);
 expect(response.headers().location).toBe('/login');
}

export async function isolatedAdmin(browser,info){
 const lang=info.project.metadata.locale;
 const context=await browser.newContext({baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443',ignoreHTTPSErrors:true,viewport:info.project.use.viewport,locale:lang});
 try{
  const page=await context.newPage();
  const account=await register(page,lang);
  await query("INSERT INTO account_roles(account_id,role_id) SELECT a.id,r.id FROM accounts a CROSS JOIN roles r WHERE a.email=$1 AND r.key='platform_admin'",[account.email]);
  await login(page,lang,account);
  const codes=await enrollMfa(page,lang,account);
  await loginWithRecoveryCode(page,lang,account,codes[0]);
  return {context,page,account,password:account.password,codes};
 }catch(error){await context.close();throw error;}
}
