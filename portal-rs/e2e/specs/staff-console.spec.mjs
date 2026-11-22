import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { test as base, expect } from '../fixtures.mjs';
import { password, submit, waitForLoginBudget } from '../helpers.mjs';
import { membership, query } from '../staff-rbac-helpers.mjs';
import {
  adminAgent, agentId, characterId, characterName, ledgerId, nodeId, resultRevision, staffSeedEnvironment,
  startStaffConsoleHarness, worldId,
} from '../admin-agent.mjs';

const copies = {
  en: { fresh: 'Fresh', unavailable: 'Unavailable', pending: 'Awaiting a stored result',
    rejected: 'Query rejected', expired: 'Query expired', source: 'Game agent · observed at',
    poll: 'Check stored results', player: 'Players',
    playerSections: ['Characters and dossier', 'Inventory', 'Sanctions'] },
  pl: { fresh: 'Aktualne', unavailable: 'Niedostępne', pending: 'Oczekiwanie na zapisany wynik',
    rejected: 'Zapytanie odrzucone', expired: 'Zapytanie wygasło', source: 'Agent gry · czas obserwacji',
    poll: 'Sprawdź zapisane wyniki', player: 'Gracze',
    playerSections: ['Postacie i dossier', 'Ekwipunek', 'Kary'] },
};
const operations = [
  ['dashboard', 'ops.overview.get', '', '41'],
  ['worlds', 'world.list', '', 'E2E Console World'],
  ['worlds', 'world.get', worldId, 'E2E Console World'],
  ['nodes', 'node.list', '', nodeId],
  ['nodes', 'node.get', nodeId, nodeId],
  ['database', 'db.status', '', 'e2e-admin: 3'],
  ['database', 'db.migrations.list', '', 'e2e-migration-42'],
];
const players = '/admin/console/players';
const audit = '/admin/console/audit';
const queryForm = operation => `form[method=post]:has(input[name=operation][value="${operation}"])`;
const fields = form => form.evaluate(element => Object.fromEntries(new FormData(element)));
const url = (route, params) => `${route}?${new URLSearchParams(params)}`;

async function unavailableSection(page, heading, unavailable) {
  const section = page.locator('main section').filter({
    has: page.getByRole('heading', { level: 2, name: heading, exact: true }),
  });
  const rows = section.locator('ul > li');
  await expect(rows).toHaveText([unavailable]);
  await expect(rows).toBeVisible();
}

function harness(directory) {
  const secret = name => fs.readFileSync(path.join(directory, 'secrets', name), 'utf8');
  const settings = JSON.parse(secret('admin-agent.json'));
  const env = staffSeedEnvironment(JSON.parse(secret('env.json')));
  const seedBinary = secret('seed-binary');
  return {
    ...settings,
    databaseURL: env.OTERYN_DATABASE_URL,
    seed(entity, input) {
      const command = spawnSync(seedBinary, ['seed', entity, JSON.stringify(input)], { env, encoding: 'utf8' });
      assert(command.status === 0, `Portal seed ${entity} failed (exit ${command.status})`);
      try {
        return JSON.parse(command.stdout);
      } catch {
        throw new Error(`Portal seed ${entity} returned invalid JSON`);
      }
    },
  };
}

function seedAccount(setup, lang) {
  const account = { email: `console-${randomUUID()}@example.invalid`, password: password() };
  const { accountId } = setup.seed('account', { ...account, locale: lang });
  const { recoveryCodes } = setup.seed('account-mfa', { accountId, password: account.password });
  return { ...account, accountId, code: recoveryCodes[0] };
}

async function staffLogin(page, lang, account) {
  expect((await page.goto(`/${lang}/login`)).status()).toBe(200);
  await waitForLoginBudget();
  await submit(page, 'main form', { email: account.email, password: account.password });
  await expect(page).toHaveURL(/mfa\/challenge/);
  await submit(page, 'form[action$="/mfa/challenge"]', { code: account.code });
  await expect(page).not.toHaveURL(/\/(?:login|mfa\/challenge)(?:\?|$)/);
}

async function navigation(page, trigger, method = 'GET') {
  const [response] = await Promise.all([
    page.waitForResponse(response => response.request().isNavigationRequest()
      && response.request().method() === method && response.frame() === page.mainFrame()),
    trigger(),
  ]);
  return response;
}

async function post(page, route, form, status) {
  const response = await page.request.post(route, {
    form, headers: { origin: new URL(page.url()).origin }, maxRedirects: 0,
  });
  expect(response.status(), `${route} ${form.operation ?? 'player.search'}`).toBe(status);
}

