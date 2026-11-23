import { createHmac, randomBytes, randomInt } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { test as base, expect } from '@playwright/test';

export { expect, delay };
const origin = 'https://staging.oteryn.com';
const host = new URL(origin).host;
const accessHeaders = () => ({
  'CF-Access-Client-Id': process.env.STAGING_CF_ACCESS_CLIENT_ID,
  'CF-Access-Client-Secret': process.env.STAGING_CF_ACCESS_CLIENT_SECRET,
});

async function routeStaging(context) {
  await context.route('**/*', async route => {
    const request = route.request();
    const url = new URL(request.url());
    if (url.host !== host || url.protocol !== 'https:') {
      await route.continue();
      return;
    }
    try {
      // Keep the injected headers on this request even if staging redirects.
      const response = await route.fetch({
        headers: { ...await request.allHeaders(), ...accessHeaders() },
        maxRedirects: 0,
        timeout: 30_000,
      });
      try {
        const location = response.headers().location;
        if (location && response.status() >= 300 && response.status() < 400 &&
            new URL(location, url).origin !== origin && request.isNavigationRequest()) {
          await route.abort();
        } else {
          await route.fulfill({ response });
        }
      } finally {
        await response.dispose();
      }
    } catch {
      await route.abort().catch(() => {});
    }
  });
}
const inbox = '/__mailpit/api/v1/messages';
const runId = randomBytes(5).toString('hex');
let sequence = 0;

export const test = base.extend({
  context: async ({ context, request }, use) => {
    await expect.poll(async () => {
      const response = await safeGet(request, '/readyz');
      const status = response?.status();
      await response?.dispose();
      return status;
    }, { timeout: 300_000, intervals: [1000, 5000, 10_000], message: 'Staging /readyz returns 200' }).toBe(200);
    await routeStaging(context);
    await use(context);
  },
  isolatedPage: async ({ browser }, use) => {
    const contexts = [];
    try {
      await use(async () => {
        const isolated = await browser.newContext();
        contexts.push(isolated);
        await routeStaging(isolated);
        return isolated.newPage();
      });
    } finally {
      for (const isolated of contexts) await isolated.close();
    }
  },
});

export function identity() {
  return {
    email: `sim-lockedout-${runId}-${++sequence}@example.test`,
    password: password(),
  };
}

export function password() {
  return `Sim!${randomBytes(18).toString('hex')}`;
}

export async function safeGet(request, path) {
  const url = new URL(path, origin);
  if (url.origin !== origin) throw new Error('Staging API request has a different origin');
  try {
    return await request.get(url.href, { headers: accessHeaders(), maxRedirects: 0, timeout: 10_000 });
  } catch {
    // Playwright's raw API error can include request headers.
    return null;
  }
}

export async function open(page, path) {
  const url = new URL(path, origin);
  if (url.origin !== origin) throw new Error('Portal navigation has a different origin');
  try {
    return await page.goto(url.href, { waitUntil: 'domcontentloaded' });
  } catch (error) {
    const code = error.message.match(/net::ERR_[A-Z_]+/)?.[0];
    throw new Error(code ? `Portal navigation failed (${code})` : 'Portal navigation failed');
  }
}

export const form = route => `main form[action$="${route}"]`;

export async function submit(page, selector, fields, { timeout = 20_000, double = false } = {}) {
  const target = page.locator(selector);
  for (const [name, value] of Object.entries(fields)) {
    const input = target.locator(`[name="${name}"]`);
    if (typeof value === 'boolean') await input.setChecked(value);
    else {
      try {
        await input.fill(value);
      } catch {
        throw new Error(`Could not fill the ${name} field`);
      }
    }
  }
  const action = new URL(await target.getAttribute('action'), page.url());
  expect(action.origin).toBe(origin);
  const button = target.locator('button').first();
  try {
    const [response] = await Promise.all([
      page.waitForResponse(response => response.url() === action.href &&
        response.request().method() === 'POST', { timeout }),
      page.waitForEvent('framenavigated', { predicate: frame => frame === page.mainFrame(), timeout }),
      double ? button.dblclick() : button.click(),
    ]);
    await page.waitForLoadState('domcontentloaded');
    return response;
  } catch {
    throw new Error('The portal form submission did not complete');
  }
}

export async function mailAvailable(request) {
  const response = await safeGet(request, inbox);
  const status = response?.status();
  await response?.dispose();
  test.skip(status === 404 || status === 405, `Staging Mailpit GET returned HTTP ${status}`);
  expect(status, 'Mailpit list status').toBe(200);
}

export async function messages(request, email, since) {
  const response = await safeGet(request, inbox);
  expect(response?.status(), 'Mailpit list status').toBe(200);
  try {
    const body = await response.json();
    expect(Array.isArray(body.messages), 'Mailpit messages array').toBe(true);
    return body.messages.sort((a, b) => Date.parse(b.Created) - Date.parse(a.Created))
      .slice(0, 100)
      .filter(message => Date.parse(message.Created) >= since &&
        message.To?.some(to => to.Address.toLowerCase() === email.toLowerCase()));
  } finally {
    await response.dispose();
  }
}

