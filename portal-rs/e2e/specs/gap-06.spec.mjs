import { readFileSync } from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, submit } from '../helpers.mjs';
import { query, seededStaff, unique } from '../staff-rbac-helpers.mjs';

test('staff publishes and resolves a public world notice', async ({ page, context }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const summary = {
    summary_en: 'Scheduled restart '.padEnd(240, '中'),
    summary_pl: 'Planowany restart '.padEnd(240, '界'),
  };
  const worldName = 'Status ' + unique();
  const [{ id }] = await query(
    'INSERT INTO world_status(id,name) VALUES(uuidv7(),$1) RETURNING id',
    [worldName],
  );
  const publicPath = `/${lang}/worlds/${id}/uptime`;
  const operatorPath = `/admin/world-status/${lang}/${id}`;

  expect((await page.goto(`/${lang}/worlds`)).status()).toBe(200);
  await page.getByRole('link', { name: new RegExp(worldName) }).click();
  await expect(page).toHaveURL(url => url.pathname === publicPath);
  await expect(page.getByRole('main')).toContainText(
    lang === 'en' ? 'Temporarily unavailable.' : 'Tymczasowo niedostępne.',
  );
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
    info.project.use.viewport.width,
  );

  const anonymous = await page.request.get(operatorPath, { maxRedirects: 0 });
  expect(anonymous.status()).toBe(303);
  expect(anonymous.headers().location).toBe('/login');
  const actor = await seededStaff(page, lang, 'platform_admin');
  expect((await page.goto(publicPath)).status()).toBe(200);

  await page.getByRole('link', {
    name: lang === 'en' ? 'Manage world incidents' : 'Zarządzaj incydentami świata',
  }).click();
  await expect(page).toHaveURL(url => url.pathname === operatorPath);

  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
    info.project.use.viewport.width,
  );
  const publishSelector = `form[action="${operatorPath}"]`;
  const csrf = await page.locator(`${publishSelector} input[name=_csrf]`).inputValue();
  const invalid = await page.request.post(operatorPath, {
    form: { ...summary, summary_pl: ' ', _csrf: csrf },
    headers: { origin: new URL(page.url()).origin },
    maxRedirects: 0,
  });
  expect(invalid.status()).toBe(400);
  expect(await query('SELECT id FROM world_incidents WHERE world_id=$1', [id])).toEqual([]);

  await submit(page, publishSelector, summary);
  await expect(page.getByRole('heading', { name: summary[`summary_${lang}`] })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
    info.project.use.viewport.width,
  );
  const resolve = page.locator('form[action$="/resolve"]');
  const resolvePath = await resolve.getAttribute('action');
  await csrfRejected(page, resolvePath, {});
  await resolve.getByRole('button').click();
  await expect(resolve).toHaveCount(0);

  await context.clearCookies();
  const response = await page.goto(publicPath);
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toContain('no-store');
  const main = page.getByRole('main');
  await expect(main.getByRole('heading', { name: summary[`summary_${lang}`] })).toBeVisible();
  await expect(main).toContainText(lang === 'en' ? 'Resolved' : 'Rozwiązano');
  await expect(main.locator('form[method=post]')).toHaveCount(0);
  expect(await query(
    'SELECT action FROM audit_log WHERE actor=$1 AND target=$2 ORDER BY created_at,id',
    [actor, resolvePath.split('/').at(-2)],
  )).toEqual([{ action: 'portal.world_incident.published' }, { action: 'portal.world_incident.resolved' }]);
});

test('i18n-parity for world uptime and incidents', () => {
  const template = readFileSync(
    new URL('../../templates/community/world_uptime.html', import.meta.url), 'utf8',
  );
  const keys = [...new Set([...template.matchAll(/language\.text\("([^"]+)"\)/g)]
    .map(([, key]) => key))];
  expect(keys).toContain('world-uptime-title');
  for (const lang of ['en', 'pl']) {
    const catalog = readFileSync(
      new URL(`../../crates/ui/i18n/${lang}.ftl`, import.meta.url), 'utf8',
    );
    const messages = new Map([...catalog.matchAll(/^([\w-]+) *= *(\S.*)$/gm)]
      .map(([, key, value]) => [key, value]));
    for (const key of keys) {
      expect(messages.get(key), `${lang} translation for ${key}`).toBeTruthy();
    }
    expect([...messages.keys()].filter(key => key.startsWith('world-uptime-')).sort())
      .toEqual(keys.filter(key => key.startsWith('world-uptime-')).sort());
  }
});
