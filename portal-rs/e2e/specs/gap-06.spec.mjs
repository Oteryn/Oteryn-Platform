import { test, expect } from '../fixtures.mjs';
import { csrfRejected, submit } from '../helpers.mjs';
import { query, seededStaff, unique } from '../staff-rbac-helpers.mjs';

test('staff publishes and resolves a world notice players can read', async ({ page }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const summary = {
    summary_en: 'Scheduled restart ' + unique(),
    summary_pl: 'Planowany restart ' + unique(),
  };
  const worldName = 'Status ' + unique();
  const [{ id }] = await query(
    'INSERT INTO world_status(id,name) VALUES(uuidv7(),$1) RETURNING id',
    [worldName],
  );
  const publicPath = `/${lang}/worlds/${id}/uptime`;
  const operatorPath = `/admin/world-status/${lang}/${id}`;
  const actor = await seededStaff(page, lang, 'platform_admin');

  expect((await page.goto(`/${lang}/worlds`)).status()).toBe(200);
  await page.getByRole('link', { name: new RegExp(worldName) }).click();
  await expect(page).toHaveURL(url => url.pathname === publicPath);
  await expect(page.getByRole('main')).toContainText(
    lang === 'en' ? 'Temporarily unavailable.' : 'Tymczasowo niedostępne.',
  );
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
    info.project.use.viewport.width,
  );
  await page.getByRole('link', {
    name: lang === 'en' ? 'Manage world incidents' : 'Zarządzaj incydentami świata',
  }).click();
  await expect(page).toHaveURL(url => url.pathname === operatorPath);

  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
    info.project.use.viewport.width,
  );
  await submit(page, `form[action="${operatorPath}"]`, summary);
  await expect(page.getByRole('heading', { name: summary[`summary_${lang}`] })).toBeVisible();
  const resolve = page.locator('form[action$="/resolve"]');
  const resolvePath = await resolve.getAttribute('action');
  await csrfRejected(page, resolvePath, {});
  await resolve.getByRole('button').click();
  await expect(resolve).toHaveCount(0);

  const response = await page.goto(publicPath);
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toContain('no-store');
  await expect(page.getByRole('heading', { name: summary[`summary_${lang}`] })).toBeVisible();
  await expect(page.getByRole('main')).toContainText(lang === 'en' ? 'Resolved' : 'Rozwiązano');
  await expect(page.locator('form[method=post]')).toHaveCount(0);
  expect(await query(
    'SELECT action FROM audit_log WHERE actor=$1 AND target=$2 ORDER BY created_at,id',
    [actor, resolvePath.split('/').at(-2)],
  )).toEqual([{ action: 'portal.world_incident.published' }, { action: 'portal.world_incident.resolved' }]);
});
