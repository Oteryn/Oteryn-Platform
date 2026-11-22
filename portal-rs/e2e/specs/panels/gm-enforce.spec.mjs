// Run with local.mjs or harness.mjs --outbox --gm-enforce and its generated env.json.
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { Secret, TOTP } from 'otpauth';
import { expect, test as base } from '../../fixtures.mjs';
import { loginWithRecoveryCode, query } from '../../staff-rbac-helpers.mjs';
import { fixture } from '../../api/transport.mjs';
import { gameJson, gamePeer, portalCommand, seededAccount, sourceTime } from '../../outbox-helpers.mjs';
import { uuid7 } from '../../game-sim-helpers.mjs';

const outboxEnv = process.env.PORTAL_E2E_OUTBOX_ENV
  ? JSON.parse(await readFile(process.env.PORTAL_E2E_OUTBOX_ENV, 'utf8'))
  : null;

const test = base.extend({
  outboxEnvironment: [async ({}, use) => {
    const previous = Object.fromEntries(Object.keys(outboxEnv ?? {}).map(key => [key, process.env[key]]));
    Object.assign(process.env, outboxEnv);
    try {
      await use();
    } finally {
      for (const [key, value] of Object.entries(previous)) {
        if (value === undefined) delete process.env[key];
        else process.env[key] = value;
      }
    }
  }, { auto: true }],
});
if (outboxEnv) test.use({ baseURL: outboxEnv.PORTAL_E2E_ORIGIN });

const path = '/admin/console/enforcement';
const exchange = '/internal/v1/support/enforcement-commands';
const portfolio = '/internal/v1/game-auth/native-account-characters';
const wireFixtures = 'game-enforcement-wire-v1';
const messages = {
  en: {
    heading: 'Character enforcement', none: 'None', pending: 'Pending in Game',
    unconfirmed: 'No confirmed state', applied: 'Applied in Game', refused: 'Refused by Game', revoked: 'Revoked in Game',
    invalid: 'Invalid input', missing: 'Character not found',
    conflict: 'Enforcement state changed. Reload the dossier.', denied: 'Permission denied',
  },
  pl: {
    heading: 'Sankcje postaci', none: 'Brak', pending: 'Oczekuje na grę',
    unconfirmed: 'Brak potwierdzonego stanu', applied: 'Nałożona w grze', refused: 'Gra odmówiła', revoked: 'Cofnięta w grze',
    invalid: 'Nieprawidłowe dane', missing: 'Nie znaleziono postaci',
    conflict: 'Stan sankcji się zmienił. Odśwież widok.', denied: 'Brak uprawnień',
  },
};

async function staffActor(screen, lang, role) {
  const account = await seededAccount(lang);
  const granted = await query(
    'INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2 RETURNING role_id',
    [account.id, role],
  );
  expect(granted).toHaveLength(1);
  const enrolled = JSON.parse(await portalCommand(['seed', 'account-mfa', JSON.stringify({
    accountId: account.id, password: account.password,
  })]));
  await loginWithRecoveryCode(screen, lang, account, enrolled.recoveryCodes[0]);
  return { account, totp: new TOTP({ secret: Secret.fromBase32(enrolled.totpSecret) }) };
}

async function formPayload(form) {
  const target = await form.getAttribute('action');
  const fields = await form.evaluate(node => Object.fromEntries(new FormData(node)));
  return { target, fields };
}

function post(screen, target, fields) {
  return screen.request.post(target, {
    form: fields,
    headers: { origin: new URL(screen.url()).origin },
    maxRedirects: 0,
  });
}

async function refused(response, status, message) {
  expect(response.status()).toBe(status);
  expect(await response.text()).toContain(message);
}

async function browserSubmit(screen, form, fields, route) {
  const response = screen.waitForResponse(r => r.request().isNavigationRequest()
    && r.request().method() === 'POST' && new URL(r.url()).pathname === route);
  for (const [name, value] of Object.entries(fields)) {
    const input = form.locator(`[name="${name}"]`);
    if (name === 'reason') await input.selectOption(value);
    else await input.fill(value);
  }
  await form.getByRole('button').click();
  expect((await response).status()).toBe(303);
}

