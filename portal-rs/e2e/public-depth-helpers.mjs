import { execFile } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { promisify } from 'node:util';
import { expect } from './fixtures.mjs';
import { disposableDatabaseUrl, login, password, waitForWikiBudget } from './helpers.mjs';

import { enrollMfa, loginWithRecoveryCode, query } from './staff-rbac-helpers.mjs';

const run = promisify(execFile);
export const missingId = '019a0000-0000-7000-8000-00000000deed';

export function messages(directory, lang) {
  const source = readFileSync(new URL(`../${directory}/${lang}.ftl`, import.meta.url), 'utf8');
  return Object.fromEntries([...source.matchAll(/^([\w-]+) = ([^{}\r\n]+)$/gm)]
    .map(([, key, value]) => [key, value.trim()]));
}

export async function visit(page, path, status, { lang, message, noStore = true } = {}) {
  await waitForWikiBudget(path);
  const response = await page.goto(path, { waitUntil: "domcontentloaded" });
  expect(response.status(), path).toBe(status);
  if (noStore) expect(response.headers()['cache-control'], path).toContain('no-store');
  if (lang) await expect(page.locator('html')).toHaveAttribute('lang', lang);
  if (message !== undefined) {
    expect(message, path).toBeTruthy();
    await expect(page.locator('main'), path).toContainText(message);
  }
  return response;
}

// Exercise read-only query permutations without fetching the same browser assets.
export async function probe(page, path, status, { lang, message, empty } = {}) {
  await waitForWikiBudget(path);
  const response = await page.request.get(path, { headers: { accept: 'text/html' } });
  expect(response.status(), path).toBe(status);
  expect(response.headers()['cache-control'], path).toContain('no-store');
  if (lang || message || empty) {
    expect(response.headers()['content-type'], path).toContain('text/html');
    const result = await page.evaluate(({ html, empty }) => {
      const document = new DOMParser().parseFromString(html, 'text/html');
      return {
        lang: document.documentElement.lang,
        text: document.querySelector('main')?.textContent?.replace(/\s+/g, ' ').trim(),
        count: empty ? document.querySelectorAll(empty).length : 0,
      };
    }, { html: await response.text(), empty });
    if (lang) expect(result.lang, path).toBe(lang);
    if (message !== undefined) {
      expect(message, path).toBeTruthy();
      expect(result.text, path).toContain(message);
    }
    if (empty) expect(result.count, path).toBe(0);
  }
}

export async function redirect(request, path, location, status = 303) {
  const response = await request.get(path, { maxRedirects: 0 });
  expect(response.status(), path).toBe(status);
  expect(response.headers().location, path).toBe(location);
}

async function seed(entity, input) {
  const databaseUrl = disposableDatabaseUrl();
  const binary = process.env.PORTAL_E2E_BINARY;
  if (!binary) throw new Error('The E2E launcher must supply PORTAL_E2E_BINARY');
  // SQLx reads PGHOSTADDR and other libpq defaults before applying the URL.
  const env = {
    OTERYN_ENV: 'testing',
    OTERYN_DATABASE_URL: databaseUrl,
  };
  const result = await run(binary, ['seed', entity, JSON.stringify(input)], { env })
    .catch(error => { throw new Error(`Seed CLI failed for ${entity} (exit ${error.code})`); });
  return JSON.parse(result.stdout);
}

export async function seedPlayer(locale) {
  const email = `public-depth-${crypto.randomUUID()}@example.invalid`;
  const pass = password();
  const result = await seed('account', { email, password: pass, locale });
  expect(result.status).toBe('active');
  return { email, password: pass };
}

export async function seedWorld() {
  const worldId = `019a0000-0000-7000-8000-${crypto.randomUUID().slice(-12)}`;
  const name = `Registry ${worldId.slice(-12)}`;
  await seed('world', { worldId, name, region: 'local' });
  return { id: worldId, name };
}

export async function seedEditor(browser, info) {
  const lang = info.project.metadata.locale;
  const context = await browser.newContext({
    baseURL: process.env.PORTAL_E2E_ORIGIN ?? 'https://localhost:18443',
    ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport,
  });
  try {
    const page = await context.newPage();
    const account = await seedPlayer(lang);
    // Bootstrap the local actor using the same role grant as the staff harness.
    const rows = await query(
      "INSERT INTO account_roles(account_id,role_id) SELECT a.id,r.id FROM accounts a CROSS JOIN roles r WHERE a.email=$1 AND r.key='platform_admin' RETURNING account_id",
      [account.email],
    );
    expect(rows).toHaveLength(1);
    await login(page, lang, account);
    await enrollMfa(page, lang, account);
    const recoveryCode = page.locator("textarea[readonly]").first();
    await expect(recoveryCode).toBeVisible();
    await loginWithRecoveryCode(page, lang, account, await recoveryCode.inputValue());
    return { context, page };
  } catch (error) {
    await context.close();
    throw error;
  }
}
