import { execFile } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import { expect } from './fixtures.mjs';
import { csrfRejected, fixtureDatabaseEnv, password, waitForLoginBudget } from './helpers.mjs';
import { query } from './staff-rbac-helpers.mjs';
import { revealPortalLink } from './journey-navigation.mjs';

export async function newActor(browser, info, storageState) {
  const context = await browser.newContext({
    viewport: info.project.use.viewport,
    locale: info.project.metadata.locale,
    baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443',
    ignoreHTTPSErrors: true,
    storageState,
  });
  context.setDefaultTimeout(info.project.use.actionTimeout);
  context.setDefaultNavigationTimeout(info.project.use.navigationTimeout);
  return { context, page: await context.newPage() };
}

const runSeed = promisify(execFile);

async function seed(entity, fields) {
  const env = fixtureDatabaseEnv();
  const target = process.env.CARGO_TARGET_DIR || fileURLToPath(new URL('../target', import.meta.url));
  const binary = process.env.PORTAL_E2E_BINARY || path.join(target, 'debug', 'oteryn-portal');
  try {
    const { stdout } = await runSeed(binary, ['seed', entity, JSON.stringify(fields)], { env });
    return JSON.parse(stdout);
  } catch {
    // CLI arguments and output can contain disposable credentials.
    throw new Error(`Could not seed ${entity}`);
  }
}

// Seed verified identities and MFA before the story. Sign in via the real
// forms in provisioning, then hand only browser storage to each journey actor.
export async function provision(browser, info, role, mfa = Boolean(role)) {
  const lang = info.project.metadata.locale;
  const account = { email: `journey-${randomUUID()}@example.invalid`, password: password() };
  const { accountId: id } = await seed('account', { ...account, locale: lang });
  if (role) {
    const grants = await query(
      'INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2 RETURNING role_id',
      [id, role],
    );
    expect(grants).toHaveLength(1);
  }
  const factors = mfa ? await seed('account-mfa', { accountId: id, password: account.password }) : null;
  const { context, page } = await newActor(browser, info);
  try {
    await page.goto(`/${lang}/login`, { waitUntil: 'commit' });
    await waitForLoginBudget();
    await clickForm(page, page.locator('main form'), account);
    if (mfa) {
      await expect(page).toHaveURL(/mfa\/challenge/);
      await clickForm(page, page.locator('main form[action$="/mfa/challenge"]'), {
        code: factors.recoveryCodes[0],
      });
      if (!role) {
        await page.goto(`/${lang}/account/sudo`, { waitUntil: 'commit' });
        await clickForm(page, page.locator('main form'), { current_password: account.password });
      }
    }
    // Mandatory login redirects and session rotation finish before copying
    // storage into the actor's long-lived journey context.
    await expect(page).not.toHaveURL(/\/(?:login|mfa\/challenge|account\/sudo)(?:\?|$)/);
    return { id, email: account.email, state: await context.storageState() };
  } finally {
    await context.close();
  }
}

export async function follow(page, path) {
  // The destination can render its own navigation just after the response.
  // Wait for it before opening the portal menu, which overlays links on mobile.
  await page.waitForLoadState('domcontentloaded');
  const link = await revealPortalLink(page, `a[href="${path}"]`);
  return navigate(page, link, path);
}

async function navigate(page, link, path) {
  const [response] = await Promise.all([
    page.waitForResponse(response => response.request().isNavigationRequest()
      && new URL(response.url()).pathname === new URL(path, page.url()).pathname),
    link.click(),
  ]);
  return response;
}

export async function fill(form, values) {
  for (const [name, value] of Object.entries(values)) {
    const control = form.locator(`[name="${name}"]`);
    if (await control.evaluate(element => element.tagName === 'SELECT')) {
      await control.selectOption(value);
    } else if (typeof value === 'boolean') {
      await control.setChecked(value);
    } else {
      await control.fill(value);
    }
  }
}

export async function formPayload(form) {
  return form.evaluate(element => Object.fromEntries(new FormData(element)));
}

export function postForm(page, target, form) {
  return page.request.post(target, {
    form,
    headers: { origin: new URL(page.url()).origin },
    maxRedirects: 0,
  });
}

export async function clickForm(page, form, fields = {}, button = form.locator('button').first()) {
  await fill(form, fields);
  const target = await form.getAttribute('action') || new URL(page.url()).pathname;
  const payload = await formPayload(form);
  const name = await button.getAttribute('name');
  if (name) payload[name] = await button.getAttribute('value');
  await csrfRejected(page, target, payload);
  const [response] = await Promise.all([
    page.waitForResponse(response => response.request().isNavigationRequest()
      && response.request().method() === 'POST'
      && new URL(response.url()).pathname === new URL(target, page.url()).pathname),
    button.click({ noWaitAfter: true }),
  ]);
  return response;
}

export async function submitAndReplay(page, form, changedField) {
  const target = await form.getAttribute('action');
  const fields = await formPayload(form);
  await csrfRejected(page, target, fields);
  const isSubmit = request => request.isNavigationRequest()
    && request.method() === 'POST' && new URL(request.url()).pathname === target;
  const [navigation, replay] = await Promise.all([
    page.waitForResponse(response => isSubmit(response.request())),
    page.waitForRequest(isSubmit).then(() => postForm(page, target, fields)),
    form.locator('button').click(),
  ]);
  // Exact duplicates are acknowledged idempotently. Reusing the key for a
  // different action must conflict, even when the changed input is valid.
  expect(navigation.status()).toBe(303);
  expect(replay.status()).toBe(303);
  expect(replay.headers().location).toBe(navigation.headers().location);
  expect((await postForm(page, target, { ...fields, ...changedField })).status()).toBe(409);
  return fields;
}
