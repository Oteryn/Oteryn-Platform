import {test, expect} from '../fixtures.mjs';
import {csrfRejected, submit} from '../helpers.mjs';
import {action, query, seededStaff, unique} from '../staff-rbac-helpers.mjs';
import {assertShell} from '../shell.mjs';

test('ops views preserve unavailable state and CMS lifecycle records staff decisions', async ({page}, info) => {
  test.setTimeout(420_000);
  const lang = info.project.metadata.locale;
  const staff = await seededStaff(page, lang, 'platform_admin');
  expect((await page.request.get('/console/dashboard?agent=missing-agent')).status()).toBe(403);
  await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='game_viewer'", [staff]);
  const localized = (pl, en) => lang === 'pl' ? pl : en;
  const views = [
    ['/console/dashboard', localized('Pulpit', 'Dashboard')],
    ['/console/worlds', localized('Światy', 'Worlds')],
    ['/console/nodes', localized('Węzły i hosty', 'Nodes and hosts')],
    ['/console/database', localized('Baza danych', 'Database')],
  ];
  for (const [route, title] of views) {
    const response = await page.goto(`${route}?lang=${lang}&agent=missing-agent`);
    expect(response.status()).toBe(200);
    expect(response.headers()['cache-control']).toContain('no-store');
    await expect(page.getByRole('heading', {level: 1})).toHaveText(title);
    await expect(page.locator('main')).toContainText(localized('Niedostępne', 'Unavailable'));
    await expect(page.locator('main form[method=post]')).toHaveCount(0);
    await assertShell(page);
  }
  await expect(page.getByRole('heading', {name: localized('Migracje', 'Migrations')})).toBeVisible();
  for (const [route, title] of [
    ['/console/worlds', localized('Szczegóły świata', 'World details')],
    ['/console/nodes', localized('Szczegóły węzła', 'Node details')],
  ]) {
    expect((await page.goto(`${route}?lang=${lang}&agent=missing-agent&target=missing-target`)).status()).toBe(200);
    await expect(page.getByRole('main').getByRole('heading', {level: 2})).toHaveText(title);
    await expect(page.locator('main')).toContainText(localized('Niedostępne', 'Unavailable'));
  }
  expect((await page.request.get('/console/worlds?agent=../invalid')).status()).toBe(400);
  expect((await page.request.get('/console/database?target=wrong-scope')).status()).toBe(400);
  expect((await page.goto(`/admin/console/access?lang=${lang}`)).status()).toBe(200);
  await expect(page.getByRole('heading', {name: localized('Polityka gry', 'Game policy')})).toBeVisible();
  await expect(page.getByRole('heading', {name: localized(
    'Połączeni agenci, czasy obserwacji i ogłaszane operacje',
    'Connected agents, observation times and advertised operations',
  )})).toBeVisible();

  expect((await page.goto('/admin/events/create')).status()).toBe(200);
  const csrf = await page.locator('main form input[name=_csrf]').inputValue();
  const consolePost = (route, form) => page.request.post(route, {
    form: {_csrf: csrf, lang, agent: 'missing-agent', ...form},
    headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
  });
  await csrfRejected(page, '/console/worlds', {operation: 'world.list', lang, agent: 'missing-agent'});
  expect((await consolePost('/console/worlds', {operation: 'node.list'})).status()).toBe(400);
  expect((await consolePost('/console/database', {operation: 'db.migrations.list'})).status()).toBe(503);
  expect(await query("SELECT request_id FROM admin_control_requests WHERE actor=$1 AND operation='db.migrations.list'", [staff])).toEqual([]);

  const minute = offset => new Date(Date.now() + offset * 60_000).toISOString().slice(0, 16);
  const slug = unique().replaceAll('_', '-');
  const draft = {
    starts_at: minute(60), ends_at: minute(120), reason: 'Create event announcement',
    en_title: 'World event announcement', en_slug: slug, en_summary: 'Editorial event summary.', en_body: 'Editorial event details.',
    pl_title: 'Zapowiedź wydarzenia', pl_slug: slug, pl_summary: 'Opis wydarzenia.', pl_body: 'Szczegóły wydarzenia.',
  };
  const post = (route, form) => page.request.post(route, {
    form, headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
  });
  expect((await post('/admin/events', {...draft, _csrf: csrf, reason: ' '})).status()).toBe(422);
  await submit(page, 'main form', draft);
  await expect(page).toHaveURL(/\/admin\/events\/[a-f0-9-]+\/edit$/);
  const edit = new URL(page.url()).pathname;
  const id = edit.split('/').at(-2);
  const link = `/${lang}/events/${slug}`;
  const statusForm = 'form[action$="/status"]';
  const setStatus = async (status, reason) => {
    await action(page, statusForm, {status, reason});
    await expect(page).toHaveURL(new RegExp(`${edit}$`));
  };
  const transition = async (status, reason) => {
    const form = await page.locator(statusForm).evaluate(node => Object.fromEntries(new FormData(node)));
    return post(`/admin/events/${id}/status`, {...form, status, reason});
  };
  expect((await transition('unrecognized', 'Invalid editorial status')).status()).toBe(422);
  expect((await transition('scheduled', ' ')).status()).toBe(422);
  await setStatus('scheduled', 'Schedule editorial event');
  expect((await page.request.get(link)).status()).toBe(404);
  await setStatus('published', 'Publish editorial event');
  expect((await page.request.get(link)).status()).toBe(200);
  await submit(page, 'main form:not([action$="/status"])', {
    ends_at: minute(180), reason: 'Extend editorial event window',
  });
  await setStatus('published', 'Publish updated event window');
  expect((await page.request.get(link)).status()).toBe(200);
  await setStatus('archived', 'Archive editorial event');
  expect((await page.request.get(link)).status()).toBe(404);

  const audits = await query(
    'SELECT actor, request_id, action, before, after FROM audit_log WHERE target=$1 ORDER BY created_at,id', [id],
  );
  expect(audits.map(row => [row.actor, row.before?.status ?? null, row.after.status, row.after.reason])).toEqual([
    [staff, null, 'draft', draft.reason],
    [staff, 'draft', 'scheduled', 'Schedule editorial event'],
    [staff, 'scheduled', 'published', 'Publish editorial event'],
    [staff, 'published', 'draft', 'Extend editorial event window'],
    [staff, 'draft', 'published', 'Publish updated event window'],
    [staff, 'published', 'archived', 'Archive editorial event'],
  ]);
  expect(audits[3].after.ends_at).toBeGreaterThan(audits[3].before.ends_at);
  const auditUrl = `/admin/console/audit?lang=${lang}&request_id=${audits[5].request_id}`;
  expect((await page.goto(auditUrl)).status()).toBe(200);
  await expect(page.locator('main')).toContainText('events.status_changed');
  await expect(page.locator('main')).toContainText(id);
  await expect(page.locator('main')).toContainText(localized('Niedostępne', 'Unavailable'));

  await page.goto(edit);
  await query("DELETE FROM account_roles WHERE account_id=$1 AND role_id=(SELECT id FROM roles WHERE key='platform_admin')", [staff]);
  expect((await transition('published', 'Unauthorized publication')).status()).toBe(403);
  expect((await page.request.get(`/admin/console/access?lang=${lang}`)).status()).toBe(403);
  expect((await page.request.get(link)).status()).toBe(404);
});
