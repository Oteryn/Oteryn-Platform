import {test,expect} from '../fixtures.mjs';
import {login} from '../helpers.mjs';
import {staff,player,action,audit,unique} from '../staff-rbac-helpers.mjs';
test('player search and force logout revoke the player session and allow fresh login',async({page,browser},info)=>{
 test.setTimeout(180000);const lang=info.project.metadata.locale;const account=await player(page,lang);const s=await staff(browser,info);
 try{
  expect((await s.page.goto('/admin/players')).status()).toBe(200);
  await s.page.locator('[name=q]').fill(account.email);await s.page.locator('form[method=get] button').click();
  await s.page.locator(`a[href="/admin/players/${account.id}"]`).click();
  await expect(s.page.locator('main'),'player detail identifies target').toContainText(account.id);
  await action(s.page,'form:has(button[value=logout])',{reason:unique()},'logout');
  await expect.poll(async()=>{await page.goto(`/${lang}/account/sessions`);return new URL(page.url()).pathname;},{message:'old player session is refused'}).toContain('/login');
  await login(page,lang,account);await page.goto(`/${lang}/account/sessions`);await expect(page,'fresh login restores player account access').not.toHaveURL(/\/login/);
  await audit(s.page,'players.logout_all',account.id);
 }finally{await s.context.close();}
});