async function received(agent, operation, target, params = {}) {
  const requests = await agent.poll(3);
  expect(requests.map(request => request.operation)).toEqual([operation]);
  const [request] = requests;
  expect(request.target).toEqual({ world_id: null, node_id: null, character_id: null, account_ref: null, ...target });
  expect(request.params).toEqual(params);
  return request;
}

async function select(page, route, lang, target) {
  if (new URL(page.url()).pathname !== route) {
    expect((await page.goto(url(route, { lang }))).status()).toBe(200);
  }
  const selection = page.locator('main form[method=get]');
  await selection.locator('[name=agent]').fill(agentId);
  await selection.locator('[name=target]').fill(target);
  expect((await navigation(page, () => selection.getByRole('button').click())).status()).toBe(200);
  await expect(page).toHaveURL(url(route, { lang, agent: agentId, target }));
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
}

async function invalidSelection(page, route, lang) {
  const selection = page.locator('main form[method=get]');
  for (const name of ['agent', 'target']) {
    const input = selection.locator(`[name=${name}]`);
    const original = await input.inputValue();
    await input.fill('invalid/id');
    expect(await input.evaluate(control => control.validity.patternMismatch)).toBe(true);
    await input.fill(original);
  }
  expect((await page.request.get(url(route, { lang, agent: 'invalid/id' }))).status()).toBe(400);
}

const test = base.extend({
  staffHarness: [async ({}, use) => {
    const existing = process.env.PORTAL_E2E_ADMIN_AGENT_DIRECTORY;
    const previous = process.env.PORTAL_E2E_DATABASE_URL;
    const managed = existing ? undefined : await startStaffConsoleHarness(previous || process.env.OTERYN_DATABASE_URL);
    try {
      const setup = harness(existing || managed.out);
      process.env.PORTAL_E2E_DATABASE_URL = setup.databaseURL;
      await use(setup);
    } finally {
      if (previous === undefined) delete process.env.PORTAL_E2E_DATABASE_URL;
      else process.env.PORTAL_E2E_DATABASE_URL = previous;
      await managed?.close();
    }
  }, { scope: 'worker', timeout: 600_000 }],
  baseURL: async ({ staffHarness }, use) => { await use(staffHarness.origin); },
});

