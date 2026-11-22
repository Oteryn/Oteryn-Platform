import { expect } from '@playwright/test';
import { randomUUID } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { csrfRejected, login, password, pool } from './helpers.mjs';
import { loginWithRecoveryCode, query } from './staff-rbac-helpers.mjs';

const portal = fileURLToPath(new URL('../', import.meta.url));

export async function actor(browser, info, role, use, { mfa = Boolean(role) } = {}) {
  const locale = info.project.metadata.locale;
  const context = await browser.newContext({
    baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443',
    ignoreHTTPSErrors: true,
    viewport: info.project.use.viewport,
    locale,
  });
  try {
    const page = await context.newPage();
    const database = pool();
    await database.end();
    const account = { email: `support-${randomUUID()}@example.invalid`, password: password() };
    const binary = process.env.PORTAL_E2E_BINARY ||
      resolve(process.env.CARGO_TARGET_DIR || resolve(portal, 'target'), 'debug/oteryn-portal');
    const seed = (entity, values) => {
      const result = spawnSync(binary, ['seed', entity, JSON.stringify(values)], {
        encoding: 'utf8',
        env: {
          ...process.env,
          OTERYN_ENV: 'testing',
          OTERYN_DATABASE_URL: process.env.PORTAL_E2E_DATABASE_URL || process.env.OTERYN_DATABASE_URL,
        },
      });
      expect(result.status, `seed CLI creates a disposable ${entity} fixture`).toBe(0);
      return JSON.parse(result.stdout);
    };
    account.id = seed('account', { ...account, locale }).accountId;
    if (role) {
      const grants = await query(
        'INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2 RETURNING role_id',
        [account.id, role],
      );
      expect(grants).toHaveLength(1);
    }
    if (mfa) {
      const { recoveryCodes } = seed('account-mfa', { accountId: account.id, password: account.password });
      await loginWithRecoveryCode(page, locale, account, recoveryCodes[0]);
      await expect(page).not.toHaveURL(/mfa\/challenge/);
    } else {
      await login(page, locale, account);
    }
    await use(page, account);
  } finally {
    await context.close();
  }
}

export function copy(area, locale) {
  const paths = {
    tickets: `support/src/tickets/${locale}.ftl`,
    reports: `support/src/reports/${locale}.ftl`,
    moderation: `admin/src/support_${locale}.ftl`,
    players: `admin/src/players/${locale}.ftl`,
  };
  const source = readFileSync(resolve(portal, 'crates', paths[area]), 'utf8');
  return Object.fromEntries(source.trim().split('\n').map(line => line.split(' = ')));
}

export async function open(page, path, status = 200) {
  expect((await page.goto(path)).status(), path).toBe(status);
}

export async function submitForm(form, values = {}, { button = 'button', bad, method = 'POST' } = {}) {
  const page = form.page();
  const path = await form.getAttribute('action');
  for (const [name, value] of Object.entries(values)) {
    const field = form.locator(`[name="${name}"]`);
    if (typeof value === 'boolean') await field.setChecked(value);
    else if (await field.evaluate(element => element.tagName) === 'SELECT') await field.selectOption(value);
    else await field.fill(value);
  }
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const control = form.locator(button);
  const name = await control.getAttribute('name');
  if (name) fields[name] = await control.getAttribute('value');
  if (method === 'POST') await csrfRejected(page, path, fields);
  else {
    const { _csrf, ...unsigned } = fields;
    await mutation(page, path, unsigned, 403, method);
  }
  if (bad) await mutation(page, path, { ...fields, ...bad }, bad.expected ?? 422, method);
  if (method === 'POST') await control.click();
  else await mutation(page, path, fields, 302, method);
}

export async function mutation(page, path, fields, expected, method = 'POST') {
  const { expected: _, ...form } = fields;
  const origin = new URL(page.url()).origin;
  const response = await page.request.fetch(new URL(path, origin).href, {
    method, form, headers: { origin }, maxRedirects: 0,
  });
  expect(response.status(), `${method} ${path}`).toBe(expected);
  return response;
}

export async function labels(page, strings) {
  for (const string of strings) {
    expect(string).toBeTruthy();
    await expect(page.locator('main')).toContainText(string);
  }
}
