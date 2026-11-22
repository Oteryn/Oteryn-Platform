import { createHash, randomUUID } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { expect } from '../fixtures.mjs';
import { password, pool } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';

export const sessionCookie = '__Host-oteryn_session';
export const deviceCookie = '__Host-oteryn_mfa_device';
export const emailAddress = () => `auth-${randomUUID()}@example.invalid`;
export const hash = value => createHash('sha256').update(value).digest();

export async function seedEntity(entity, input) {
  await pool().end();
  const binary = process.env.PORTAL_E2E_BINARY;
  if (!binary || process.env.OTERYN_ENV !== 'testing' ||
      !process.env.OTERYN_DATABASE_URL ||
      process.env.OTERYN_DATABASE_URL !== process.env.PORTAL_E2E_DATABASE_URL) {
    throw new Error('Use the same portal binary and disposable testing database as the browser launcher');
  }
  const seeded = spawnSync(binary, ['seed', entity, JSON.stringify(input)], {
    encoding: 'utf8',
  });
  expect(seeded.status, `seed CLI completed ${entity}`).toBe(0);
  return JSON.parse(seeded.stdout);
}

export async function seedAccount(locale) {
  const account = { email: emailAddress(), password: password(), locale };
  const seeded = await seedEntity('account', account);
  return { ...account, id: seeded.accountId };
}

export async function cookie(context, name = sessionCookie) {
  const result = (await context.cookies()).find(entry => entry.name === name);
  expect(Boolean(result), `${name} exists`).toBe(true);
  expect({ secure: result.secure, httpOnly: result.httpOnly, sameSite: result.sameSite,
    path: result.path, domain: result.domain }).toEqual({
    secure: true, httpOnly: true, sameSite: 'Lax', path: '/', domain: 'localhost',
  });
  return result.value;
}

export async function expectSession(value, account) {
  const [row] = await query('SELECT account_id FROM web_sessions WHERE id_hash=$1', [hash(value)]);
  expect(row?.account_id).toBe(account);
}

export async function otherBrowser(browser, info) {
  return browser.newContext({
    baseURL: info.project.use.baseURL,
    viewport: info.project.use.viewport,
    locale: info.project.use.locale,
    ignoreHTTPSErrors: true,
  });
}

export async function postForm(page, selector, values) {
  const form = page.locator(selector);
  const action = await form.getAttribute('action') || new URL(page.url()).pathname;
  const target = new URL(action, page.url());
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  return page.request.post(target.href, {
    form: { ...fields, ...values },
    headers: { origin: target.origin },
    maxRedirects: 0,
  });
}
