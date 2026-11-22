import { randomUUID } from 'node:crypto';
import { execFile, spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { createInterface } from 'node:readline';
import { resolve } from 'node:path';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';
import { expect } from '@playwright/test';
import { csrfRejected, fixtureDatabaseURL, login, password } from './helpers.mjs';

const exec = promisify(execFile);
const portalRoot = fileURLToPath(new URL('../', import.meta.url));
const binary = process.env.PORTAL_E2E_BINARY ??
  resolve(portalRoot, process.env.CARGO_TARGET_DIR ?? 'target', 'debug/oteryn-portal');

async function seed(entity, args, databaseURL) {
  databaseURL = fixtureDatabaseURL(databaseURL);
  try {
    const { stdout } = await exec(binary, ['seed', entity, JSON.stringify(args)], {
      env: { ...process.env, OTERYN_DATABASE_URL: databaseURL },
    });
    return JSON.parse(stdout);
  } catch {
    throw new Error(`The disposable portal seed CLI failed for ${entity}`);
  }
}

export async function seedPlayer(locale, databaseURL) {
  const email = `commerce-${randomUUID()}@example.invalid`;
  const pass = password();
  const { accountId: id } = await seed('account', { email, password: pass, locale }, databaseURL);
  return { email, password: pass, id };
}

export async function seedMfa(player) {
  const { recoveryCodes } = await seed('account-mfa', { accountId: player.id, password: player.password });
  return recoveryCodes[0];
}

export async function signInPlayer(page, lang, databaseURL) {
  const player = await seedPlayer(lang, databaseURL);
  await login(page, lang, player, databaseURL);
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


export async function withUnpublishedDownloads(browser, locale, check) {
  const adminURL = fixtureDatabaseURL();
  const portNames = ['HTTP', 'INTERNAL', 'ADMISSION', 'GATEWAY'];
  const listeners = portNames.map(() => createServer());
  let ports;
  try {
    ports = await Promise.all(listeners.map(async server => {
      server.listen(0, '127.0.0.1');
      await once(server, 'listening');
      return server.address().port;
    }));
  } finally {
    await Promise.all(listeners.map(server => new Promise(resolve => server.close(resolve))));
  }

  const scratch = await mkdtemp(resolve(tmpdir(), 'portal-e2e-downloads-'));
  const out = resolve(scratch, 'harness');
  const origin = `http://localhost:${ports[0]}`;
  const child = spawn(process.execPath, [resolve(portalRoot, 'e2e/harness.mjs'), '--serve-only'], {
    env: {
      ...process.env,
      ...Object.fromEntries(portNames.map((name, i) => [`PORTAL_E2E_${name}_PORT`, String(ports[i])])),
      PORTAL_E2E_ADMIN_DATABASE_URL: adminURL,
      PORTAL_E2E_BINARY: binary,
      PORTAL_E2E_OUT: out,
      PORTAL_E2E_KEEP_DATABASE: '0',
      PORTAL_E2E_SEED_PLAN: '',
      OTERYN_PUBLIC_ORIGIN: origin,
      OTERYN_ACCOUNT_LOGIN: 'true',
      OTERYN_DOWNLOADS_ENABLED: 'true',
    },
    stdio: ['ignore', 'pipe', 'ignore'],
  });
  const closed = once(child, 'close');
  const lines = createInterface({ input: child.stdout });
  try {
    const [line] = await Promise.race([
      once(lines, 'line', { signal: AbortSignal.timeout(120_000) }),
      closed.then(([code]) => { throw new Error(`Download harness exited during startup (${code})`); }),
    ]);
    const report = JSON.parse(line);
    expect(report.status).toBe('ready');
    const env = JSON.parse(await readFile(resolve(out, 'secrets/env.json'), 'utf8'));
    const databaseURL = fixtureDatabaseURL(env.OTERYN_DATABASE_URL);
    const context = await browser.newContext({ baseURL: origin, locale });
    try {
      const page = await context.newPage();
      await signInPlayer(page, locale, databaseURL);
      await check(page);
    } finally {
      await context.close();
    }
  } finally {
    lines.close();
    if (child.exitCode === null) child.kill('SIGTERM');
    await closed;
    await rm(scratch, { recursive: true, force: true });
  }
}
