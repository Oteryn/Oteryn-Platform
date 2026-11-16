import {test,expect} from '../fixtures.mjs';
import {submit,login} from '../helpers.mjs';
import * as OTPAuth from 'otpauth';
import {staff,player,role,membership,audit} from '../staff-rbac-helpers.mjs';
test('role grant enables only permitted pages and revocation refuses the next request',async({page,browser},info)=>{
 test.setTimeout(300000);const lang=info.project.metadata.locale;const account=await player(page,lang);const s=await staff(browser,info);
 try{
  // Staff access requires real enrollment and a satisfied MFA challenge even for narrow roles.
  await page.goto(`/${lang}/mfa`);await submit(page,'form[action$="/mfa/enroll"]',{password:account.password});
  const totp=new OTPAuth.TOTP({secret:await page.locator('main .op-prose').innerText(),algorithm:'SHA1',digits:6,period:30});
  await submit(page,'form[action$="/mfa/confirm"]',{password:account.password,code:totp.generate()});
  const codes=await page.locator('textarea[readonly]').allTextContents();
  // MFA confirmation revokes existing sessions; authenticate again before using CSRF.
  await login(page,lang,account);await expect(page).toHaveURL(/mfa\/challenge/);await submit(page,'form[action$="/mfa/challenge"]',{code:codes[0]});
  const id=await role(s.page,'admin.access audit.view');
  await page.goto(`/${lang}/`);const ordinaryToken=await page.locator('input[name=_csrf]').first().inputValue();
  expect((await page.request.post('/admin/roles',{form:{_csrf:ordinaryToken,account:account.id,role:id,action:'grant'},headers:{origin:new URL(page.url()).origin},maxRedirects:0})).status(),'ordinary player cannot grant a role').toBe(403);
  await membership(s.page,account.id,id,'grant');
  expect((await page.goto('/admin/console/audit')).status(),'granted audit permission allows read').toBe(200);
  expect((await page.goto('/admin/roles')).status(),'narrow role cannot manage roles').toBe(403);
  await page.goto(`/${lang}/`);const token=await page.locator('input[name=_csrf]').first().inputValue();
  expect((await page.request.post('/admin/roles',{form:{_csrf:token,account:account.id,role:id,action:'grant'},headers:{origin:new URL(page.url()).origin},maxRedirects:0})).status(),'non-manager grant refused with valid CSRF').toBe(403);
  await membership(s.page,account.id,id,'revoke');
  expect((await page.request.get('/admin/console/audit',{maxRedirects:0})).status(),'next request loses audit access').toBe(403);
  await audit(s.page,'rbac.role.created',id);await audit(s.page,'rbac.role.granted',account.id);await audit(s.page,'rbac.role.revoked',account.id);
 }finally{await s.context.close();}
});