function enforcementGame() {
  const reader = gamePeer('enforcement-reader');
  const publisher = gamePeer('enforcement-results');
  let cursor = null;
  return async (operation, expected, result) => {
    // Consume the actual journal, then hash the original command representation.
    let command;
    for (;;) {
      const page = await gameJson(reader, `${exchange}/stream/read`, {
        schema: 'oteryn.enforcement_command_stream_request.v1', after: cursor, limit: 200,
      });
      expect(page.schema).toBe('oteryn.enforcement_command_stream_page.v1');
      expect(page.items.length).toBeGreaterThan(0);
      cursor = page.next_cursor;
      command = page.items.find(item => item.operation_id === operation);
      if (command) break;
    }
    expect(command).toMatchObject(expected);
    const exact = await reader(`${exchange}/commands/read`, {
      schema: 'oteryn.enforcement_command_read_request.v1', operation_id: operation,
    });
    expect(exact.status).toBe(200);
    expect(JSON.parse(exact.raw)).toEqual(command);
    const resultFixture = JSON.parse(fixture(wireFixtures, 'result-applied.json'));
    const receipt = {
      ...resultFixture,
      operation_id: operation,
      sanction_stream_id: command.sanction_stream_id,
      command_digest: createHash('sha256').update(exact.raw).digest('hex'),
      accepted_decision_revision: command.decision_revision,
      effective_until_ms: command.expires_at_ms,
      decided_at_ms: Date.now(),
      ...result,
    };
    const response = await publisher(`${exchange}/results`, receipt);
    expect(response.status).toBe(200);
    expect(response.raw).toBe(fixture(wireFixtures, 'result-ack.json').toString());
    return command;
  };
}

