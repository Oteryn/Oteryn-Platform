import {test,expect} from '../fixtures.mjs';
import {register,login,submit,pool} from '../helpers.mjs';

test('referral link binds a newly registered invitee; self-referral refused',async({page,browser},info)=>{
 const lang=info.project.metadata.locale,db=pool();
 const a=await register(page,lang);await login(page,lang,a);
 const guest=await browser.newContext({baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443',ignoreHTTPSErrors:true});
 try {
  // Inline Askama Page and routes in community/src/referrals/web.rs.
  await page.goto(`/${lang}/account/referrals`);
  const link=await page.locator('#invite-link').inputValue();
  await page.goto(link);
  const csrf=await page.locator('main [name="_csrf"]').inputValue();
  const denied=await page.request.post(link,{form:{_csrf:csrf},maxRedirects:0});
  expect(denied.status(),'inviter cannot accept own link').toBe(403);
  const bpage=await guest.newPage();await bpage.goto(link);await submit(bpage);
  await expect(bpage,'anonymous referral accepts into registration').toHaveURL(new RegExp(`/${lang}/register$`));
  const b=await register(bpage,lang);await login(bpage,lang,b);
  await expect.poll(async()=>(await db.query('SELECT i.kind FROM referral_campaign_invitations i JOIN accounts a ON a.id=i.inviter_id JOIN accounts b ON b.id=i.invitee_id WHERE a.email=$1 AND b.email=$2',[a.email,b.email])).rows,{message:'registration binds referral',timeout:30000}).toEqual([{kind:'new'}]);
  await page.goto(`/${lang}/account/referrals`);
  await expect(page.locator('section[aria-labelledby="invitations"] li'),'inviter sees referral').toHaveCount(1);
  // Campaign qualification rewards require first paid activity; signup alone does not earn them.
  expect((await db.query('SELECT count(*)::int n FROM referral_campaign_invitations i JOIN accounts a ON a.id=i.inviter_id WHERE a.email=$1 AND i.invitee_id=i.inviter_id',[a.email])).rows[0].n).toBe(0);
 } finally {await guest.close();await db.end();}
});
