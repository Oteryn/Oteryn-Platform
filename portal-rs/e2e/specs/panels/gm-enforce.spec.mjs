// Run with local.mjs or harness.mjs --outbox --gm-enforce and its generated env.json.
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { Secret, TOTP } from 'otpauth';
import { expect, test as base } from '../../fixtures.mjs';
import { loginWithRecoveryCode, query } from '../../staff-rbac-helpers.mjs';
import { pool } from '../../helpers.mjs';
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
    stepUp: 'Confirm with a fresh TOTP code',
  },
  pl: {
    heading: 'Sankcje postaci', none: 'Brak', pending: 'Oczekuje na grę',
    unconfirmed: 'Brak potwierdzonego stanu', applied: 'Nałożona w grze', refused: 'Gra odmówiła', revoked: 'Cofnięta w grze',
    invalid: 'Nieprawidłowe dane', missing: 'Nie znaleziono postaci',
    conflict: 'Stan sankcji się zmienił. Odśwież widok.', denied: 'Brak uprawnień',
    stepUp: 'Potwierdź działanie kodem TOTP',
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
  return async (operation, expected) => {
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
    };
    async function publish(outcome) {
      const response = await publisher(`${exchange}/results`, { ...receipt, decided_at_ms: Date.now(), ...outcome });
      expect(response.status).toBe(200);
      expect(response.raw).toBe(fixture(wireFixtures, 'result-ack.json').toString());
    }
    return { command, publish };
  };
}

