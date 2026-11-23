import { execFile } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { promisify } from 'node:util';
import { test as portal, expect } from '../fixtures.mjs';
import { password, pool } from '../helpers.mjs';

export { expect };
export const digest = token => createHash('sha256').update(token).digest();
export const text = (lang, en, pl) => lang === 'en' ? en : pl;

// Leave enough time for a browser round trip and password hashing. Each fixture
// moves only its own row. Product checks and the worker still use the real clock.
export const before = 5;
export const after = -1;

export const test = portal.extend({
  lang: async ({}, use, info) => use(info.project.metadata.locale),
  db: async ({}, use) => {
    const db = pool();
    try {
      await use(db);
    } finally {
      await db.end();
    }
  },
  seedEntity: async ({ db }, use) => {
    await use(async (entity, args) => {
      // The shared pool enforces the local disposable database allowlist.
      const connection = db.options.connectionString;
      const binary = process.env.PORTAL_E2E_BINARY || process.env.PORTAL_E2E_OUTBOX_BINARY;
      if (!binary) throw new Error('Run expiry through the portal E2E launcher');
      let result;
      try {
        result = await promisify(execFile)(binary, ['seed', entity, JSON.stringify(args)], {
          env: { ...process.env, OTERYN_ENV: 'testing', OTERYN_DATABASE_URL: connection },
        });
      } catch {
        throw new Error(`Portal seed failed for ${entity}`);
      }
      return JSON.parse(result.stdout);
    });
  },
  seed: async ({ lang, seedEntity }, use) => {
    await use(async () => {
      const account = { email: `expiry-${randomUUID()}@example.invalid`, password: password() };
      const { accountId: id, status } = await seedEntity('account', { ...account, locale: lang });
      expect(id).toMatch(/^[\da-f-]{36}$/);
      expect(status).toBe('active');
      return { ...account, id };
    });
  },
});

export async function one(db, sql, args) {
  const { rows } = await db.query(sql, args);
  expect(rows).toHaveLength(1);
  return rows[0];
}

export async function sweep(db, kind, read, expected) {
  // Schedule an extra run of the existing worker handler so the suite needs
  // neither a clock override nor a wait for the hourly schedule.
  const { id } = await one(db,
    "INSERT INTO outbox(kind,payload,idempotency_key) VALUES($1,'{}',$2) RETURNING id",
    [kind, `expiry:${randomUUID()}`]);
  await expect.poll(async () => one(db, 'SELECT state,last_error FROM outbox WHERE id=$1', [id]))
    .toEqual({ state: 'done', last_error: null });
  expect(await read()).toEqual(expected);
}

export async function formPost(page, selector, values = {}, headers = {}) {
  const form = page.locator(selector);
  const action = new URL(await form.evaluate(element =>
    new URL(element.getAttribute('action') || '', document.baseURI).href));
  const origin = new URL(page.url()).origin;
  expect(action.origin === origin && ['localhost', '127.0.0.1', '[::1]'].includes(action.hostname),
    'credential form posts to the originating loopback portal').toBe(true);
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  return page.request.post(action.href, {
    form: { ...fields, ...values },
    headers: { origin, ...headers },
    maxRedirects: 0,
  });
}

export function redirected(response, status, location) {
  expect(response.status()).toBe(status);
  expect(response.headers().location).toBe(location);
}
