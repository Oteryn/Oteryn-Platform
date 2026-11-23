import { expect } from '@playwright/test';
import { spawnSync } from 'node:child_process';
import crypto from 'node:crypto';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { password, pool } from './helpers.mjs';

export async function seedEntity(entity, args) {
  const db = pool();
  const connectionString = db.options.connectionString;
  await db.end();
  const target = process.env.CARGO_TARGET_DIR || fileURLToPath(new URL('../target', import.meta.url));
  const binary = process.env.PORTAL_E2E_BINARY || path.join(target, 'debug', 'oteryn-portal');
  const result = spawnSync(binary, ['seed', entity, JSON.stringify(args)], {
    env: { ...process.env, OTERYN_DATABASE_URL: connectionString },
    encoding: 'utf8',
    timeout: 30_000,
    stdio: ['ignore', 'pipe', 'ignore'],
  });
  // The account-MFA result contains disposable credentials. Keep output private
  // even on failure by asserting only the exit code and parsing in memory.
  expect(result.status, `seed CLI command ${entity}`).toBe(0);
  try {
    return JSON.parse(result.stdout);
  } catch {
    throw new Error(`seed CLI command ${entity} returned invalid JSON`);
  }
}

export async function seedAccount(locale) {
  const account = { email: `browser-seed-${crypto.randomUUID()}@example.invalid`, password: password() };
  const { accountId } = await seedEntity('account', { ...account, locale });
  return { ...account, accountId };
}
