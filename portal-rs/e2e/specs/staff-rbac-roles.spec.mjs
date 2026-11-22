import {test,expect} from '../fixtures.mjs';
import {isolatedAdmin,confirmStaffPassword,player,role,membership,audit,enrollMfa,loginWithRecoveryCode,expectLoginRedirect} from '../staff-rbac-helpers.mjs';
test('role grant enables only permitted pages and revocation refuses the next request',async({page,browser},info)=>{
 test.setTimeout(300000);const lang=info.project.metadata.locale;const account=await player(page,lang);const s=await isolatedAdmin(browser,info);
 try{
  const codes=await enrollMfa(page,lang,account);
  await loginWithRecoveryCode(page,lang,account,codes[0]);
  const id=await role(s.page,'admin.access audit.view');
  await page.goto(`/${lang}/`);const ordinaryToken=await page.locator('input[name=_csrf]').first().inputValue();
  expect((await page.request.post('/admin/roles',{form:{_csrf:ordinaryToken,account:account.id,role:id,action:'grant'},headers:{origin:new URL(page.url()).origin},maxRedirects:0})).status(),'ordinary player cannot grant a role').toBe(403);
  await confirmStaffPassword(s.page,lang,s.account.password);
  await membership(s.page,account.id,id,'grant');
  await expectLoginRedirect(page,'/admin/console/audit');
  await loginWithRecoveryCode(page,lang,account,codes[1]);
  expect((await page.goto('/admin/console/audit')).status(),'granted audit permission allows read').toBe(200);
  expect((await page.goto('/admin/roles')).status(),'narrow role cannot manage roles').toBe(403);
  await page.goto(`/${lang}/`);const token=await page.locator('input[name=_csrf]').first().inputValue();
  expect((await page.request.post('/admin/roles',{form:{_csrf:token,account:account.id,role:id,action:'grant'},headers:{origin:new URL(page.url()).origin},maxRedirects:0})).status(),'non-manager grant refused with valid CSRF').toBe(403);
  await membership(s.page,account.id,id,'revoke');
  await expectLoginRedirect(page,'/admin/console/audit');
  await audit(s.page,'rbac.role.created',id);await audit(s.page,'rbac.role.granted',account.id);await audit(s.page,'rbac.role.revoked',account.id);
 }finally{await s.context.close();}
});
