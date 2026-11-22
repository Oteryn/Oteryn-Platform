import { randomUUID } from 'node:crypto';
import { spawn, spawnSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { once } from 'node:events';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { createServer } from 'node:net';
import { request as httpRequest } from 'node:http';
import { createServer as createHttpsServer } from 'node:https';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createInterface } from 'node:readline';
import { test as base, expect } from '../fixtures.mjs';
import { csrfRejected, login, password, pool } from '../helpers.mjs';
import { query as databaseQuery } from '../staff-rbac-helpers.mjs';
import { withDeadline } from '../support/harness-deadline.mjs';

// Reserve the loopback ports together so the child never reuses one
// of its own ports. The harness verifies readiness on the new listeners.
async function puzzlePorts() {
  const servers = [];
  try {
    for (let index = 0; index < 5; index++) {
      const server = createServer();
      servers.push(server);
      server.listen(0, '127.0.0.1');
      await once(server, 'listening');
    }
    return servers.map(server => server.address().port);
  } finally {
    await Promise.all(servers.map(server => new Promise(resolve => server.close(resolve))));
  }
}

const test = base.extend({
  puzzleApp: [async ({}, use) => {
    const [http, internal, admission, gateway, https] = await puzzlePorts();
    const origin = `https://localhost:${https}`;
    const scratch = await mkdtemp(join(tmpdir(), 'portal-e2e-puzzles-'));
    const output = join(scratch, 'run');
    const child = spawn(process.execPath, ['harness.mjs', '--puzzle-browser'], {
      cwd: new URL('../', import.meta.url),
      env: {
        ...process.env,
        PORTAL_E2E_OUT: output,
        PORTAL_E2E_KEEP_DATABASE: '0',
        PORTAL_E2E_SEED_PLAN: '',
        PORTAL_E2E_HTTP_PORT: String(http),
        PORTAL_E2E_INTERNAL_PORT: String(internal),
        PORTAL_E2E_ADMISSION_PORT: String(admission),
        PORTAL_E2E_GATEWAY_PORT: String(gateway),
        OTERYN_PUBLIC_ORIGIN: origin,
        PORTAL_GAME_RESTORE_NOTICE_ENABLED: 'false',
      },
      detached: true,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const exited = once(child, 'close');
    const lines = createInterface({ input: child.stdout });
    const diagnostics = createInterface({ input: child.stderr });
    let database = 'unallocated';
    diagnostics.on('line', line => {
      // The harness logs the name before allocating it. Keep credentials out
      // of Playwright output, including on startup and teardown failures.
      const match = /^\[harness\] database (portal_e2e_[a-f0-9]{12})$/.exec(line);
      if (match) database = match[1];
    });
    let proxy;
    try {
      const ready = await Promise.race([
        once(lines, 'line', { signal: AbortSignal.timeout(60_000) }).then(([line]) => line),
        exited.then(([code, signal]) => { throw new Error(`Puzzle harness stopped before readiness (${code ?? signal}) for database ${database}`); }),
      ]);
      const report = JSON.parse(ready);
      if (report.status !== 'ready' || !/^portal_e2e_[a-f0-9]{12}$/.test(report.database)) {
        throw new Error('Puzzle harness did not report a disposable database');
      }
      database = report.database;
      const env = JSON.parse(await readFile(join(output, 'secrets', 'env.json'), 'utf8'));
      const databaseURL = env.OTERYN_DATABASE_URL;
      if (!databaseURL || new URL(databaseURL).pathname !== `/${database}`
        || [process.env.OTERYN_DATABASE_URL, process.env.PORTAL_E2E_DATABASE_URL].some(
        shared => shared && new URL(shared).pathname === new URL(databaseURL).pathname,
      )) throw new Error('Puzzle harness must supply a separate database');
      proxy = createHttpsServer({
        cert: await readFile(env.OTERYN_INTERNAL_TLS_CERT_FILE),
        key: await readFile(env.OTERYN_INTERNAL_TLS_KEY_FILE),
      }, (req, res) => {
        const upstream = httpRequest({
          hostname: '127.0.0.1', port: http, path: req.url, method: req.method, headers: req.headers,
        }, response => {
          res.writeHead(response.statusCode, response.headers);
          response.pipe(res);
        });
        upstream.on('error', () => res.writeHead(502).end());
        req.pipe(upstream);
      });
      proxy.listen(https, '127.0.0.1');
      await once(proxy, 'listening');
      await use({ origin, databaseURL, env });
    } finally {
      if (proxy) {
        proxy.close();
        proxy.closeAllConnections();
      }
      try {
        child.kill('SIGTERM');
        // Allow the harness its service and database budgets plus exit reporting.
        const [code, signal] = await withDeadline(exited, 34_000, 'harness shutdown timed out')
          .catch(async error => {
            if (child.pid !== undefined && child.exitCode === null && child.signalCode === null) {
              // Kill the whole fixture process group if the harness cannot
              // finish its own bounded cleanup.
              try {
                process.kill(-child.pid, 'SIGKILL');
              } catch (killError) {
                if (killError.code !== 'ESRCH') throw killError;
              }
              await withDeadline(exited, 2_000, 'harness did not exit after SIGKILL');
            }
            throw error;
          });
        if (code !== 0) throw new Error(`harness exited with ${code ?? signal}`);
      } catch (error) {
        throw new Error(`Puzzle harness cleanup failed for database ${database} (${error.code ?? error.message})`);
      } finally {
        lines.close();
        diagnostics.close();
        await rm(scratch, { recursive: true, force: true });
      }
    }
  }, { timeout: 100_000 }],
  baseURL: async ({ puzzleApp }, use) => use(puzzleApp.origin),
});

const worldVersion = '019a0000-0000-7000-8000-000000000022';
const publicationId = '019a0000-0000-7000-8000-000000000023';
const kind = 'puzzle-e2e-creature';
const question = { en: 'Which creature has pointed ears?', pl: 'Które zwierzę ma spiczaste uszy?' };

function cli(appEnv, args, overrides, status, error) {
  const binary = process.env.PORTAL_E2E_BINARY;
  expect(Boolean(binary), 'launcher exposes its portal binary').toBe(true);
  const result = spawnSync(binary, args, {
    env: { ...appEnv, ...overrides }, encoding: 'utf8',
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

function seedAccount(appEnv, locale) {
  const email = `puzzle-${randomUUID()}@example.invalid`;
  const secret = password();
  const { accountId } = cli(appEnv, ['seed', 'account', JSON.stringify({ email, password: secret, locale })], {}, 0);
  return { id: accountId, email, password: secret };
}

async function fixture(databaseURL) {
  const db = pool(databaseURL);
  const conn = await db.connect();
  try {
    await conn.query('BEGIN');
    const { rows: [active] } = await conn.query(
      'SELECT version_id FROM world_active_version WHERE singleton FOR UPDATE',
    );
    // The isolated app starts with an empty catalogue.
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

test('an operator publishes a daily quiz and a player can solve it once', async ({ page, puzzleApp }, info) => {
  const lang = info.project.metadata.locale;
  const route = `/${lang}/puzzles`;
  const { databaseURL, env } = puzzleApp;
  await fixture(databaseURL);
  const query = (sql, values) => databaseQuery(sql, values, databaseURL);
  const invoke = (args, overrides, status, error) => cli(env, args, overrides, status, error);
  const operator = seedAccount(env, lang);
  const player = seedAccount(env, lang);
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
  const audits = () => query("SELECT actor, request_id FROM audit_log WHERE action='puzzles.publish' AND target=$1", [publicationId]);
  const publicationState = async () => ({
    claims: await query('SELECT id, document_sha256 FROM puzzles_publications WHERE id=$1', [publicationId]),
    slots: await query("SELECT id, publication_id FROM puzzles_daily WHERE publication_id=$1 OR (day=$2 AND kind='quiz') ORDER BY id", [publicationId, day]),
    audit: await audits(),
  });
  const before = await publicationState();
  expect(before).toEqual({ audit: [], claims: [], slots: [] });

  writeFileSync(file, JSON.stringify(document), { mode: 0o600 });
  invoke(publish(player.id), {}, 2, 'Denied');
  expect(await publicationState()).toEqual(before);
  invoke(publish(operator.id), { OTERYN_DAILY_PUZZLES_ENABLED: '' }, 2, 'Disabled');
  expect(await publicationState()).toEqual(before);

  expect(await query(
    'UPDATE accounts SET email_verified_at=NULL WHERE id=$1 RETURNING status,email_verified_at',
    [operator.id],
  )).toEqual([{ status: 'active', email_verified_at: null }]);
  invoke(publish(operator.id), {}, 2, 'Denied');
  expect(await publicationState()).toEqual(before);
  await query('UPDATE accounts SET email_verified_at=now() WHERE id=$1', [operator.id]);

  const invalid = structuredClone(document);
  invalid.puzzles[0].definition.correct_choice = 6;
  writeFileSync(file, JSON.stringify(invalid));
  invoke(publish(operator.id), {}, 2, 'InvalidDocument');
  expect(await publicationState()).toEqual(before);

  writeFileSync(file, JSON.stringify(document));
  expect(invoke(publish(operator.id), {}, 0)).toEqual({ publicationId, replayed: false });
  const activated = await publicationState();
  expect(activated.claims).toHaveLength(1);
  expect(activated.slots).toEqual([{ id: expect.any(String), publication_id: publicationId }]);
  expect(invoke(publish(operator.id), {}, 0)).toEqual({ publicationId, replayed: true });
  expect(await publicationState()).toEqual(activated);
  expect(activated.audit).toEqual([{ actor: operator.id, request_id: publicationId }]);

  const guest = await page.request.get(route, { maxRedirects: 0 });
  expect(guest.status()).toBe(303);
  expect(guest.headers().location).toBe(`/${lang}/login`);
  await login(page, lang, player, databaseURL);
  const opened = await page.goto(route);
  expect(opened.status()).toBe(200);
  expect(opened.headers()['cache-control']).toContain('no-store');
  await expect(page.getByText(question[lang], { exact: true })).toBeVisible();
  const form = page.locator('main form:has(select[name=choice])');
  const action = await form.getAttribute('action');
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const answers = () => query(
    'SELECT choice,correct FROM puzzles_answers WHERE account_id=$1 AND puzzle_id=$2',
    [player.id, action.split('/').at(-2)],
  );
  expect(await answers()).toEqual([]);
  await csrfRejected(page, action, { choice: '0' });
  expect(await answers()).toEqual([]);
  const post = choice => page.request.post(action, {
    form: { ...fields, choice }, headers: { origin: new URL(page.url()).origin }, maxRedirects: 0,
  });
  expect((await post('6')).status()).toBe(400);
  expect(await answers()).toEqual([]);
  await form.locator('select').selectOption({ label: lang === 'pl' ? 'Lis' : 'Fox' });
  const [answer] = await Promise.all([
    page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname === action),
    form.locator('button[type=submit]').click(),
  ]);
  expect(answer.status()).toBe(303);
  expect(answer.headers().location).toBe(route);
  await expect(page.locator('main [role=status]')).toHaveText(lang === 'pl' ? 'Poprawna odpowiedź.' : 'Correct answer.');
  const solved = [{ choice: 0, correct: true }];
  expect(await answers()).toEqual(solved);
  expect((await post('1')).status()).toBe(409);
  expect(await answers()).toEqual(solved);
});