test('GM applies, replaces and revokes MUTE and JAIL through Game receipts', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const text = messages[lang];
  const target = await seededAccount(lang);
  const gm = await staffActor(page, lang, 'game_master_senior');
  const lcfa = gamePeer('lcfa');
  const character = uuid7();
  const name = 'Aldric <Archive>';
  const world = uuid7();
  const url = `${path}?lang=${lang}&character=${character}`;
  const readGame = enforcementGame();
  async function expectAccountEligible(eligible) {
    expect(await query(
      'SELECT account_value_eligible($1) AS value, community_invitation_delivery_eligible($1) AS invitation',
      [target.id],
    )).toEqual([{ value: eligible, invitation: eligible }]);
  }
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
  async function postFresh(screen, targetPath, fields) {
    await refreshProjection();
    return post(screen, targetPath, fields);
  }
  async function submitFresh(form, fields, route) {
    await refreshProjection();
    await browserSubmit(page, form, fields, route);
  }
  async function reloadDossier() {
    await refreshProjection();
    expect((await page.reload()).status()).toBe(200);
  }
  await refreshProjection();
  expect((await page.goto(`/admin/console/players?lang=${lang}`)).status()).toBe(200);
  await page.getByRole('link', { name: text.heading }).click();
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.getByRole('heading', { name: text.heading })).toBeVisible();
  const malformed = await page.request.get(`${path}?lang=${lang}&character=invalid`);
  await refused(malformed, 400, text.invalid);
  await page.locator('[name=character]').fill(character);
  await page.locator('form[method=get] button').click();
  await expect(page.getByRole('heading', { name })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(info.project.use.viewport.width);
  await expect(page.locator('[data-action]')).toHaveCount(0);

  const stepForm = page.locator('form[action*="/step-up?"]');
  const step = await formPayload(stepForm);
  await refreshProjection();
  const code = gm.totp.generate();
  expect((await post(page, step.target, { ...step.fields, _csrf: 'forged', code })).status()).toBe(403);
  await browserSubmit(page, stepForm, { code }, `${path}/step-up`);
  await expect(page).toHaveURL(url);

  const viewer = await browser.newContext({
    baseURL: new URL(page.url()).origin, viewport: info.project.use.viewport,
    locale: lang, ignoreHTTPSErrors: true,
  });
  try {
    const readOnly = await viewer.newPage();
    const warningWriter = await staffActor(readOnly, lang, 'game_support');
    await refreshProjection();
    expect((await readOnly.goto(url)).status()).toBe(200);
    await expect(readOnly.locator('[data-action], form[action*="/step-up?"]')).toHaveCount(0);
    // Use the warning writer’s own CSRF token and valid TOTP.
    await readOnly.goto(`/${lang}/account`);
    const warningWriterCsrf = await readOnly.locator('input[name=_csrf]').first().inputValue();
    await refused(await post(readOnly, step.target, {
      ...step.fields, _csrf: warningWriterCsrf, code: warningWriter.totp.generate(),
    }), 403, text.denied);
    const apply = await formPayload(page.locator('[data-profile=MUTE] [data-action=apply]'));
    await refused(await postFresh(readOnly, apply.target, {
      ...apply.fields, _csrf: warningWriterCsrf, minutes: '15', reason: 'rule_violation',
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
    await expect(current()).toHaveText(text.unconfirmed);
    const early = await formPayload(form('apply'));
    const valid = { minutes: '15', reason: 'disruptive_behavior' };

    if (profile === 'MUTE') {
      await refreshProjection();
      const db = pool();
      const locker = await db.connect();
      try {
        await locker.query('BEGIN');
        await locker.query('SELECT id FROM account_character_feed WHERE id=1 FOR UPDATE');
        expect((await page.request.get(url, { timeout: 8_000 })).status()).toBe(503);
        const blocked = await page.request.post(early.target, {
          form: { ...early.fields, ...valid }, headers: { origin: new URL(url, page.url()).origin },
          maxRedirects: 0, timeout: 8_000,
        });
        expect(blocked.status()).toBe(503);
      } finally {
        await locker.query('ROLLBACK');
        locker.release();
        await db.end();
      }

      await refreshProjection();
      const stepUpDb = pool();
      const publisher = await stepUpDb.connect();
      const stepUpPath = '{admin_control_step_up,step_up_at}';
      const [original] = (await stepUpDb.query(
        `SELECT (data#>>$2::text[])::bigint AS verified_at FROM web_sessions
         WHERE account_id=$1 AND data#>>$2::text[] IS NOT NULL`,
        [gm.account.id, stepUpPath],
      )).rows;
      expect(original.verified_at).toBeTruthy();
      let waiting;
      try {
        await publisher.query('BEGIN');
        await publisher.query('SELECT singleton FROM support_enforcement_counter FOR UPDATE');
        const [{ pid }] = (await publisher.query('SELECT pg_backend_pid() AS pid')).rows;
        const [{ deadline }] = (await stepUpDb.query(
          `UPDATE web_sessions SET data=jsonb_set(data,$2::text[],
             to_jsonb(floor(extract(epoch FROM clock_timestamp()))::bigint-298))
           WHERE account_id=$1 AND data#>>$2::text[] IS NOT NULL
           RETURNING (data#>>$2::text[])::bigint+300 AS deadline`,
          [gm.account.id, stepUpPath],
        )).rows;
        waiting = post(page, early.target, { ...early.fields, ...valid });
        await expect.poll(async () => (await stepUpDb.query(
          `SELECT EXISTS(SELECT 1 FROM pg_stat_activity
             WHERE query='SELECT 1 FROM support_enforcement_counter WHERE singleton FOR UPDATE'
               AND $1 = ANY (pg_blocking_pids(pid))) AS waiting`,
          [pid],
        )).rows[0].waiting, { intervals: [20, 40, 80], timeout: 2500 }).toBe(true);
        await expect.poll(async () => (await stepUpDb.query(
          'SELECT extract(epoch FROM clock_timestamp()) >= $1::bigint AS expired', [deadline],
        )).rows[0].expired, { intervals: [20, 40, 80], timeout: 2500 }).toBe(true);
      } finally {
        await publisher.query('ROLLBACK');
        publisher.release();
        try {
          if (waiting) await refused(await waiting, 403, text.stepUp);
        } finally {
          await stepUpDb.query(
            `UPDATE web_sessions SET data=jsonb_set(data,$2::text[],to_jsonb($3::bigint))
             WHERE account_id=$1 AND data#>>$2::text[] IS NOT NULL`,
            [gm.account.id, stepUpPath, original.verified_at],
          );
          await stepUpDb.end();
        }
      }
      expect(await query(
        'SELECT 1 FROM audit_log WHERE request_id=$1', [early.fields.operation],
      )).toEqual([]);
      expect(await query(
        'SELECT 1 FROM support_enforcement_commands WHERE operation_id=$1', [early.fields.operation],
      )).toEqual([]);

      const competitor = await seededAccount(lang);
      const snapshot = {
        ...snapshotHead, operation: 'PublishAccountCharactersV1', account_id: competitor.id,
        projection_revision: '1', source_observed_at: sourceTime(),
        characters: [{ character_id: character, world_id: world, name, availability: 'AVAILABLE' }],
      };
      await gameJson(lcfa, portfolio, snapshot);
      const conflict = await lcfa(portfolio, {
        ...snapshot, characters: [{ ...snapshot.characters[0], name: 'Conflicting character' }],
      });
      expect(conflict.status).toBe(409);
      await refreshProjection();
      expect((await page.request.get(url)).status()).toBe(503);
      expect((await postFresh(page, early.target, { ...early.fields, ...valid })).status()).toBe(503);
      await gameJson(lcfa, portfolio, {
        ...snapshot, projection_revision: '2', source_observed_at: sourceTime(), characters: [],
      });
    }

    for (const minutes of ['0', '1441']) {
      await refused(await postFresh(page, early.target, { ...early.fields, ...valid, minutes }), 400, text.invalid);
    }
    await refused(await postFresh(page, early.target, { ...early.fields, ...valid, reason: '' }), 400, text.invalid);
    expect((await postFresh(page, early.target, { ...early.fields, ...valid, _csrf: 'forged' })).status()).toBe(403);
    const missing = early.target.replace(character, uuid7());
    await refused(await postFresh(page, missing, { ...early.fields, ...valid }), 404, text.missing);
    expect(await query('SELECT 1 FROM support_enforcement_commands WHERE operation_id=$1', [early.fields.operation])).toEqual([]);

    await reloadDossier();
    const first = await formPayload(form('apply'));
    await submitFresh(form('apply'), valid, path);
    await expect(status()).toHaveText(text.pending);
    await expect(current()).toHaveText(text.unconfirmed);
    await expectAccountEligible(true);
    expect((await postFresh(page, first.target, { ...first.fields, ...valid })).status()).toBe(303);
    await refused(await postFresh(page, first.target, { ...first.fields, ...valid, minutes: '16' }), 409, text.conflict);
    await refused(await postFresh(page, early.target, { ...early.fields, ...valid }), 409, text.conflict);
    const common = { account_id: target.id, character_id: character, profile };
    const initial = await readGame(first.fields.operation, {
      ...common, intent: 'APPLY', decision_revision: 1, prior_decision_revision: null, reason_code: 7,
    });
    await initial.publish({ state: 'APPLIED' });
    const applied = initial.command;
    expect(applied.expires_at_ms - applied.effective_at_ms).toBe(15 * 60_000);

    await reloadDossier();
    await expect(status()).toHaveText(text.applied);
    await expect(current()).toHaveText(text.applied);
    await expect(panel.locator('time[data-receipt]')).toHaveAttribute('datetime', /T\d{2}:\d{2}:\d{2}\.\d{3}Z$/);

    const refusedReplace = await formPayload(form('replace'));
    const staleRevoke = await formPayload(form('revoke'));
    await submitFresh(form('replace'), { ...valid, minutes: '30' }, path);
    await expect(status()).toHaveText(text.pending);
    await expect(current()).toHaveText(text.applied);
    const rejected = await readGame(refusedReplace.fields.operation, {
      ...common, intent: 'REPLACE', decision_revision: 2, prior_decision_revision: 1,
      sanction_stream_id: applied.sanction_stream_id,
    });
    await rejected.publish({
      state: 'REJECTED', result_code: 'LIFECYCLE_CONFLICT', accepted_decision_revision: 1,
      effective_until_ms: applied.expires_at_ms, runtime_effect: 'NONE',
    });
    await reloadDossier();
    await expect(status()).toHaveText(text.refused);
    await expect(current()).toHaveText(text.applied);
    await expect(panel.getByText('LIFECYCLE_CONFLICT', { exact: true })).toBeVisible();
    await expectAccountEligible(true);
    await refused(await postFresh(page, staleRevoke.target, {
      ...staleRevoke.fields, reason: 'rule_violation',
    }), 409, text.conflict);

    const replace = await formPayload(form('replace'));
    await submitFresh(form('replace'), { minutes: '30', reason: 'rule_violation' }, path);
    const replaced = await readGame(replace.fields.operation, {
      ...common, intent: 'REPLACE', decision_revision: 3, prior_decision_revision: 1, reason_code: 8,
      sanction_stream_id: applied.sanction_stream_id,
    });
    await replaced.publish({ state: 'ACCEPTED_PENDING', result_code: 'PENDING_RUNTIME' });
    await reloadDossier();
    await expect(status()).toHaveText(text.pending);
    await expect(current()).toHaveText(text.applied);
    await expect(panel.locator('time[data-current-receipt]')).toBeVisible();
    const appliedUntil = await panel.locator('time[data-effective-until]').getAttribute('datetime');
    expect(new Date(appliedUntil).toISOString()).toBe(new Date(applied.expires_at_ms).toISOString());
    await replaced.publish({ receipt_revision: 2, state: 'REPLACED' });
    expect(replaced.command.expires_at_ms - replaced.command.effective_at_ms).toBe(30 * 60_000);
    await reloadDossier();
    await expect(status()).toHaveText(text.applied);
    await expect(status()).toHaveAttribute('data-operation', replace.fields.operation);

    const revoke = await formPayload(form('revoke'));
    await submitFresh(form('revoke'), { reason: 'rule_violation' }, path);
    await expect(status()).toHaveText(text.pending);
    await expect(current()).toHaveText(text.applied);
    const revoking = await readGame(revoke.fields.operation, {
      ...common, intent: 'REVOKE', decision_revision: 4, prior_decision_revision: 3,
      sanction_stream_id: applied.sanction_stream_id, effective_at_ms: null, expires_at_ms: null,
    });
    await revoking.publish({
      state: 'ACCEPTED_PENDING', result_code: 'PENDING_RUNTIME', stream_state: 'REVOKED',
      runtime_effect: 'NONE',
    });
    await reloadDossier();
    await expect(status()).toHaveText(text.pending);
    await expect(current()).toHaveText(text.applied);
    await expect(panel.locator('time[data-current-receipt]')).toBeVisible();
    await revoking.publish({ receipt_revision: 2, state: 'REVOKED', stream_state: 'REVOKED' });
    await reloadDossier();
    await expect(status()).toHaveText(text.revoked);
    await expect(current()).toHaveText(text.revoked);
    await expect(panel.locator('time[data-receipt]')).toHaveAttribute('datetime', /T\d{2}:\d{2}:\d{2}\.\d{3}Z$/);

    const audit = await query(
      "SELECT a.id,a.actor,a.target,a.before,a.after FROM audit_log a WHERE action='game.enforcement.requested' AND request_id=$1",
      [first.fields.operation],
    );
    expect(audit).toEqual([{
      id: applied.audit_ref, actor: gm.account.id, target: applied.sanction_stream_id, before: null,
      after: {
        operation_id: first.fields.operation, character_id: character, profile, intent: 'APPLY',
        reason_code: 7, duration_minutes: 15, form_version: 0, decision_revision: 1,
        sanction_stream_id: applied.sanction_stream_id, state: 'pending',
        effective_until_ms: applied.expires_at_ms,
      },
    }]);
    const decisions = await query(
      `SELECT d.operation_id,d.decision_revision::integer,d.intent,d.profile,d.character_id,
              s.account_id,s.status,a.owner_id,n.kind,n.reason_code,n.revoked
       FROM support_enforcement_decisions d
       JOIN sanctions s ON s.id=d.sanction_id
       JOIN support_appeal_workflows a ON a.sanction_id=s.id
       JOIN support_sanction_export_revisions n
         ON n.sanction_stream_id=s.id AND n.decision_revision=d.decision_revision
       WHERE s.id=$1 ORDER BY d.decision_revision`,
      [applied.sanction_stream_id],
    );
    expect(decisions).toEqual([
      [first.fields.operation, 'APPLY', 7, false],
      [refusedReplace.fields.operation, 'REPLACE', 7, false],
      [replace.fields.operation, 'REPLACE', 8, false],
      [revoke.fields.operation, 'REVOKE', 8, true],
    ].map(([operation_id, intent, reason_code, revoked], index) => ({
      operation_id, intent, reason_code, revoked, decision_revision: index + 1, profile,
      character_id: character, account_id: target.id, owner_id: target.id,
      status: 'revoked', kind: profile === 'MUTE' ? 1 : 2,
    })));
    const operations = await query(
      'SELECT operation_id FROM support_enforcement_commands WHERE character_id=$1 AND profile=$2 ORDER BY position',
      [character, profile],
    );
    expect(operations.map(row => row.operation_id)).toEqual([
      first.fields.operation, refusedReplace.fields.operation, replace.fields.operation, revoke.fields.operation,
    ]);
  }
  await query(
    `INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by)
     VALUES($1,'restriction','active','Account restriction',clock_timestamp(),clock_timestamp()+interval '1 hour',$2)`,
    [target.id, gm.account.id],
  );
  await expectAccountEligible(false);
});
