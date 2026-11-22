import { randomUUID } from 'node:crypto';
import { execFile } from 'node:child_process';
import { resolve } from 'node:path';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';
import { expect } from '@playwright/test';
import { csrfRejected, login, password, pool } from './helpers.mjs';

const exec = promisify(execFile);
const portalRoot = fileURLToPath(new URL('../', import.meta.url));
const binary = process.env.PORTAL_E2E_BINARY ??
  resolve(portalRoot, process.env.CARGO_TARGET_DIR ?? 'target', 'debug/oteryn-portal');

async function seed(entity, args) {
  const db = pool();
  const databaseURL = db.options.connectionString;
  await db.end();
  try {
    const { stdout } = await exec(binary, ['seed', entity, JSON.stringify(args)], {
      env: { ...process.env, OTERYN_DATABASE_URL: databaseURL },
    });
    return JSON.parse(stdout);
  } catch {
    throw new Error(`The disposable portal seed CLI failed for ${entity}`);
  }
}

export async function seedPlayer(locale) {
  const email = `commerce-${randomUUID()}@example.invalid`;
  const pass = password();
  const { accountId: id } = await seed('account', { email, password: pass, locale });
  return { email, password: pass, id };
}

export async function seedMfa(player) {
  const { recoveryCodes } = await seed('account-mfa', { accountId: player.id, password: player.password });
  return recoveryCodes[0];
}

export async function signInPlayer(page, lang) {
  const player = await seedPlayer(lang);
  await login(page, lang, player);
  return player;
}

export async function readForm(form) {
  return form.evaluate(element => ({
    url: new URL(element.getAttribute('action') || element.ownerDocument.URL, element.baseURI).href,
    fields: Object.fromEntries(new FormData(element)),
  }));
}

export async function postAs(page, url, fields) {
  return page.request.post(url, {
    form: fields,
    headers: { origin: new URL(url).origin },
    maxRedirects: 0,
  });
}

export async function pressCheckout(page, action, label) {
  const form = page.locator('main form[action*="/shop/test-checkout/"]');
  const { url, fields } = await readForm(form);
  await csrfRejected(page, url, { ...fields, action });
  const button = form.locator(`button[value="${action}"]`);
  await expect(button).toHaveText(label);
  await button.click();
}
