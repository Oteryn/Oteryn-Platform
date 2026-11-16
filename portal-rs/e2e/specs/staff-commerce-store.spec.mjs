import {test,expect} from '../fixtures.mjs';
import {register,login,pool,csrfRejected} from '../helpers.mjs';
import {paidTopup,uuid7} from '../staff-commerce-helpers.mjs';

test('bank-funded store reports unavailable Game catalogue without claiming delivery',async({page},info)=>{
 const lang=info.project.metadata.locale,db=pool();
 const account=await register(page,lang);await login(page,lang,account);
 try {
  const topup=await paidTopup(page,lang,db);
  await page.goto(`/${lang}/shop/history`);
  await expect(page.locator('main dl dd').first(),'bank-funded balance visible').toHaveText(topup.coins);
  expect((await page.goto(`/${lang}/shop`)).status()).toBe(200);
  // local.mjs has no Store catalogue/ownership producer. Bazaar, league and closure
  // producers cannot establish a Store offer. Never seed Game catalogue or ownership.
  // store/src/web.rs offers() renders this exact documented state for catalog=None.
  await expect(page.locator('main [role=status]'),'missing Game catalogue stays explicitly unavailable').toHaveText(lang==='en'?'This operation is unavailable. No success has been confirmed.':'Operacja jest niedostępna. Powodzenie nie zostało potwierdzone.');
  await expect(page.locator('main form[action$="/shop"]'),'no fabricated purchasable offer').toHaveCount(0);
  const fields={operation:uuid7(),character:uuid7(),world:uuid7(),revision:'missing',offer:'missing',price:'1',quantity:'1'};
  await csrfRejected(page,`/${lang}/shop`,fields);
  await page.goto(`/${lang}/shop/history`);
  await expect(page.locator('main dl dd').first(),'unavailable purchase leaves funded balance intact').toHaveText(topup.coins);
 } finally {await db.end();}
});