test('GM applies, replaces and revokes MUTE and JAIL through Game receipts', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const text = messages[lang];
  const target = await seededAccount(lang);
  const gm = await staffActor(page, lang, 'game_support');
  const lcfa = gamePeer('lcfa');
  const character = uuid7();
  const name = 'Aldric <Archive>';
  const world = uuid7();
  const url = `${path}?lang=${lang}&character=${character}`;
  const readGame = enforcementGame();
  const snapshotHead = {
    contract_version: 1, source_authority: process.env.OTERYN_LCFA_SOURCE_AUTHORITY,
    projection_epoch: '1',
  };
  await gameJson(lcfa, portfolio, {
    ...snapshotHead, operation: 'PublishAccountCharactersV1', account_id: target.id,
    projection_revision: '1', source_observed_at: sourceTime(),
    characters: [{ character_id: character, world_id: world, name, availability: 'AVAILABLE' }],
  });
  async function refreshProjection() {
    const at = sourceTime();
    await gameJson(lcfa, `${portfolio}/watermark`, {
      ...snapshotHead, operation: 'PublishProjectionWatermarkV1',
      complete_through: at, observed_at: at,
    });
  }
  await refreshProjection();
  expect((await page.goto(`/admin/console/players?lang=${lang}`)).status()).toBe(200);
  await page.getByRole('link', { name: text.heading }).click();
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.getByRole('heading', { name: text.heading })).toBeVisible();
  await page.locator('[name=character]').fill(character);
  await page.locator('form[method=get] button').click();
  await expect(page.getByRole('heading', { name })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(info.project.use.viewport.width);
  await expect(page.locator('[data-action]')).toHaveCount(0);

  const stepForm = page.locator('form[action*="/step-up?"]');
  const step = await formPayload(stepForm);
  expect((await post(page, step.target, { ...step.fields, _csrf: 'forged', code: '000000' })).status()).toBe(403);
  await browserSubmit(page, stepForm, { code: gm.totp.generate() }, `${path}/step-up`);
  await expect(page).toHaveURL(url);

  const viewer = await browser.newContext({
    baseURL: new URL(page.url()).origin, viewport: info.project.use.viewport,
    locale: lang, ignoreHTTPSErrors: true,
  });
  try {
    const readOnly = await viewer.newPage();
    await staffActor(readOnly, lang, 'game_viewer');
    await refreshProjection();
    expect((await readOnly.goto(url)).status()).toBe(200);
    await expect(readOnly.locator('[data-action], form[action*="/step-up?"]')).toHaveCount(0);
    // Obtain a real viewer-bound CSRF token from the staff page before trying the write.
    await readOnly.goto(`/${lang}/account`);
    const viewerCsrf = await readOnly.locator('input[name=_csrf]').first().inputValue();
    const apply = await formPayload(page.locator('[data-profile=MUTE] [data-action=apply]'));
    await refused(await post(readOnly, apply.target, {
      ...apply.fields, _csrf: viewerCsrf, minutes: '15', reason: 'rule_violation',
    }), 403, text.denied);
  } finally {
    await viewer.close();
  }

  for (const profile of ['MUTE', 'JAIL']) {
    await refreshProjection();
    expect((await page.goto(url)).status()).toBe(200);
    const panel = page.locator(`[data-profile=${profile}]`);
    const form = action => panel.locator(`[data-action=${action}]`);
    const status = () => panel.locator('[data-state]');
    const current = () => panel.locator('[data-current]');
    await expect(status()).toHaveText(text.none);
    const early = await formPayload(form('apply'));
    const valid = { minutes: '15', reason: 'disruptive_behavior' };

    for (const minutes of ['0', '1441']) {
      await refused(await post(page, early.target, { ...early.fields, ...valid, minutes }), 400, text.invalid);
    }
    await refused(await post(page, early.target, { ...early.fields, ...valid, reason: '' }), 400, text.invalid);
    expect((await post(page, early.target, { ...early.fields, ...valid, _csrf: 'forged' })).status()).toBe(403);
    const missing = early.target.replace(character, uuid7());
    await refused(await post(page, missing, { ...early.fields, ...valid }), 404, text.missing);
    expect(await query('SELECT 1 FROM support_enforcement_commands WHERE operation_id=$1', [early.fields.operation])).toEqual([]);

    await page.reload();
    const first = await formPayload(form('apply'));
    await browserSubmit(page, form('apply'), valid, path);
    await expect(status()).toHaveText(text.pending);
    await expect(current()).toHaveText(text.unconfirmed);
    expect((await post(page, first.target, { ...first.fields, ...valid })).status()).toBe(303);
    await refused(await post(page, first.target, { ...first.fields, ...valid, minutes: '16' }), 409, text.conflict);
    await refused(await post(page, early.target, { ...early.fields, ...valid }), 409, text.conflict);
    const common = { account_id: target.id, character_id: character, profile };
    const applied = await readGame(first.fields.operation, {
      ...common, intent: 'APPLY', decision_revision: 1, prior_decision_revision: null, reason_code: 7,
    }, { state: 'APPLIED' });
    expect(applied.expires_at_ms - applied.effective_at_ms).toBe(15 * 60_000);

    await page.reload();
    await expect(status()).toHaveText(text.applied);
    await expect(current()).toHaveText(text.applied);
    await expect(panel.locator('time[data-receipt]')).toHaveAttribute('datetime', /T\d{2}:\d{2}:\d{2}\.\d{3}Z$/);

    const refusedReplace = await formPayload(form('replace'));
    const staleRevoke = await formPayload(form('revoke'));
    await browserSubmit(page, form('replace'), { ...valid, minutes: '30' }, path);
    await expect(status()).toHaveText(text.pending);
    await expect(current()).toHaveText(text.applied);
    await readGame(refusedReplace.fields.operation, {
      ...common, intent: 'REPLACE', decision_revision: 2, prior_decision_revision: 1,
      sanction_stream_id: applied.sanction_stream_id,
    }, {
      state: 'REJECTED', result_code: 'LIFECYCLE_CONFLICT', accepted_decision_revision: 1,
      effective_until_ms: applied.expires_at_ms, runtime_effect: 'NONE',
    });
    await page.reload();
    await expect(status()).toHaveText(text.refused);
    await expect(current()).toHaveText(text.applied);
    await expect(panel.getByText('LIFECYCLE_CONFLICT', { exact: true })).toBeVisible();
    await refused(await post(page, staleRevoke.target, {
      ...staleRevoke.fields, reason: 'rule_violation',
    }), 409, text.conflict);

    const replace = await formPayload(form('replace'));
    await browserSubmit(page, form('replace'), { minutes: '30', reason: 'rule_violation' }, path);
    const replaced = await readGame(replace.fields.operation, {
      ...common, intent: 'REPLACE', decision_revision: 3, prior_decision_revision: 1, reason_code: 8,
      sanction_stream_id: applied.sanction_stream_id,
    }, { state: 'REPLACED' });
    expect(replaced.expires_at_ms - replaced.effective_at_ms).toBe(30 * 60_000);
    await page.reload();
    await expect(status()).toHaveText(text.applied);
    await expect(status()).toHaveAttribute('data-operation', replace.fields.operation);

    const revoke = await formPayload(form('revoke'));
    await browserSubmit(page, form('revoke'), { reason: 'rule_violation' }, path);
    await expect(status()).toHaveText(text.pending);
    await expect(current()).toHaveText(text.applied);
    await readGame(revoke.fields.operation, {
      ...common, intent: 'REVOKE', decision_revision: 4, prior_decision_revision: 3,
      sanction_stream_id: applied.sanction_stream_id, effective_at_ms: null, expires_at_ms: null,
    }, { state: 'REVOKED', stream_state: 'REVOKED' });
    await page.reload();
    await expect(status()).toHaveText(text.revoked);
    await expect(current()).toHaveText(text.revoked);
    await expect(panel.locator('time[data-receipt]')).toHaveAttribute('datetime', /T\d{2}:\d{2}:\d{2}\.\d{3}Z$/);

    const audit = await query(
      "SELECT a.id,a.actor,a.target,a.before,a.after FROM audit_log a WHERE action='game.enforcement.requested' AND request_id=$1",
      [first.fields.operation],
    );
    expect(audit).toEqual([{
      id: applied.audit_ref, actor: gm.account.id, target: character, before: null,
      after: {
        operation_id: first.fields.operation, character_id: character, profile, intent: 'APPLY',
        reason_code: 7, duration_minutes: 15, form_version: 0, decision_revision: 1,
        sanction_stream_id: applied.sanction_stream_id, state: 'pending',
        effective_until_ms: applied.expires_at_ms,
      },
    }]);
    const operations = await query(
      'SELECT operation_id FROM support_enforcement_commands WHERE character_id=$1 AND profile=$2 ORDER BY position',
      [character, profile],
    );
    expect(operations.map(row => row.operation_id)).toEqual([
      first.fields.operation, refusedReplace.fields.operation, replace.fields.operation, revoke.fields.operation,
    ]);
  }
});
