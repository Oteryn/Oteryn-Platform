import {test,expect} from '../fixtures.mjs';
import crypto from 'node:crypto';
import {register,login,pool,csrfRejected} from '../helpers.mjs';
import {token,uuid7} from '../staff-commerce-helpers.mjs';
import {isolatedAdmin} from '../staff-rbac-helpers.mjs';

test('single-use gift code credits once and refuses both repeat and foreign redemption',async({page,browser},info)=>{
 const lang=info.project.metadata.locale;
 const account=await register(page,lang);await login(page,lang,account);
 const {context:staff,page:admin}=await isolatedAdmin(browser,info);
 const db=pool();
 const otherContext=await browser.newContext({baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443',ignoreHTTPSErrors:true});
 try {
  expect((await admin.goto('/admin/commerce')).status()).toBe(200);
  const csrf=await admin.locator('[name="_csrf"]').first().inputValue();
  const action=`/${lang}/admin/store/codes`;
  const data={request_id:uuid7(),kind:'coins',amount:37,count:1,max_per_code:1,max_per_account:1,valid_from:Math.floor(Date.now()/1000)-60,valid_until:Math.floor(Date.now()/1000)+3600,note:`Browser ${crypto.randomUUID()}`};
  await csrfRejected(admin,action);
  const generated=await admin.request.post(action,{data,headers:{'x-csrf-token':csrf}});
  expect(generated.status(),'staff generates gift code').toBe(200);
  const batch=await generated.json();expect(batch.codes).toHaveLength(1);
  const redeem=`/${lang}/shop/codes/redeem`;
  const redeemAs=async(screen,code)=>screen.request.post(redeem,{data:{code,operation:uuid7()},headers:{'x-csrf-token':await token(screen,lang)}});
  expect((await redeemAs(page,batch.codes[0])).status(),'first redemption succeeds').toBe(200);
  await page.goto(`/${lang}/shop/history`);await expect(page.locator('main dl dd').first(),'gift credit visible').toHaveText('37');
  expect((await redeemAs(page,batch.codes[0])).status(),'second redemption refused').toBe(403);
  const otherPage=await otherContext.newPage();
  const other=await register(otherPage,lang);await login(otherPage,lang,other);
  expect((await redeemAs(otherPage,batch.codes[0])).status(),'single-use code exhausted for another account').toBe(403);
  expect((await db.query('SELECT count(*)::int n FROM code_redemptions r JOIN gift_codes c ON c.id=r.code_id WHERE c.batch_id=$1',[batch.batch_id])).rows[0].n).toBe(1);
 } finally {await otherContext.close();await staff.close();await db.end();}
});