export async function mailLink(request, email, since, pattern, subject) {
  let link;
  let created;
  await expect.poll(async () => {
    const message = (await messages(request, email, since))
      .find(item => !subject || item.Subject === subject);
    if (!message) return false;
    const response = await safeGet(request, `/__mailpit/api/v1/message/${encodeURIComponent(message.ID)}`);
    expect(response?.status(), 'Mailpit message status').toBe(200);
    try {
      const body = await response.json();
      link = `${body.Text || ''}\n${body.HTML || ''}`.match(pattern)?.[0];
      created = Date.parse(message.Created);
      return Boolean(link);
    } finally {
      await response.dispose();
    }
  }, { timeout: 60_000, intervals: [1000, 2500, 5000], message: 'Expected link in the player mailbox' }).toBe(true);
  const url = new URL(link.replaceAll('&amp;', '&'));
  const allowed = url.origin === origin ||
    (url.protocol === 'https:' && ['localhost', '127.0.0.1'].includes(url.hostname));
  expect(allowed, 'Portal mail links use staging or a loopback host').toBe(true);
  return { url: new URL(origin + url.pathname + url.search), created };
}

export async function register(page, lang) {
  const account = identity();
  const since = Date.now() - 5000;
  const route = `/${lang}/register`;
  const fields = {
    email: account.email, password: account.password, password_confirmation: account.password,
    terms: true, privacy: true,
  };
  const response = await retrySharedLimit(async () => {
    expect((await open(page, route)).status()).toBe(200);
    let challengeLimit;
    const observeChallenge = response => {
      if (response.status() === 429 && new URL(response.url()).pathname === '/register/abuse/challenge') {
        challengeLimit = response;
      }
    };
    page.on('response', observeChallenge);
    try {
      const submitted = await submit(page, form(route), fields, { timeout: 250_000 });
      return challengeLimit ?? submitted;
    } finally {
      page.off('response', observeChallenge);
    }
  });
  expect(response.status(), 'Registration POST').toBe(303);
  await expect(page).toHaveURL(`/${lang}/register/check-inbox`);
  await expect(page.locator('main')).toContainText(lang === 'pl'
    ? 'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.'
    : 'Check your inbox for an e-mail with the next steps.');
  return { ...account, since };
}

export async function activePlayer(page, request, lang) {
  await mailAvailable(request);
  return verifyPlayer(page, request, lang, await register(page, lang));
}

export async function verifyPlayer(page, request, lang, account) {
  const pattern = /https:\/\/(?:staging\.oteryn\.com|localhost|127\.0\.0\.1)(?::[0-9]+)?\/(?:pl|en)\/verify-email\?token=[a-f0-9]+(?:&[^\s<>"\x27]*)?/;
  const { url } = await mailLink(request, account.email, account.since, pattern);
  expect((await open(page, url.href)).status(), 'Verification GET').toBe(200);
  const response = await submit(page, 'main form', {});
  expect(response.status(), 'Verification POST').toBe(200);
  await expect(page.locator('main')).toContainText(lang === 'pl'
    ? 'Twój adres e-mail został zweryfikowany.' : 'Your e-mail has been verified.');
  return account;
}

export async function retrySharedLimit(operation) {
  for (let attempt = 0; attempt < 3; attempt++) {
    const response = await operation();
    if (response.status() !== 429) return response;
    await waitForRetry(response);
  }
  test.skip(true, 'Shared staging egress was busy across three attempts');
}

export async function waitForRetry(response) {
  const seconds = Number((await response.allHeaders())['retry-after']);
  expect(seconds).toBeGreaterThanOrEqual(1);
  expect(seconds).toBeLessThanOrEqual(60);
  await delay(seconds * 1000 + randomInt(1000, 4001));
}

export function totp(secret, step = Math.floor(Date.now() / 30_000)) {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  let bits = 0;
  let buffer = 0;
  const key = [];
  for (const char of secret.trim().replace(/=+$/, '')) {
    const index = alphabet.indexOf(char);
    if (index < 0) throw new Error('Invalid authenticator secret format');
    buffer = (buffer << 5) | index;
    bits += 5;
    if (bits >= 8) {
      bits -= 8;
      key.push((buffer >> bits) & 255);
    }
  }
  const counter = Buffer.alloc(8);
  counter.writeBigUInt64BE(BigInt(step));
  const digest = createHmac('sha1', Buffer.from(key)).update(counter).digest();
  return ((digest.readUInt32BE(digest.at(-1) & 15) & 0x7fffffff) % 1_000_000).toString().padStart(6, '0');
}
