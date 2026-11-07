import {test,expect} from '@playwright/test';
import {pool} from '../helpers.mjs';

test('locked native export stays inactive and unsupported public sources refuse',async({page},info)=>{
 const lang=info.project.metadata.locale;
 for(const suffix of ['wiki','wiki/search','wiki/items','wiki/creatures','wiki/npcs','atlas','atlas/delivery']){
  const response=await page.goto(`/${lang}/${suffix}`);
  expect(response.status(),suffix).toBe(503);
  expect(response.headers()['cache-control']).toContain('no-store');
  await expect(page.locator('body.op-shell > header.op-shell__header')).toHaveCount(1);
  await expect(page.locator('body.op-shell > footer.op-shell__footer')).toHaveCount(1);
 }
 const data=await page.request.get(`/${lang}/today/data`);
 expect(data.status()).toBe(503);expect(data.headers()['cache-control']).toContain('no-store');expect(await data.json()).toEqual({status:'unavailable'});
 const widget=await page.request.get(`/${lang}/today/widget`);
 expect(widget.status()).toBe(503);expect(widget.headers()['cache-control']).toContain('no-store');
 // Widget is deliberately a fragment; the JSON and fragment routes do not use the page shell.
 expect(widget.headers()['content-type']).toContain('text/html');
 const client=pool();
 try{
  const rows=await client.query("SELECT status,artifact FROM world_catalog_candidates");
  expect(rows.rows).toHaveLength(1);expect(rows.rows[0].status).toBe('inactive');expect(JSON.parse(rows.rows[0].artifact.toString('utf8')).entities).toEqual([]);
  const active=await client.query('SELECT version_id FROM world_active_version WHERE singleton');expect(active.rows[0].version_id).toBeNull();
  const visibility=await client.query('SELECT count(*)::int AS count FROM world_visibility');expect(visibility.rows[0].count).toBe(0);
 }finally{await client.end();}
});

test('world statistics opens a real registry picker and preserves the privacy gate',async({page},info)=>{
 const lang=info.project.metadata.locale;
 const response=await page.goto(`/${lang}/stats/world`);
 expect(response.status()).toBe(200);expect(response.headers()['cache-control']).toContain('no-store');
 await expect(page.locator('body.op-shell > header.op-shell__header')).toHaveCount(1);
 const select=page.locator('select[name=world]');await expect(select).toBeVisible();
 expect(await select.locator('option').count()).toBeGreaterThan(0);
 await expect(select).toContainText('Local Bazaar World');
 const world=await select.inputValue();expect(world).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
 await page.locator('input[name=start]').fill('2026-11-01');await page.locator('input[name=end]').fill('2026-11-02');
 const [refusal]=await Promise.all([page.waitForResponse(r=>r.url().includes('/stats/world?')&&r.request().isNavigationRequest()),page.locator('button[type=submit]').click()]);
 expect(refusal.status()).toBe(503);expect(refusal.headers()['cache-control']).toContain('no-store');
 await expect(page.locator('body.op-shell > header.op-shell__header')).toHaveCount(1);
});
