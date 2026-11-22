import {adminControlAgent, advertisedOperations, agentId, nodeId, worldId} from '../support/admin-control-fixture.mjs';
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

test('ratified Game agent results render in every ops read view', async ({page}, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const actor = await seededStaff(page, lang, 'platform_admin');
  await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='game_viewer'", [actor]);
  const agent = adminControlAgent();
  const answer = async (operation, target, requestId) => {
    const requests = await agent.poll();
    expect(requests.length, `claimed ${operation} through mTLS`).toBe(1);
    const [request] = requests;
    expect(request.operation).toBe(operation);
    expect(request.operation_version).toBe(1);
    expect(request.kind).toBe('query');
    expect(request.actor.admin_id).toBe(actor);
    expect(request.target).toEqual({
      world_id: target === worldId ? worldId : null,
      node_id: target === worldId ? null : nodeId,
      character_id: null,
      account_ref: null,
    });
    if (requestId) expect(request.request_id).toBe(requestId);
    expect(agent.authentic(request), 'Platform signed the claimed request').toBe(true);
    const detailField = {'world.get': 'world_id', 'node.get': 'node_id'}[operation];
    if (detailField) {
      await expect(agent.complete(request, {[detailField]: 'different-scope'}))
        .rejects.toThrow('Admin control results returned HTTP 400');
      expect(await query('SELECT request_id FROM admin_control_results WHERE request_id=$1', [requestId]))
        .toEqual([]);
    }
    await agent.complete(request);
  };

  await agent.connect();
  try {
    for (const [route, operation, target, expected, heading] of [
      ['/console/dashboard', 'ops.overview.get', null, '137', lang === 'pl' ? 'Przegląd' : 'Overview'],
      ['/console/worlds', 'world.list', null, `${worldId} · Verified frontier`],
      ['/console/worlds', 'world.get', worldId, `${worldId} · Verified frontier`],
      ['/console/nodes', 'node.list', null, 'verified-build-42'],
      ['/console/nodes', 'node.get', nodeId, 'verified-build-42'],
      ['/console/database', 'db.status', null, 'verified-runtime: 7', lang === 'pl' ? 'Baza danych' : 'Database'],
      ['/console/database', 'db.migrations.list', null, '20261122004200'],
    ]) {
      await agent.heartbeat();
      const params = new URLSearchParams({lang, agent: agentId});
      if (target) params.set('target', target);
      expect((await page.goto(`${route}?${params}`)).status()).toBe(200);
      const form = page.locator(`main form:has(input[name=operation][value="${operation}"])`);
      const fields = await form.evaluate(node => Object.fromEntries(new FormData(node)));
      await csrfRejected(page, route, fields);
      const queued = await page.request.post(route, {
        form: fields, headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
      });
      expect(queued.status()).toBe(303);
      const resultUrl = queued.headers().location;
      const requestId = new URL(resultUrl, page.url()).searchParams.get('request');
      expect(requestId).toMatch(/^[a-f0-9-]{36}$/);
      await answer(operation, target, requestId);
      expect((await page.goto(resultUrl)).status()).toBe(200);
      const record = page.locator('main section').filter({
        has: page.getByText(expected, {exact: true}),
      });
      await expect(record).toHaveCount(1);
      await expect(record.getByText(expected, {exact: true})).toBeVisible();
      await expect(record.getByText('verified-fixture-revision', {exact: true})).toBeVisible();
      if (heading) await expect(record.getByRole('heading', {level: 3, name: heading, exact: true})).toBeVisible();
    }

    for (const [route, operation, expected] of [
      ['/admin/console/audit', 'admin.ledger.query', 'verified-fixture-ledger'],
      ['/admin/console/access', 'admin.policy.get', 'world.get · R0 · game.world.read'],
    ]) {
      await agent.heartbeat();
      const url = `${route}?lang=${lang}`;
      expect((await page.goto(url)).status()).toBe(200);
      await answer(operation);
      expect((await page.reload()).status()).toBe(200);
      await expect(page.locator('main').getByText(expected)).toBeVisible();
    }
    const agentSection = page.locator('main section').filter({
      has: page.getByRole('heading', {name: lang === 'pl'
        ? 'Połączeni agenci, czasy obserwacji i ogłaszane operacje'
        : 'Connected agents, observation times and advertised operations'}),
    });
    const agentRow = agentSection.getByRole('listitem').filter({hasText: agentId});
    await expect(agentRow).toBeVisible();
    const [displayedAgent, observation, operations] = (await agentRow.innerText()).trim().split(' · ');
    expect(displayedAgent).toBe(agentId);
    const parts = observation.match(/^(\d{4}-\d{2}-\d{2}) (\d{1,2})(:\d{2}:\d{2}(?:\.\d+)?) \+00:00:00$/);
    expect(parts).not.toBeNull();
    const [{last_seen_at: observedAt}] = await query(
      'SELECT last_seen_at FROM admin_control_agents WHERE agent_id=$1', [agentId],
    );
    expect(Date.parse(`${parts[1]}T${parts[2].padStart(2, '0')}${parts[3]}Z`))
      .toBe(observedAt.getTime());
    expect(operations.split(', ').sort()).toEqual(advertisedOperations.map(op => `${op}@1 (query)`).sort());
  } finally {
    await agent.close();
  }
});
