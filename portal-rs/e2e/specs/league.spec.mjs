import {test,expect} from '@playwright/test';
const control=process.env.PORTAL_E2E_LEAGUE_CONTROL;
test.beforeEach(async({request})=>{expect(control,'fixture producer must be running').toBeTruthy();expect((await request.post(`${control}/resume`)).ok()).toBeTruthy();});
test('league: Game push -> worker -> public decided outcomes and complete standings',async({page},info)=>{
 const lang=info.project.metadata.locale;
 await expect.poll(async()=>{const r=await page.request.get(`/${lang}/league`);return r.status();},{timeout:45000}).toBe(200);
 const response=await page.goto(`/${lang}/league`);expect(response.status()).toBe(200);expect(response.headers()['cache-control']).toContain('no-store');
 await expect(page.locator('html')).toHaveAttribute('lang',lang);
 await expect(page.getByRole('cell',{name:'Aldric Guild',exact:true})).toBeVisible();
 await expect(page.getByRole('cell',{name:'Second Guild',exact:true})).toBeVisible();
 await expect(page.locator('tbody tr')).toHaveCount(2);
 await expect(page.locator('main')).toContainText('timed-out');
 await expect(page.locator('form')).toHaveCount(0);
 expect(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth)).toBe(false);
});
test('league: invalid signature refused, then stale Game cut makes user path unavailable',async({page,request},info)=>{
 const lang=info.project.metadata.locale;
 await expect.poll(async()=>(await request.get(`/${lang}/league`)).status(),{timeout:45000}).toBe(200);
 try{
  await expect(request.post(`${process.env.OTERYN_LEAGUE_BASE_URL}/internal/v1/game-public/league-watermark`,{data:{}})).rejects.toThrow();
  const reply=await request.post(`${control}/bad-signature`);expect((await reply.json()).refusalStatus).toBe(401);
  await expect.poll(async()=>(await request.get(`/${lang}/league`)).status(),{timeout:45000,intervals:[1000,2000]}).toBe(503);
  const response=await page.goto(`/${lang}/league`);expect(response.status()).toBe(503);expect(response.headers()['cache-control']).toContain('no-store');
  await expect(page.locator('html')).toHaveAttribute('lang',lang);
  await expect(page.locator('tbody tr')).toHaveCount(0);
  await expect(page.locator('form')).toHaveCount(0);
 }finally{await request.post(`${control}/resume`);}
});