test('staff console forms use the authenticated Admin Agent channel', async ({ page, browser, staffHarness: setup }, info) => {
  // Keep the scenario budget separate from the request and heartbeat deadlines.
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const copy = copies[lang];
  const agent = adminAgent(setup);
  const contextOptions = {
    baseURL: setup.origin, ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport,
  };
  const manager = await browser.newContext(contextOptions);
  const managerPage = await manager.newPage();
  const restricted = await browser.newContext(contextOptions);
  const deniedPage = await restricted.newPage();
  const auditPage = await page.context().newPage();
  const node = { node_id: nodeId };
  try {
    const admin = seedAccount(setup, lang);
    const operator = seedAccount(setup, lang);
    const support = seedAccount(setup, lang);
    const actor = operator.accountId;
    // The seed CLI has no role action. Bootstrap the initial role as in
    // isolatedAdmin, then grant the restricted role through the RBAC form.
    const granted = await query(
      "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin' RETURNING account_id",
      [admin.accountId],
    );
    expect(granted).toHaveLength(1);
    await staffLogin(managerPage, lang, admin);
    const roles = await query("SELECT id,key FROM roles WHERE key IN ('game_support','support')");
    await membership(managerPage, actor, roles.find(role => role.key === 'game_support').id, 'grant');
    await membership(managerPage, support.accountId, roles.find(role => role.key === 'support').id, 'grant');
    await staffLogin(page, lang, operator);
    await staffLogin(deniedPage, lang, support);
    const deniedCsrf = await deniedPage.locator('form[action$="/logout"] input[name=_csrf]').first().getAttribute('value');
    assert(deniedCsrf, 'Restricted staff session has a CSRF token');
    expect((await deniedPage.goto(url(players, { lang }))).status()).toBe(200);
    await expect(deniedPage.locator('main form')).toHaveCount(0);
    expect((await auditPage.goto(url(audit, { lang }))).status()).toBe(200);
    await unavailableSection(auditPage, copy.source, copy.unavailable);
    expect(await agent.hello()).toEqual({ status: 'registered' });
    expect((await managerPage.goto(url('/admin/console/access', { lang }))).status()).toBe(200);
    for (const [, operation] of operations) {
      await expect(managerPage.locator('main')).toContainText(`${operation}@1 (query)`);
    }

    const submissions = [];
    const requestIds = new Map();
    const checkedSelections = new Set();
    for (const [area, operation, target, evidence] of operations) {
      await test.step(operation, async () => {
        const route = `/console/${area}`;
        await agent.heartbeat();
        await select(page, route, lang, target);
        if (!checkedSelections.has(route)) {
          expect((await deniedPage.goto(url(route, { lang, agent: agentId, target }))).status()).toBe(403);
          await invalidSelection(page, route, lang);
          checkedSelections.add(route);
        }
        const selector = queryForm(operation);
        const form = page.locator(selector);
        await expect(form).toBeVisible();
        const data = await fields(form);
        await post(deniedPage, route, { ...data, _csrf: deniedCsrf }, 403);
        await post(page, route, { ...data, agent: 'invalid/agent' }, 400);
        expect((await agent.poll()).map(request => request.operation)).toEqual([]);
        const admitted = await navigation(page, () => submit(page, selector), 'POST');
        expect(admitted.status()).toBe(303);
        const request = await received(agent, operation, target === worldId ? { world_id: worldId } : node);
        await expect(page).toHaveURL(url(route, { lang, agent: agentId, target, request: request.request_id }));
        await expect(form.locator('..').locator('.op-badge')).toHaveText(`R0 · ${copy.pending}`);
        await agent.answer(request, 'success');
        const reload = await navigation(page, () => page.getByRole('link', { name: copy.poll }).click());
        expect(reload.status()).toBe(200);
        const section = form.locator('..');
        await expect(section.locator('.op-badge')).toHaveText(`R0 · ${copy.fresh}`);
        await expect(section.locator('article')).toContainText(evidence);
        await expect(section.locator('dl').first()).toContainText(resultRevision);
        submissions.push({ route, target, selector, data, heading: await section.getByRole('heading', { level: 2 }).innerText() });
        requestIds.set(operation, request.request_id);
      });
    }

    const search = await test.step('player search', async () => {
      await agent.heartbeat();
      expect((await page.goto(url(players, { lang }))).status()).toBe(200);
      await expect(page.getByRole('heading', { name: copy.player, exact: true })).toBeVisible();
      const selector = 'main form[method=post]';
      const form = page.locator(selector);
      await form.locator('[name=q]').fill('E2E Console');
      const data = await fields(form);
      await post(deniedPage, players, { ...data, _csrf: deniedCsrf }, 403);
      await post(page, players, { ...data, q: '<bad>' }, 400);
      expect((await agent.poll()).map(request => request.operation)).toEqual([]);
      expect((await navigation(page, () => submit(page, selector), 'POST')).status()).toBe(303);
      const request = await received(agent, 'player.search', node, { query: data.q });
      await agent.answer(request, 'success');
      expect((await page.reload()).status()).toBe(200);
      await expect(page.locator('main > p')).toContainText(copy.source);
      await expect(page.locator('main')).toContainText(characterName);
      return data;
    });

    await test.step('character dossier', async () => {
      expect((await page.goto(url(players, { lang }))).status()).toBe(200);
      const form = page.locator('main form[method=get]');
      await form.locator('[name=character]').fill('invalid/id');
      expect((await navigation(page, () => form.getByRole('button').click())).status()).toBe(400);
      expect((await page.goto(url(players, { lang }))).status()).toBe(200);
      await form.locator('[name=character]').fill(characterId);
      expect((await navigation(page, () => form.getByRole('button').click())).status()).toBe(200);
      const target = { ...node, character_id: characterId };
      await agent.answer(await received(agent, 'player.character.get', target), 'success');
      expect((await page.reload()).status()).toBe(200);
      const requests = await agent.poll(3);
      expect(requests.map(request => request.operation)).toEqual(['player.inventory.get', 'sanction.list']);
      for (const request of requests) {
        expect(request.target.character_id).toBe(characterId);
        await agent.answer(request, 'success');
      }
      expect((await page.reload()).status()).toBe(200);
      await expect(page.locator('main > p')).toContainText(copy.source);
      await expect(page.locator('main')).toContainText(`${characterId} · ${characterName}`);
      await expect(page.locator('main')).toContainText('42, 27, 7');
      expect((await deniedPage.goto(url(players, { lang, character: characterId }))).status()).toBe(403);
    });

    const auditFields = { request_id: requestIds.get('world.get'), admin: actor, operation: 'world.get', target: worldId };
    const auditFilter = url(audit, { lang, ...auditFields });
    await test.step('audit filter', async () => {
      await agent.heartbeat();
      const form = auditPage.locator('main form[method=get]');
      for (const [name, value] of Object.entries(auditFields)) {
        await form.locator(`[name=${name}]`).fill(value);
      }
      expect((await navigation(auditPage, () => form.getByRole('button').click())).status()).toBe(200);
      await expect(auditPage).toHaveURL(auditFilter);
      const request = await received(agent, 'admin.ledger.query', node, {
        request_id: requestIds.get('world.get'), admin_id: actor, operation: 'world.get', target: worldId,
      });
      await agent.answer(request, 'success');
      expect((await auditPage.reload()).status()).toBe(200);
      await expect(auditPage.getByText(new RegExp(ledgerId))).toContainText(requestIds.get('world.get'));
      expect((await deniedPage.goto(auditFilter)).status()).toBe(403);
      await form.locator('[name=request_id]').fill('bad-uuid');
      expect((await navigation(auditPage, () => form.getByRole('button').click())).status()).toBe(400);
      expect((await agent.poll()).map(request => request.operation)).toEqual([]);
    });

    await test.step('agent refusal and timeout retain distinct states', async () => {
      const route = '/console/dashboard';
      const operation = 'ops.overview.get';
      const selector = queryForm(operation);
      const section = page.locator(selector).locator('..');
      await select(page, route, lang, '');
      for (const outcome of ['refusal', 'timeout']) {
        expect((await navigation(page, () => submit(page, selector), 'POST')).status()).toBe(303);
        const request = await received(agent, operation, node);
        await agent.answer(request, outcome);
        expect((await page.reload()).status()).toBe(200);
        const badge = section.locator('.op-badge');
        await expect(badge).toHaveText(`R0 · ${outcome === 'refusal' ? copy.rejected : copy.pending}`);
        await expect(section.locator('article')).toHaveCount(0);
        if (outcome === 'timeout') {
          await expect.poll(async () => {
            await agent.heartbeat();
            expect((await page.reload()).status()).toBe(200);
            return badge.textContent();
          }, { timeout: 70_000, intervals: [1000] }).toBe(`R0 · ${copy.expired}`);
          await expect(section.locator('article')).toHaveCount(0);
        }
      }
    });

    await test.step('stopped agent makes every Game form unavailable', async () => {
      agent.close();
      await expect.poll(async () => {
        expect((await page.reload()).status()).toBe(200);
        return page.locator('main section').first().locator('.op-badge').textContent();
      }, { timeout: 35_000, intervals: [1000] }).toBe(`R0 · ${copy.unavailable}`);
      for (const { route, target, selector, data, heading } of submissions) {
        await select(page, route, lang, target);
        await expect(page.locator(selector)).toHaveCount(0);
        const section = page.locator('main section').filter({ has: page.getByRole('heading', { name: heading, exact: true }) });
        await expect(section.locator('.op-badge')).toHaveText(`R0 · ${copy.unavailable}`);
        await expect(section.locator('article')).toHaveCount(0);
        await post(page, route, data, 503);
      }
      expect((await page.goto(url(players, { lang }))).status()).toBe(200);
      await expect(page.locator('main form[method=post]')).toHaveCount(0);
      const dossierForm = page.locator('main form[method=get]');
      await dossierForm.locator('[name=character]').fill(characterId);
      expect((await navigation(page, () => dossierForm.getByRole('button').click())).status()).toBe(200);
      for (const heading of copy.playerSections) {
        await unavailableSection(page, heading, copy.unavailable);
      }
      await expect(page.locator('main')).not.toContainText(characterName);
      await post(page, players, search, 303);
      expect((await page.goto(url(players, { lang, q: search.q }))).status()).toBe(200);
      for (const heading of copy.playerSections) {
        await unavailableSection(page, heading, copy.unavailable);
      }
      await expect(page.locator('main')).not.toContainText(characterName);

      expect((await auditPage.goto(url(audit, { lang }))).status()).toBe(200);
      const auditForm = auditPage.locator('main form[method=get]');
      for (const [name, value] of Object.entries(auditFields)) {
        await auditForm.locator(`[name=${name}]`).fill(value);
      }
      expect((await navigation(auditPage, () => auditForm.getByRole('button').click())).status()).toBe(200);
      await unavailableSection(auditPage, copy.source, copy.unavailable);
      await expect(auditPage.locator('main')).not.toContainText(ledgerId);
    });
  } finally {
    agent.close();
    await auditPage.close();
    await restricted.close();
    await manager.close();
  }
});
