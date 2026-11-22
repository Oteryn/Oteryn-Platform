import {randomUUID} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {expect} from '@playwright/test';
import {fixtureDatabaseURL, password, submit, waitForLoginBudget} from './helpers.mjs';

export function seedPlayer(locale) {
  const credentials = {email: `account-${randomUUID()}@example.invalid`, password: password()};
  const database = fixtureDatabaseURL();
  const target = process.env.CARGO_TARGET_DIR || fileURLToPath(new URL('../target', import.meta.url));
  const binary = process.env.PORTAL_E2E_BINARY || resolve(target, 'debug/oteryn-portal');
  try {
    const result = execFileSync(binary, ['seed', 'account', JSON.stringify({...credentials, locale})], {
      stdio: ['ignore', 'pipe', 'ignore'],
      encoding: 'utf8',
      env: {...process.env, OTERYN_ENV: 'testing', OTERYN_DATABASE_URL: database},
    });
    return {...credentials, id: JSON.parse(result).accountId};
  } catch {
    throw new Error('Portal account seed failed. Check that the local harness binary and migrated database are ready.');
  }
}

export async function signIn(page, lang, player) {
  await page.goto(`/${lang}/login`, {waitUntil: 'domcontentloaded'});
  await waitForLoginBudget();
  await submit(page, 'main form', {email: player.email, password: player.password});
  await expect(page).not.toHaveURL(new RegExp(`/${lang}/login(?:[/?]|$)`));
}

export async function switchPlayer(page, lang, player) {
  await page.context().clearCookies();
  await signIn(page, lang, player);
}

export async function sudo(page, lang, player) {
  await page.goto(`/${lang}/account/sudo`, {waitUntil: 'domcontentloaded'});
  await submit(page, 'main form', {current_password: player.password});
}

export async function post(page, target, fields, form = page.locator('main form').first()) {
  return page.request.post(target, {
    form: {_csrf: await form.locator('[name=_csrf]').inputValue(), ...fields},
    headers: {origin: new URL(page.url()).origin},
    maxRedirects: 0,
  });
}

export async function privatePage(page, target, status = 200) {
  const response = await page.goto(target, {waitUntil: 'domcontentloaded'});
  expect(response.status(), target).toBe(status);
  expect(response.headers()['cache-control']).toContain('no-store');
  return response;
}

export async function strings(main, catalog, keys) {
  const content = readFileSync(new URL(`../${catalog}`, import.meta.url), 'utf8');
  const messages = new Map(content.split('\n').flatMap(line => {
    const match = /^([\w-]+) = (.+)$/.exec(line);
    return match ? [[match[1], match[2]]] : [];
  }));
  for (const key of keys) {
    const message = messages.get(key);
    expect(message, `${catalog} message ${key}`).toBeTruthy();
    await expect(main).toContainText(message);
  }
}

export async function refusesAnonymous(page, target, status, location) {
  const response = await page.request.get(target, {maxRedirects: 0});
  expect(response.status(), target).toBe(status);
  if (location) expect(response.headers().location).toBe(location);
}
