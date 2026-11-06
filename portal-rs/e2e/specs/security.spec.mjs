import {test,expect} from '@playwright/test';
import {register,login,submit,password,csrfRejected,waitForLoginBudget} from '../helpers.mjs';
import crypto from 'node:crypto';

test('privacy changes persist and account export contains the owning account',async({page},info)=>{
 const lang=info.project.metadata.locale;const account=await register(page,lang);await login(page,lang,account);
 await page.goto(`/${lang}/account/privacy`);
 await page.locator('[name=locale]').selectOption(lang);
 await submit(page,'main form',{public_account_association:true,public_status_visible:true});
 await page.reload();
 await expect(page.locator('[name=public_account_association]')).toBeChecked();
 await expect(page.locator('[name=public_status_visible]')).toBeChecked();
 await page.goto(`/${lang}/account/sudo`);
 await submit(page,'main form',{current_password:account.password});
 const exported=await page.request.get(`/${lang}/account/privacy/export`);
 expect(exported.status()).toBe(200);
 expect(exported.headers()['cache-control']).toContain('no-store');
 expect(await exported.text()).toContain(account.email);
});

test('sessions: revoke other browser sessions and refuse the revoked cookie',async({page,browser},info)=>{
 const lang=info.project.metadata.locale;const account=await register(page,lang);await login(page,lang,account);
 const context=await browser.newContext({ignoreHTTPSErrors:true,viewport:info.project.use.viewport,locale:info.project.metadata.locale,baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443'});
 const other=await context.newPage();await login(other,lang,account);
 await page.goto(`/${lang}/account/sessions`);
 await submit(page,'main form[action$="/others"]');
 await other.goto('/account');
 await expect(other).toHaveURL(/login/);
 await context.close();
});

test('password change requires sudo, revokes sessions and accepts the new password',async({page},info)=>{
 const lang=info.project.metadata.locale;const account=await register(page,lang);await login(page,lang,account);
 await page.goto(`/${lang}/account/sudo`);
 await submit(page,'main form',{current_password:account.password});
 await page.goto(`/${lang}/password/change`);
 const next=password();
 await submit(page,'main form',{current_password:account.password,password:next,password_confirmation:next});
 await expect(page).toHaveURL(/login/);
 await login(page,lang,{...account,password:next});
 await expect(page).not.toHaveURL(/login/);
});

test('login rate limit refuses the sixth wrong password and supplies Retry-After',async({page},info)=>{
 const lang=info.project.metadata.locale;
 await page.goto(`/${lang}/login`);
 await csrfRejected(page,`/${lang}/login`);
 const token=await page.locator('[name=_csrf]').inputValue();
 const email=`limit-${crypto.randomUUID()}@local.test`;
 await waitForLoginBudget(6);
 const statuses=[];let last;
 for(let i=0;i<6;i++){
   last=await page.request.post(`/${lang}/login`,{form:{_csrf:token,email,password:'Wrong local password!'},headers:{origin:new URL(page.url()).origin},maxRedirects:0});
   statuses.push(last.status());
 }
 expect(statuses).toEqual([302,302,302,302,302,429]);
 expect(Number(last.headers()['retry-after'])).toBeGreaterThan(0);
});
