import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import { localPostgresUrl } from '../postgres-url.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { password as generatePassword } from '../helpers.mjs';

export { query };
export const portal = fileURLToPath(new URL('../../', import.meta.url));
export const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'portal-cli-'));
const out = path.join(scratch, 'harness');
const harness = fileURLToPath(new URL('../harness.mjs', import.meta.url));
const suppliedBinary = process.env.PORTAL_E2E_BINARY;
let child;
let childClosed;
let environment;
const credentials = [];

export async function start() {
  const adminURL = localPostgresUrl(process.env.PORTAL_E2E_ADMIN_DATABASE_URL);
  child = spawn(process.execPath, [harness], {
    env: {
      ...process.env,
      PORTAL_E2E_ADMIN_DATABASE_URL: adminURL.href,
      PORTAL_E2E_OUT: out,
      PORTAL_E2E_KEEP_DATABASE: '0',
      PORTAL_E2E_SEED_PLAN: '',
      OTERYN_ENV: 'testing',
      RUST_LOG: 'info',
    },
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  childClosed = once(child, 'close');
  const lines = createInterface({ input: child.stdout });
  try {
    const [line] = await Promise.race([
      once(lines, 'line'),
      childClosed.then(([code]) => { throw new Error(`Harness exited before readiness (${code})`); }),
    ]);
    const report = JSON.parse(line);
    assert.equal(report.status, 'ready');
    environment = JSON.parse(fs.readFileSync(path.join(out, 'secrets/env.json'), 'utf8'));
    const database = localPostgresUrl(environment.OTERYN_DATABASE_URL);
    assert.match(report.database, /^portal_e2e_[a-f0-9]{12}$/);
    const expected = new URL(`/${report.database}`, adminURL);
    assert.ok(database.href === expected.href, 'Harness uses the exact disposable connection');
    environment.OTERYN_DATABASE_URL = database.href;
    credentials.push(database.href, database.password, decodeURIComponent(database.password),
      environment.OTERYN_MFA_KEY,
      fs.readFileSync(environment.OTERYN_PLATFORM_SERVICE_TOKEN_FILE, 'utf8').trim());
    process.env.PORTAL_E2E_DATABASE_URL = database.href;
  } finally {
    lines.close();
  }
}

export async function stop() {
  try {
    if (child) {
      child.kill('SIGTERM');
      const [code, signal] = await childClosed;
      if (environment) {
        assert.equal(signal, null, 'Harness exits normally');
        assert.equal(code, 0, 'Harness shuts down cleanly');
      }
    }
  } finally {
    fs.rmSync(scratch, { recursive: true, force: true });
  }
}

export function run(args, code = 0, { env, input, emptyEnv = false } = {}) {
  const label = args[0];
  const result = spawnSync(suppliedBinary || path.join(out, 'bin/oteryn-portal'), args, {
    cwd: portal, env: emptyEnv ? {} : { ...environment, ...env }, input, encoding: 'utf8', timeout: 60_000,
  });
  assert.ok(!result.error, `${label} executes within the process deadline`);
  assert.ok(!containsCredential(result.stderr) && !containsCredential(result.stdout),
    `${label} output excludes configured secrets`);
  assert.equal(result.signal, null, `${label} exits normally`);
  assert.equal(result.status, code, `${label} exit status`);
  return result;
}

export function success(args, options) {
  const result = run(args, 0, options);
  assert.ok(result.stderr === '', 'Successful command has empty stderr');
  return result.stdout;
}

export function json(text) {
  try { return JSON.parse(text); } catch { throw new Error('Expected one JSON response'); }
}

export function refused(args, code, error, reason, options, stream = 'stderr') {
  const result = run(args, code, options);
  assert.ok(result[stream === 'stderr' ? 'stdout' : 'stderr'] === '',
    'Refusal has no output on the opposite stream');
  const body = json(result[stream]);
  if (args[0] === 'seed' && args[1] === 'account-mfa') {
    assert.ok(!hasMfaMaterial(body), 'MFA refusal excludes enrollment material');
  }
  assert.ok(isDeepStrictEqual(body, reason === undefined ? { error } : { error, reason }),
    `Unexpected refusal response for ${args[0]}`);
}

function containsCredential(text) {
  return credentials.some(secret => secret && text.includes(secret));
}

function hasMfaMaterial(value) {
  if (typeof value === 'string') return containsCredential(value);
  return value !== null && typeof value === 'object' &&
    Object.entries(value).some(([key, child]) =>
      ['totpsecret', 'recoverycodes'].includes(key.replaceAll(/[_-]/g, '').toLowerCase()) ||
      containsCredential(key) || hasMfaMaterial(child));
}

export function seed(entity, args) {
  const body = json(success(['seed', entity, JSON.stringify(args)]));
  if (entity === 'account-mfa') {
    for (const value of [body.totpSecret, ...(Array.isArray(body.recoveryCodes) ? body.recoveryCodes : [])]) {
      if (typeof value === 'string' && value) credentials.push(value);
    }
  }
  return body;
}

export function seedRefused(entity, args, code, error, reason, options) {
  refused(['seed', entity, JSON.stringify(args)], code, error, reason, options, 'stdout');
}

export async function id() {
  return (await query('SELECT uuidv7() AS id'))[0].id;
}

export function password() {
  const value = generatePassword();
  credentials.push(value);
  return value;
}

export async function account() {
  const accountId = await id();
  const value = password();
  const email = `cli-${accountId}@example.invalid`;
  const args = { accountId, email, password: value, locale: 'en' };
  assert.equal(seed('account', args).accountId, accountId);
  return args;
}

export async function grant(accountId, permissions) {
  // Same database RBAC fixture boundary as the harness staff tests.
  const role = await id();
  await query(`WITH role AS (
    INSERT INTO roles (id,key,built_in) VALUES ($1,$2,false) RETURNING id
  ), permissions AS (
    INSERT INTO role_permissions (role_id,permission)
    SELECT id, unnest($3::text[]) FROM role
  )
  INSERT INTO account_roles (account_id,role_id) SELECT $4,id FROM role`,
  [role, `cli_${role.replaceAll('-', '')}`, permissions, accountId]);
}

export function audit(target) {
  return query('SELECT actor,action,before,after,request_id FROM audit_log WHERE target=$1 ORDER BY action', [target]);
}
