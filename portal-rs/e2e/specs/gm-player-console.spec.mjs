import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { test as base, expect } from '../fixtures.mjs';
import { login, password, submit } from '../helpers.mjs';
import { confirmStaffPassword, loginWithRecoveryCode, membership, query, role } from '../staff-rbac-helpers.mjs';
import {
  accountSanctionId, adminAgent, characterId, characterName, itemId, nodeId, sanctionId,
  publishStaffPortfolio, startStaffConsoleHarness, wrongAccountSanctionCharacterId, wrongCharacterId, wrongSanctionCharacterId,
} from '../admin-agent.mjs';

const route = '/admin/console/players';
const url = (lang, params = {}) => `${route}?${new URLSearchParams({lang, ...params})}`;
const searchForm = 'main form[method=post]';
const dossierForm = 'main form[method=get]';

function readHarness(directory) {
  const secret = name => fs.readFileSync(path.join(directory, 'secrets', name), 'utf8');
  const settings = JSON.parse(secret('admin-agent.json'));
  const env = JSON.parse(secret('env.json'));
  const binary = secret('seed-binary');
  const report = JSON.parse(fs.readFileSync(path.join(directory, 'report.json'), 'utf8'));
  return {
    ...settings,
    databaseURL: env.OTERYN_DATABASE_URL,
    lcfaClient: report.clients.lcfa,
    lcfaAuthority: env.OTERYN_LCFA_SOURCE_AUTHORITY,
    seed(entity, input) {
      const result = spawnSync(binary, ['seed', entity, JSON.stringify(input)], { env, encoding: 'utf8' });
      // Seed arguments and output can contain disposable credentials.
      assert.equal(result.status, 0, `Portal seed failed for ${entity}`);
      try {
        return JSON.parse(result.stdout);
      } catch {
        throw new Error(`Portal seed returned invalid JSON for ${entity}`);
      }
    },
  };
}

function seedAccount(setup, lang, enroll = true) {
  const account = { email: `gm-console-${randomUUID()}@example.invalid`, password: password() };
  const { accountId } = setup.seed('account', { ...account, locale: lang });
  if (!enroll) return { ...account, accountId };
  const { recoveryCodes } = setup.seed('account-mfa', { accountId, password: account.password });
  return { ...account, accountId, code: recoveryCodes[0], nextCode: recoveryCodes[1] };
}

async function navigate(page, method, trigger) {
  const [response] = await Promise.all([
    page.waitForResponse(response => response.request().isNavigationRequest()
      && response.request().method() === method && response.frame() === page.mainFrame()),
    trigger(),
  ]);
  return response;
}

function expectRequest(request, operation, actor, target = null, params = {}) {
  expect(request).toMatchObject({
    operation, operation_version: 1, kind: 'query', risk_class: 'R1', dry_run: false,
    actor: {
      admin_id: actor,
      permission: operation === 'sanction.list' ? 'game.sanction.read' : 'game.player.read',
      roles: ['game_master_senior'],
      mfa_at: expect.any(String),
    },
  });
  expect(request.target).toEqual({ world_id: null, node_id: nodeId, character_id: target, account_ref: null });
  expect(request.params).toEqual(params);
  expect(request.admin_operation_id).toBe(request.request_id);
}

async function pollDossierChildren(agent) {
  const requests = await agent.poll(3);
  expect(requests).toHaveLength(2);
  expect(requests.map(request => request.operation).sort()).toEqual(['player.inventory.get', 'sanction.list']);
  return {
    inventory: requests.find(request => request.operation === 'player.inventory.get'),
    sanctions: requests.find(request => request.operation === 'sanction.list'),
  };
}

async function expectReadAudit(request, actor, target) {
  expect(await query(`SELECT actor, admin_control_target FROM audit_log
    WHERE request_id=$1 AND operation=$2 AND outcome='succeeded'`,
  [request.request_id, request.operation])).toContainEqual({ actor, admin_control_target: target });
}

