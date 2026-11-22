import { randomUUID } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login, password, pool } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';

const worldVersion = '019a0000-0000-7000-8000-000000000022';
const publicationId = '019a0000-0000-7000-8000-000000000023';
const kind = 'puzzle-e2e-creature';
const question = { en: 'Which creature has pointed ears?', pl: 'Które zwierzę ma spiczaste uszy?' };

function cli(args, overrides, status, error) {
  const binary = process.env.PORTAL_E2E_BINARY;
  expect(Boolean(binary), 'launcher exposes its portal binary').toBe(true);
  const result = spawnSync(binary, args, {
    env: { ...process.env, ...overrides }, encoding: 'utf8',
  });
  // Never include the subprocess arguments or its captured streams in a failure.
  expect(result.status, 'portal CLI exit status').toBe(status);
  let payload;
  try {
    payload = JSON.parse(status === 0 ? result.stdout : result.stderr);
  } catch {
    throw new Error('Portal CLI returned invalid JSON');
  }
  if (error) expect(payload.error).toBe(error);
  return payload;
}

function seedAccount(locale) {
  const email = `puzzle-${randomUUID()}@example.invalid`;
  const secret = password();
  const { accountId } = cli(['seed', 'account', JSON.stringify({ email, password: secret, locale })], {}, 0);
  return { id: accountId, email, password: secret };
}

async function fixture() {
  const db = pool();
  const conn = await db.connect();
  try {
    await conn.query('BEGIN');
    const { rows: [active] } = await conn.query(
      'SELECT version_id FROM world_active_version WHERE singleton FOR UPDATE',
    );
    // Dedicated disposable catalogue fixture. Keep native world tests cold.
    expect(active.version_id).toBeNull();
    await conn.query(
      "INSERT INTO world_versions (id, source_commit, status) VALUES ($1, repeat('a', 40), 'ready') ON CONFLICT DO NOTHING",
      [worldVersion],
    );
    for (const key of ['puzzle-fox', 'puzzle-wolf']) {
      await conn.query(
        "INSERT INTO world_entities (version_id,kind,key,data,publishable) VALUES ($1,$2,$3,'{}',true) ON CONFLICT DO NOTHING",
        [worldVersion, kind, key],
      );
      await conn.query('INSERT INTO world_visibility (kind,key,allowed) VALUES ($1,$2,true)', [kind, key]);
    }
    await conn.query('UPDATE world_active_version SET version_id=$1 WHERE singleton', [worldVersion]);
    await conn.query('COMMIT');
  } catch (error) {
    await conn.query('ROLLBACK');
    throw error;
  } finally {
    conn.release();
    await db.end();
  }
}

test('an operator publishes a daily quiz and a player can solve it once', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const route = `/${lang}/puzzles`;
  await fixture();
  let operator;
  try {
    operator = seedAccount(lang);
    const player = seedAccount(lang);
    await query(
      "INSERT INTO account_roles (account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin' AND built_in",
      [operator.id],
    );
    const [{ day }] = await query("SELECT to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD') AS day");
    const document = {
      publication_id: publicationId, day,
      puzzles: [{ definition: {
        kind: 'quiz', world_version: worldVersion, question,
        clue: { en: 'Look for a small woodland animal.', pl: 'Poszukaj małego leśnego zwierzęcia.' },
        choices: [
          { label: { en: 'Fox', pl: 'Lis' }, source: { kind, key: 'puzzle-fox' } },
          { label: { en: 'Wolf', pl: 'Wilk' }, source: { kind, key: 'puzzle-wolf' } },
        ],
        correct_choice: 0,
      } }],
    };
    const file = info.outputPath('publication.json');
    const publish = actor => ['admin-cli', 'puzzles', 'publish', file, '--actor', actor];
    writeFileSync(file, JSON.stringify(document), { mode: 0o600 });
    cli(publish(player.id), {}, 2, 'Denied');
    cli(publish(operator.id), { OTERYN_DAILY_PUZZLES_ENABLED: '' }, 2, 'Disabled');
    const audits = () => query("SELECT actor, request_id FROM audit_log WHERE action='puzzles.publish' AND target=$1", [publicationId]);
    const before = await audits();
    expect(before.length).toBeLessThanOrEqual(1);

    const invalid = structuredClone(document);
    invalid.puzzles[0].definition.correct_choice = 6;
    writeFileSync(file, JSON.stringify(invalid));
    cli(publish(operator.id), {}, 2, 'InvalidDocument');
    expect(await audits()).toEqual(before);

    writeFileSync(file, JSON.stringify(document));
    expect(cli(publish(operator.id), {}, 0)).toEqual({ publicationId, replayed: before.length === 1 });
    expect(cli(publish(operator.id), {}, 0)).toEqual({ publicationId, replayed: true });
    expect(await audits()).toEqual(before.length ? before : [{ actor: operator.id, request_id: publicationId }]);

    const guest = await page.request.get(route, { maxRedirects: 0 });
    expect(guest.status()).toBe(303);
    expect(guest.headers().location).toBe(`/${lang}/login`);
    await login(page, lang, player);
    const opened = await page.goto(route);
    expect(opened.status()).toBe(200);
    expect(opened.headers()['cache-control']).toContain('no-store');
    await expect(page.getByText(question[lang], { exact: true })).toBeVisible();
    const form = page.locator('main form:has(select[name=choice])');
    const action = await form.getAttribute('action');
    const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
    await csrfRejected(page, action, { choice: '0' });
    const post = choice => page.request.post(action, {
      form: { ...fields, choice }, headers: { origin: new URL(page.url()).origin }, maxRedirects: 0,
    });
    expect((await post('6')).status()).toBe(400);
    await form.locator('select').selectOption({ label: lang === 'pl' ? 'Lis' : 'Fox' });
    const [answer] = await Promise.all([
      page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname === action),
      form.locator('button[type=submit]').click(),
    ]);
    expect(answer.status()).toBe(303);
    expect(answer.headers().location).toBe(route);
    await expect(page.locator('main [role=status]')).toHaveText(lang === 'pl' ? 'Poprawna odpowiedź.' : 'Correct answer.');
    expect((await post('1')).status()).toBe(409);
    expect(await query(
      'SELECT choice,correct FROM puzzles_answers WHERE account_id=$1 AND puzzle_id=$2',
      [player.id, action.split('/').at(-2)],
    )).toEqual([{ choice: 0, correct: true }]);
  } finally {
    if (operator) await query('DELETE FROM account_roles WHERE account_id=$1', [operator.id]);
    await query('UPDATE world_active_version SET version_id=NULL WHERE singleton AND version_id=$1', [worldVersion]);
    await query('DELETE FROM world_visibility WHERE kind=$1', [kind]);
  }
});
