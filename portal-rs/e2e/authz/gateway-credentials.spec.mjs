import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomBytes } from 'node:crypto';
import { once } from 'node:events';
import { mkdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test } from '@playwright/test';
import { assertRefusalState, privateDatabase, privateEnvironment } from './database.mjs';

const hash = value => createHash('sha256').update(value).digest();
const secret = () => randomBytes(32).toString('base64url');
const handle = () => `otr1.${secret()}`;
const admissionBudget = { table: 'native_admission_budgets', key: hash('native-admission-gateway').toString('hex') };
const offer = {
  client_build: 'local-authz', client_platform: 'windows',
  transports: [{ protocol_major: 1, transport_profile: 1, alpn: 'oteryn-game/1' }],
};

function checkedRun(binary, args, env) {
  try {
    return execFileSync(binary, args, { env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] });
  } catch {
    throw new Error('Could not provision the disposable gateway fixture');
  }
}

async function stop(child) {
  if (child.pid && child.exitCode === null && child.signalCode === null) {
    const exited = once(child, 'exit');
    child.kill('SIGINT');
    await exited;
  }
}

test('enabled gateway checks login tickets and recovery handles', async ({ request }) => {
  test.setTimeout(120_000);
  const reportPath = process.env.PORTAL_E2E_HARNESS_REPORT;
  const report = JSON.parse(readFileSync(reportPath, 'utf8'));
  const base = Number(process.env.PORTAL_E2E_AUTHZ_GATEWAY_PORT_BASE || '18950');
  const binary = process.env.PORTAL_E2E_BINARY;
  const custody = process.env.PORTAL_E2E_AUTHZ_RECOVERY_CUSTODY_BINARY;
  if (!binary || !custody || !Number.isSafeInteger(base) || base < 1024 || base > 65532) {
    throw new Error('Provide the local Portal and recovery custody binaries and four unused loopback ports');
  }
  const env = privateEnvironment(reportPath, report);
  const directory = join(dirname(reportPath), 'secrets', 'authz-recovery-custody');
  mkdirSync(directory, { recursive: true, mode: 0o700 });
  const bind = offset => `127.0.0.1:${base + offset}`;
  const origin = `http://${bind(3)}`;
  const profile = {
    ...env,
    OTERYN_BIND: bind(0),
    OTERYN_INTERNAL_TLS_BIND: bind(1),
    PORTAL_NATIVE_ADMISSION_BIND: bind(2),
    OTERYN_PLATFORM_BASE_URL: `http://${bind(2)}`,
    GATEWAY_LISTEN_ADDR: bind(3),
    GATEWAY_NATIVE_PROTOCOL3_ENABLED: 'true',
    PORTAL_NATIVE_PROTOCOL3_ENABLED: 'true',
    PORTAL_NATIVE_RECOVERY_ENABLED: 'true',
    PORTAL_NATIVE_RECOVERY_REQUIRE_DEVICE_BINDING: 'false',
    PORTAL_NATIVE_ADMISSION_GRANTS_ENABLED: 'true',
    PORTAL_NATIVE_RECOVERY_KEY_CUSTODY_FILE: join(directory, 'recovery-custody.bin'),
    PORTAL_NATIVE_RECOVERY_KEY_WRAPPING_KEY_FILE: join(directory, 'recovery-wrapping.key'),
  };
  const children = [];
  const db = privateDatabase(reportPath, report);
  try {
    checkedRun(custody, [directory], profile);
    for (const role of ['serve', 'gateway']) {
      const child = spawn(binary, [role], {
        cwd: fileURLToPath(new URL('../../', import.meta.url)), env: profile, stdio: 'ignore',
      });
      children.push(child);
      await once(child, 'spawn');
    }
    await expect.poll(async () => {
      if (children.some(child => child.exitCode !== null)) throw new Error('The enabled gateway profile exited');
      try {
        const response = await request.get(`${origin}/ready`, { timeout: 1500 });
        try { return response.status(); } finally { await response.dispose(); }
      } catch (error) {
        if (/ECONNREFUSED|timeout/i.test(error.message)) return 0;
        throw error;
      }
    }, { timeout: 30_000 }).toBe(200);

    const { accountId } = JSON.parse(checkedRun(binary, ['seed', 'account', JSON.stringify({
      email: `authz-gateway-${randomBytes(12).toString('hex')}@example.invalid`,
      password: `Local!${secret()}`, locale: 'en',
    })], profile));
    const [{ character, world, channel }] = await db.query(
      'SELECT uuidv7() AS character, uuidv7() AS world, uuidv7() AS channel');
    const ticket = secret();
    const validHandle = handle();
    const unknownHandle = handle();
    await db.query(`
      INSERT INTO game_login_tickets(ticket_hash,account_id,game_generation,native_generation,audience,expires_at)
      SELECT $1,id,game_auth_generation::text,native_security_generation::text,'oteryn-native-game-gateway',
        clock_timestamp()+interval '5 minutes' FROM accounts WHERE id=$2
    `, [hash(ticket), accountId]);
    await db.query('INSERT INTO native_play_generations(account_id,generation) VALUES($1,1)', [accountId]);
    await db.query(`
      INSERT INTO native_recovery_handles(handle_hash,account_id,character_id,world_id,channel_id,
        security_generation,play_generation,attempt_ref,issued_at,expires_at)
      SELECT $1,id,$3,$4,$5,native_security_generation,1,uuidv7(),
        floor(extract(epoch FROM clock_timestamp()))::bigint,
        floor(extract(epoch FROM clock_timestamp()))::bigint+300 FROM accounts WHERE id=$2
    `, [hash(validHandle), accountId, character, world, channel]);

    async function probe(path, data, status, code, unchanged = true) {
      await test.step(`POST ${path} returns ${code ?? status}`, async () => {
        const before = unchanged ? await db.snapshot(undefined, admissionBudget) : null;
        const response = await request.post(origin + path, {
          data, headers: { 'content-type': 'application/json' }, maxRedirects: 0,
        });
        try {
          const body = await response.json();
          expect(response.status(), body.error?.code).toBe(status);
          if (code) expect(body.error?.code).toBe(code);
          else expect(body).toEqual({ protocol_version: 3, revoked: true });
          if (before) assertRefusalState(before, await db.snapshot(undefined, admissionBudget));
        } finally {
          await response.dispose();
        }
      });
    }
    async function login(credential, status, code) {
      const [{ id }] = await db.query('SELECT uuidv7() AS id');
      await probe('/v1/login', { protocol_version: 2, game_login_ticket: credential,
        attempt_ref: id, character_id: character, channel_id: null, offer }, status, code);
    }
    async function recover(credential, status, code) {
      const [{ id }] = await db.query('SELECT uuidv7() AS id');
      await probe('/v1/recover', { protocol_version: 3, recovery_handle: credential,
        attempt_ref: id, character_id: character, offer }, status, code);
    }
    const revoke = (credential, unchanged) => probe('/v1/recover/revoke',
      { protocol_version: 3, recovery_handle: credential }, 200, null, unchanged);

    // The local account has no Game route. A valid credential must get beyond
    // authentication and reach the separately qualified route-availability fence.
    await login(secret(), 401, 'NATIVE_LOGIN_AUTHENTICATION_REQUIRED');
    await login(ticket, 503, 'NATIVE_LOGIN_ROUTE_UNAVAILABLE');
    await recover(unknownHandle, 401, 'NATIVE_LOGIN_AUTHENTICATION_REQUIRED');
    await recover(validHandle, 503, 'NATIVE_RECOVERY_ROUTE_UNAVAILABLE');
    // Unknown handles return the same response as live ones to prevent disclosure.
    await revoke(unknownHandle, true);
    await revoke(validHandle, false);
    const [row] = await db.query('SELECT revoked_at IS NOT NULL AS revoked FROM native_recovery_handles WHERE handle_hash=$1',
      [hash(validHandle)]);
    expect(row).toEqual({ revoked: true });
    await revoke(validHandle, true);
    await recover(validHandle, 401, 'NATIVE_LOGIN_AUTHENTICATION_REQUIRED');
  } finally {
    await Promise.all(children.map(stop));
    await db.close();
  }
});