const test = base.extend({
  staffHarness: [async ({}, use) => {
    const existing = process.env.PORTAL_E2E_ADMIN_AGENT_DIRECTORY;
    const previous = process.env.PORTAL_E2E_DATABASE_URL;
    const managed = existing ? undefined : await startStaffConsoleHarness(previous || process.env.OTERYN_DATABASE_URL);
    try {
      const setup = readHarness(existing || managed.out);
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

test('GM player reads use signed results from the mTLS Admin Agent', async ({page, browser, staffHarness: setup}, info) => {
  test.setTimeout(480_000);
  await assert.rejects(startStaffConsoleHarness('postgresql://fixture@['), error => {
    assert.equal(error.message, 'A local PostgreSQL URL is required');
    assert.equal(Object.hasOwn(error, 'input'), false);
    return true;
  });
  const lang = info.project.metadata.locale;
  const copy = lang === 'en'
    ? { player: 'Players', source: 'Game agent · observed at', unavailable: 'Unavailable', inventory: 'Inventory', sanctions: 'Sanctions', activeSanction: 'Warning · Active', inactiveSanction: 'Notation · Inactive', rejected: 'Query rejected' }
    : { player: 'Gracze', source: 'Agent gry · czas obserwacji', unavailable: 'Niedostępne', inventory: 'Ekwipunek', sanctions: 'Kary', activeSanction: 'Ostrzeżenie · Aktywna', inactiveSanction: 'Adnotacja · Nieaktywna', rejected: 'Zapytanie odrzucone' };
  const agent = adminAgent(setup);
  const options = {baseURL: setup.origin, ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport};
  const manager = await browser.newContext(options);
  const restricted = await browser.newContext(options);
  const secondContext = await browser.newContext(options);
  const secondPage = await secondContext.newPage();
  const managerPage = await manager.newPage();
  const denied = await restricted.newPage();
  try {
    const anonymous = await page.request.get(url(lang), {maxRedirects: 0});
    expect(anonymous.status()).toBe(303);
    expect(anonymous.headers().location).toBe('/login');

    const admin = seedAccount(setup, lang);
    const gm = seedAccount(setup, lang);
    const support = seedAccount(setup, lang);
    const second = seedAccount(setup, lang, false);
    const subject = seedAccount(setup, lang, false);
    expect(await query("UPDATE accounts SET status='suspended' WHERE id=$1 AND status='active' RETURNING status",
      [subject.accountId])).toEqual([{status: 'suspended'}]);
    // Seed has no role command. Bootstrap the initial administrator as in isolatedAdmin.
    expect(await query(`INSERT INTO account_roles(account_id,role_id)
      SELECT $1,id FROM roles WHERE key IN ('platform_admin','game_master_senior') RETURNING account_id`,
    [admin.accountId])).toEqual([{account_id: admin.accountId}, {account_id: admin.accountId}]);
    await loginWithRecoveryCode(managerPage, lang, admin, admin.code);
    await confirmStaffPassword(managerPage, lang, admin.password);
    const roles = await query("SELECT id,key FROM roles WHERE key IN ('game_master_senior','support')");
    const seniorRole = roles.find(role => role.key === 'game_master_senior').id;
    const playerReadRole = await role(managerPage, 'admin.access game.player.read');
    await membership(managerPage, gm.accountId, seniorRole, 'grant');
    await membership(managerPage, second.accountId, roles.find(role => role.key === 'game_master_senior').id, 'grant');
    await membership(managerPage, support.accountId, roles.find(role => role.key === 'support').id, 'grant');
    await loginWithRecoveryCode(page, lang, gm, gm.code);
    await loginWithRecoveryCode(denied, lang, support, support.code);
    const deniedCsrf = await denied.locator('form[action$="/logout"] input[name=_csrf]').first().inputValue();

    await login(secondPage, lang, second);
    expect(await agent.hello()).toEqual({status: 'registered'});
    expect((await secondPage.goto(url(lang, {character: characterId}))).status()).toBe(403);
    expect(await agent.poll()).toEqual([]);
    expect(await query('SELECT request_id FROM admin_control_requests WHERE actor=$1', [second.accountId])).toEqual([]);
    const {recoveryCodes} = setup.seed('account-mfa', {accountId: second.accountId, password: second.password});
    await loginWithRecoveryCode(secondPage, lang, second, recoveryCodes[0]);
    const secondCsrf = await secondPage.locator('form[action$="/logout"] input[name=_csrf]').first().inputValue();
    await agent.heartbeat();
    expect((await denied.goto(url(lang))).status()).toBe(200);
    await expect(denied.locator('main form')).toHaveCount(0);
    expect((await denied.goto(url(lang, {character: characterId}))).status()).toBe(403);

    const landing = await page.goto(url(lang));
    expect(landing.status()).toBe(200);
    expect(landing.headers()['cache-control']).toBe('no-store');
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    await expect(page.getByRole('heading', {name: copy.player, exact: true})).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);

    const search = page.locator(searchForm);
    await search.locator('[name=q]').fill('E2E Console');
    const fields = await search.evaluate(form => Object.fromEntries(new FormData(form)));
    const post = (actor, form) => actor.request.post(route, {
      form, maxRedirects: 0, headers: {origin: setup.origin},
    });
    expect((await post(denied, {...fields, _csrf: deniedCsrf})).status()).toBe(403);
    expect((await post(page, {...fields, q: '<bad>'})).status()).toBe(400);
    expect(await agent.poll()).toEqual([]);

    // submit() verifies the missing-CSRF response before the genuine browser POST.
    expect((await navigate(page, 'POST', () => submit(page, searchForm))).status()).toBe(303);
    const found = await agent.poll(3);
    expect(found).toHaveLength(1);
    expectRequest(found[0], 'player.search', gm.accountId, null, {query: fields.q});
    await expect(page.getByText(copy.unavailable, {exact: true}).first()).toBeVisible();
    await agent.answer(found[0], 'success');
    expect((await page.reload()).status()).toBe(200);
    await expect(page.locator('main > p')).toContainText(copy.source);
    await expect(page.locator('main > p')).toContainText(found[0].request_id);
    await expect(page.locator('main')).toContainText(characterName);
    await expectReadAudit(found[0], gm.accountId, {view: 'search'});
    expect((await secondPage.goto(url(lang, {q: fields.q}))).status()).toBe(200);
    await expect(secondPage.getByText(copy.unavailable, {exact: true}).first()).toBeVisible();
    await expect(secondPage.locator('main')).not.toContainText(characterName);
    await expect(secondPage.locator('main > p')).not.toContainText(found[0].request_id);
    expect(await agent.poll()).toEqual([]);

    const dossiers = page.locator(dossierForm);
    const openDossier = async id => {
      await dossiers.locator('[name=character]').fill(id);
      return navigate(page, 'GET', () => dossiers.getByRole('button').click());
    };
    expect((await openDossier('invalid/id')).status()).toBe(400);
    expect(await agent.poll()).toEqual([]);
    expect((await page.goto(url(lang))).status()).toBe(200);

    expect((await openDossier(characterId)).status()).toBe(200);
    const dossier = await agent.poll(3);
    expect(dossier).toHaveLength(1);
    expectRequest(dossier[0], 'player.character.get', gm.accountId, characterId);
    await agent.answer(dossier[0], 'success');
    expect((await page.reload()).status()).toBe(200);
    await expect(page.locator('main > p')).toContainText(dossier[0].request_id);
    await expect(page.locator('main')).toContainText(`${characterId} · ${characterName}`);
    await expect(page.locator('main')).toContainText('42, 27, 7');
    await expectReadAudit(dossier[0], gm.accountId, {character_id: characterId});

    const publishPortfolios = async (portfolios, { cover, lagSeconds = 0 } = {}) => {
      if (cover) {
        // LCFA uses whole seconds and allows one second of clock uncertainty.
        const completeThrough = Math.floor(Date.parse(cover) / 1000) + 2;
        await expect.poll(() => Math.floor(Date.now() / 1000)).toBeGreaterThanOrEqual(completeThrough);
      }
      const [{epoch}] = await query("SELECT (COALESCE(highest_epoch::numeric, 0)+1)::text epoch FROM account_character_feed WHERE id=1");
      await publishStaffPortfolio(setup, portfolios, epoch, lagSeconds);
    };
    const subjectPortfolio = {account: subject.accountId, character: characterId};
    await publishPortfolios([subjectPortfolio], {lagSeconds: 4});
    const {inventory: subjectInventory, sanctions: subjectSanctions} = await pollDossierChildren(agent);
    expectRequest(subjectInventory, 'player.inventory.get', gm.accountId, characterId);
    expectRequest(subjectSanctions, 'sanction.list', gm.accountId, characterId);
    await agent.answer(subjectInventory, 'success');
    const completed = await agent.answer(subjectSanctions, 'success', subject.accountId);
    expect((await page.reload()).status()).toBe(200);
    const section = name => page.locator('main section').filter({has: page.getByRole('heading', {name, exact: true})});
    await expect(section(copy.inventory)).toContainText(`${itemId} · 42 · bag · 2`);
    await expect(section(copy.sanctions)).toContainText(copy.unavailable);
    await expect(section(copy.sanctions)).not.toContainText(accountSanctionId);
    const sanctionReads = () => query("SELECT id FROM audit_log WHERE request_id=$1 AND outcome='succeeded' ORDER BY id",
      [subjectSanctions.request_id]);
    expect(await sanctionReads()).toEqual([]);
    expect(await agent.poll()).toEqual([]);

    await publishPortfolios([subjectPortfolio], {cover: completed});
    expect((await page.reload()).status()).toBe(200);
    await expect(section(copy.inventory)).toContainText(`${itemId} · 42 · bag · 2`);
    await expect(section(copy.sanctions)).toContainText(`${sanctionId} · ${copy.activeSanction}`);
    await expect(section(copy.sanctions)).toContainText(`${accountSanctionId} · ${copy.inactiveSanction}`);
    for (const request of [subjectInventory, subjectSanctions]) {
      await expectReadAudit(request, gm.accountId, {character_id: characterId});
    }
    const priorSanctionReads = await sanctionReads();
    await publishPortfolios([subjectPortfolio, {account: gm.accountId, character: characterId}], {cover: completed});
    expect((await page.reload()).status()).toBe(200);
    await expect(section(copy.inventory)).toContainText(`${itemId} · 42 · bag · 2`);
    await expect(section(copy.sanctions)).toContainText(copy.unavailable);
    await expect(section(copy.sanctions)).not.toContainText(accountSanctionId);
    expect(await sanctionReads()).toEqual(priorSanctionReads);
    expect(await agent.poll()).toEqual([]);

    expect((await secondPage.goto(url(lang, {character: characterId}))).status()).toBe(200);
    await expect(secondPage.locator('main')).not.toContainText(characterName);
    await expect(secondPage.locator('main > p')).not.toContainText(dossier[0].request_id);
    const secondDossier = await agent.poll(3);
    expect(secondDossier).toHaveLength(1);
    expectRequest(secondDossier[0], 'player.character.get', second.accountId, characterId);
    await agent.answer(secondDossier[0], 'refusal');

    await agent.heartbeat();
    expect((await openDossier(wrongCharacterId)).status()).toBe(200);
    const mismatch = await agent.poll(3);
    expect(mismatch).toHaveLength(1);
    expectRequest(mismatch[0], 'player.character.get', gm.accountId, wrongCharacterId);
    await agent.answer(mismatch[0], 'success');
    const rejectedProjection = await page.reload();
    expect(rejectedProjection.status()).toBe(503);
    expect(await rejectedProjection.text()).not.toContain(characterName);
    expect(await query("SELECT id FROM audit_log WHERE request_id=$1 AND outcome='succeeded'",
      [mismatch[0].request_id])).toEqual([]);
    expect(await agent.poll()).toEqual([]);

    await agent.heartbeat();
    expect((await page.goto(url(lang))).status()).toBe(200);
    expect((await openDossier(wrongSanctionCharacterId)).status()).toBe(200);
    const firstRequests = await agent.poll(3);
    expect(firstRequests).toHaveLength(1);
    const [firstDossier] = firstRequests;
    expectRequest(firstDossier, 'player.character.get', gm.accountId, wrongSanctionCharacterId);
    await agent.answer(firstDossier, 'refusal');

    const retryRoute = route + '/retry';
    const retryQuery = async (operation, previous) => {
      const form = page.locator(`form[data-operation="${operation}"]`);
      for (let reload = 0; reload < 2; reload++) {
        expect((await page.reload()).status()).toBe(200);
        await expect(form.getByRole('status')).toContainText(copy.rejected);
        await expect(form.locator('[name=request]')).toHaveValue(previous.request_id);
        expect(await agent.poll()).toEqual([]);
      }
      const submitted = await form.evaluate(node => Object.fromEntries(new FormData(node)));
      // submit() verifies that the explicit retry also requires CSRF.
      const send = overrides => page.request.post(retryRoute, {
        form: {...submitted, ...overrides}, maxRedirects: 0, headers: {origin: setup.origin},
      });
      expect((await send({operation: 'staff.ban'})).status()).toBe(400);
      const foreign = await secondPage.request.post(retryRoute, {
        form: {...submitted, _csrf: secondCsrf}, maxRedirects: 0, headers: {origin: setup.origin},
      });
      expect(foreign.status()).toBe(409);
      expect(await agent.poll()).toEqual([]);
      expect((await navigate(page, 'POST', () => submit(page, `form[data-operation="${operation}"]`))).status()).toBe(303);
      const requests = await agent.poll(3);
      expect(requests).toHaveLength(1);
      expectRequest(requests[0], operation, gm.accountId, previous.target.character_id);
      expect(requests[0].request_id).not.toBe(previous.request_id);
      expect((await send({})).status()).toBe(409);
      expect(await agent.poll()).toEqual([]);
      return requests[0];
    };

    await agent.answer(await retryQuery('player.character.get', firstDossier), 'success');
    expect((await page.reload()).status()).toBe(200);
    const {inventory, sanctions} = await pollDossierChildren(agent);
    expectRequest(inventory, 'player.inventory.get', gm.accountId, wrongSanctionCharacterId);
    expectRequest(sanctions, 'sanction.list', gm.accountId, wrongSanctionCharacterId);
    await agent.answer(inventory, 'refusal');
    await agent.answer(await retryQuery('player.inventory.get', inventory), 'success');
    expect((await page.reload()).status()).toBe(200);
    await expect(section(copy.inventory)).toContainText(`${itemId} · 42 · bag · 2`);

    await agent.answer(sanctions, 'success', undefined, characterId);
    const wrongSanction = await page.reload();
    expect(wrongSanction.status()).toBe(503);
    expect(await wrongSanction.text()).not.toContain(sanctionId);
    expect(await query("SELECT id FROM audit_log WHERE request_id=$1 AND outcome='succeeded'",
      [sanctions.request_id])).toEqual([]);
    expect(await agent.poll()).toEqual([]);

    expect((await page.goto(url(lang))).status()).toBe(200);
    expect((await navigate(page, 'POST', () => submit(page, searchForm, {q: 'Rejected search'}))).status()).toBe(303);
    const refused = await agent.poll(3);
    expect(refused).toHaveLength(1);
    expectRequest(refused[0], 'player.search', gm.accountId, null, {query: 'Rejected search'});
    await agent.answer(refused[0], 'refusal');
    expect((await page.reload()).status()).toBe(200);
    await expect(page.getByText(copy.unavailable, {exact: true}).first()).toBeVisible();
    await expect(page.locator('main')).not.toContainText(characterName);
    expect(await query('SELECT status FROM admin_control_requests WHERE request_id=$1',
      [refused[0].request_id])).toEqual([{status: 'rejected'}]);
    expect(await agent.poll()).toEqual([]);
    await agent.heartbeat();
    expect((await openDossier(wrongAccountSanctionCharacterId)).status()).toBe(200);
    const accountDossiers = await agent.poll(3);
    expect(accountDossiers).toHaveLength(1);
    expectRequest(accountDossiers[0], 'player.character.get', gm.accountId, wrongAccountSanctionCharacterId);
    await agent.answer(accountDossiers[0], 'success');
    expect((await page.reload()).status()).toBe(200);
    const {inventory: accountInventory, sanctions: wrongAccountSanction} = await pollDossierChildren(agent);
    expectRequest(accountInventory, 'player.inventory.get', gm.accountId, wrongAccountSanctionCharacterId);
    expectRequest(wrongAccountSanction, 'sanction.list', gm.accountId, wrongAccountSanctionCharacterId);
    await agent.answer(accountInventory, 'success');
    const wrongAccountCompleted = await agent.answer(wrongAccountSanction, 'success', subject.accountId);
    await publishPortfolios([subjectPortfolio, {account: gm.accountId, character: wrongAccountSanctionCharacterId}],
      {cover: wrongAccountCompleted});
    const wrongAccount = await page.reload();
    expect(wrongAccount.status()).toBe(200);
    await expect(section(copy.inventory)).toContainText(`${itemId} · 42 · bag · 2`);
    await expect(section(copy.sanctions)).toContainText(copy.unavailable);
    await expect(section(copy.sanctions)).not.toContainText(accountSanctionId);
    expect(await query("SELECT id FROM audit_log WHERE request_id=$1 AND outcome='succeeded'",
      [wrongAccountSanction.request_id])).toEqual([]);
    expect(await agent.poll()).toEqual([]);

    // The latest dossier succeeded on retry before the reconnect. Its older
    // refusal must neither reappear nor block a fresh query in the new epoch.
    expect(await agent.hello()).toEqual({status: 'registered'});
    expect((await page.goto(url(lang, {character: wrongSanctionCharacterId}))).status()).toBe(200);
    await expect(page.locator('form[data-operation="player.character.get"]')).toHaveCount(0);
    await expect(page.locator('main')).not.toContainText(characterName);
    const afterReconnect = await agent.poll(3);
    expect(afterReconnect).toHaveLength(1);
    expectRequest(afterReconnect[0], 'player.character.get', gm.accountId, wrongSanctionCharacterId);
    await agent.answer(afterReconnect[0], 'success');
    expect((await page.reload()).status()).toBe(200);
    await expect(page.locator('main > p')).toContainText(afterReconnect[0].request_id);

    const {inventory: newInventory, sanctions: refusedSanction} = await pollDossierChildren(agent);
    await agent.answer(newInventory, 'success');
    await agent.answer(refusedSanction, 'refusal');
    const acceptedRetry = await retryQuery('sanction.list', refusedSanction);
    expect(await query(`SELECT a.actor, a.target, a.before, a.after
      FROM audit_log a JOIN admin_control_agents g ON g.agent_id=a.target
      WHERE a.request_id=$1 AND a.action='console.query.admitted'
        AND a.after->>'owner_epoch'=g.owner_epoch::text`, [acceptedRetry.request_id]))
      .toEqual([{actor: gm.accountId, target: 'staff-console-e2e', before: null, after: {owner_epoch: expect.any(Number)}}]);
    await agent.answer(acceptedRetry, 'success');
    expect((await page.reload()).status()).toBe(200);
    await expect(section(copy.sanctions)).toContainText(`${sanctionId} · ${copy.activeSanction}`);
    await expectReadAudit(acceptedRetry, gm.accountId, {character_id: wrongSanctionCharacterId});

    expect((await page.goto(url(lang, {character: wrongAccountSanctionCharacterId}))).status()).toBe(200);
    const latestDossiers = await agent.poll(3);
    expect(latestDossiers).toHaveLength(1);
    expectRequest(latestDossiers[0], 'player.character.get', gm.accountId, wrongAccountSanctionCharacterId);
    await agent.answer(latestDossiers[0], 'success');
    expect((await page.reload()).status()).toBe(200);
    const {inventory: latestInventory, sanctions: latestSanctions} = await pollDossierChildren(agent);
    await agent.answer(latestInventory, 'success');
    await agent.answer(latestSanctions, 'refusal');
    expect((await page.reload()).status()).toBe(200);
    const sanctionRetry = page.locator('form[data-operation="sanction.list"]');
    await expect(sanctionRetry.getByRole('status')).toContainText(copy.rejected);
    const ownSanctionRetry = await sanctionRetry.evaluate(form => Object.fromEntries(new FormData(form)));
    expect(ownSanctionRetry.request).toBe(latestSanctions.request_id);
    await membership(managerPage, gm.accountId, playerReadRole, 'grant');
    await membership(managerPage, gm.accountId, seniorRole, 'revoke');
    await loginWithRecoveryCode(page, lang, gm, gm.nextCode);
    await agent.heartbeat();
    expect((await page.goto(url(lang))).status()).toBe(200);
    await expect(page.locator(dossierForm)).toBeVisible();
    const playerReadCsrf = await page.locator(searchForm).locator('[name=_csrf]').inputValue();
    const sanctionRequests = () => query("SELECT request_id FROM admin_control_requests WHERE actor=$1 AND operation='sanction.list' ORDER BY request_id", [gm.accountId]);
    const admittedBeforeRefusal = await sanctionRequests();
    const forbidden = await page.request.post(retryRoute, {
      form: {...ownSanctionRetry, _csrf: playerReadCsrf}, maxRedirects: 0, headers: {origin: setup.origin},
    });
    expect(forbidden.status()).toBe(403);
    expect(await agent.poll()).toEqual([]);
    expect(await sanctionRequests()).toEqual(admittedBeforeRefusal);
  } finally {
    agent.close();
    await secondContext.close();
    await restricted.close();
    await manager.close();
  }
});
