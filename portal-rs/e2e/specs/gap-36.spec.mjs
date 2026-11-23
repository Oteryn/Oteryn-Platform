import {randomUUID} from 'node:crypto';
import {test, expect} from '../fixtures.mjs';
import {messages} from '../public-depth-helpers.mjs';
import {query, seededAccount, seededLogin} from '../staff-rbac-helpers.mjs';

const watches = '/api/v1/stats/watches';

async function fitsViewport(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
}

test('a player opens statistics availability and submits a guild comparison', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const t = messages('templates/stats/readiness', lang);
  const guild = messages('templates/stats/guilds', lang);
  const status = `/${lang}/stats`;
  expect((await page.goto(`/${lang}/worlds`)).status()).toBe(200);
  if (info.project.use.viewport.width <= 960) {
    await page.locator('details.om-fallback-menu > summary').click();
  }
  const panel = page.locator('#portal-world-panel');
  const link = panel.locator(`a[href="${status}"]`);
  await panel.locator('..').locator(':scope > summary').click();
  await link.click();
  await expect(page.getByRole('heading', {name: t.title})).toBeVisible();
  await fitsViewport(page);
  await page.getByRole('link', {name: t.guilds}).click();
  await page.getByLabel(guild['guild-guild']).fill('First guild');
  await page.getByLabel(guild['guild-compare']).fill('Second guild');
  await page.locator('main input[name=days]').fill('31');
  const [result] = await Promise.all([
    page.waitForResponse(response => response.request().isNavigationRequest() && response.url().includes('/stats/guilds?')),
    page.getByRole('button', {name: guild['guild-submit']}).click(),
  ]);
  expect(result.status()).toBe(503);
  expect(result.headers()['cache-control']).toContain('no-store');
  await expect(page.locator('main [role=status]')).toHaveText(guild['guild-unavailable']);
  await expect(page.getByLabel(guild['guild-guild'])).toHaveValue('First guild');
  await expect(page.getByLabel(guild['guild-compare'])).toHaveValue('Second guild');
  await fitsViewport(page);
  expect((await page.request.get(`/${lang}/stats/rankings/unknown`)).status()).toBe(404);
});

test('a verified player uses and revokes session watch API access', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const t = messages('templates/stats/readiness', lang);
  const actor = await seededAccount(lang);
  await seededLogin(page, lang, actor);
  await page.goto(`/${lang}/stats`);
  await page.locator('main').getByRole('link', {name: t.access}).click();
  const settings = `/${lang}/account/stats-watches`;
  expect((await page.request.post(settings, {form: {action: 'issue'}})).status()).toBe(403);
  await expect(page.locator('main [role=status]')).toHaveText(t.inactive);
  await page.getByRole('button', {name: t.issue}).click();
  const token = await page.locator('#watch-token').inputValue();
  expect(/^[a-f0-9]{64}$/.test(token)).toBe(true);
  await fitsViewport(page);
  const headers = {authorization: `Bearer ${token}`};
  expect((await page.request.get(watches)).status()).toBe(401);
  expect((await page.request.get(watches, {headers})).status()).toBe(503);
  const target = randomUUID();
  const data = {target_kind: 'guild', target_id: target, event_kind: 'guild_change'};
  await query("INSERT INTO stats_watches(account_id,target_kind,target_id,event_kind) VALUES($1,'guild',$2,'guild_change')", [actor.id, target]);
  expect((await page.request.post(watches, {headers, data})).status()).toBe(503);
  expect((await page.request.delete(watches, {headers, data})).status()).toBe(204);
  const remaining = await query('SELECT account_id FROM stats_watches WHERE account_id=$1 AND target_id=$2', [actor.id, target]);
  expect(remaining).toEqual([]);
  await page.getByRole('button', {name: t.revoke}).click();
  await expect(page.locator('main [role=status]')).toHaveText(t.inactive);
  expect((await page.request.get(watches, {headers})).status()).toBe(401);
});
