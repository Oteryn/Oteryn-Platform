import { expect, test as base } from '@playwright/test';
import { createHmac, randomBytes } from 'node:crypto';
import { mkdir, rmdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

export const baseURL = 'https://staging.oteryn.com';
const proxyURL = process.env.HTTPS_PROXY ?? process.env.https_proxy;
export const proxy = proxyURL ? { server: proxyURL } : undefined;
const messagesPath = '/__mailpit/api/v1/messages';

function accessHeaders() {
  const id = process.env.STAGING_CF_ACCESS_CLIENT_ID;
  const secret = process.env.STAGING_CF_ACCESS_CLIENT_SECRET;
  if (!id || !secret) throw new Error('Staging Cloudflare Access environment variables are required');
  return { 'CF-Access-Client-Id': id, 'CF-Access-Client-Secret': secret };
}

export const test = base.extend({
  page: async ({ context }, use) => {
    await context.route('**/*', async route => {
      const url = new URL(route.request().url());
      if (url.host !== 'staging.oteryn.com' || url.protocol !== 'https:') return route.continue();
      // route.continue header overrides follow redirects. Fulfill the first
      // response so Chromium makes each redirect request with its own headers.
      let response;
      try {
        response = await route.fetch({
          headers: { ...route.request().headers(), ...accessHeaders() },
          maxRedirects: 0,
          timeout: 20_000,
        });
        await route.fulfill({ response });
      } catch {
        await route.abort();
      } finally {
        await response?.dispose();
      }
    });
    await use(await context.newPage());
  },
});
export { expect };

export const local = (lang, pl, en) => lang === 'pl' ? pl : en;
export const password = () => `Sim1!${randomBytes(24).toString('base64url')}`;

export async function ready(request) {
  await expect.poll(async () => {
    try {
      const response = await get(request, '/readyz');
      const status = response.status();
      await response.dispose();
      return status;
    } catch {
      return 0;
    }
  }, {
    message: 'Staging GET /readyz must return 200',
    timeout: 300_000,
    intervals: [2_000, 5_000, 10_000],
  }).toBe(200);
}

export async function get(request, path, timeout = 10_000) {
  const url = new URL(path, baseURL);
  if (url.origin !== baseURL) throw new Error('Expected a staging URL');
  try {
    return await request.get(url.href, { headers: accessHeaders(), maxRedirects: 0, timeout });
  } catch {
    throw new Error('Staging GET failed before receiving an HTTP response');
  }
}

export async function mailReady(request) {
  const response = await get(request, messagesPath);
  const status = response.status();
  await response.dispose();
  test.skip(status === 404 || status === 405, `Mailpit GET ${messagesPath} returned HTTP ${status}`);
  expect(status, 'Mailpit list status').toBe(200);
}

export async function mailValue(request, email, subject, since, extract) {
  let found;
  const subjects = [subject].flat();
  const deadline = Date.now() + 60_000;
  const remaining = () => Math.min(10_000, deadline - Date.now());
  const pause = () => delay(Math.min(2_000, Math.max(0, deadline - Date.now())));
  do {
    const timeout = remaining();
    if (timeout < 250) break;
    const response = await get(request, `${messagesPath}?limit=100`, timeout).catch(() => null);
    if (!response) {
      await pause();
      continue;
    }
    let messages;
    try {
      test.skip([404, 405].includes(response.status()), `Mailpit GET ${messagesPath} returned HTTP ${response.status()}`);
      expect(response.status(), 'Mailpit list status').toBe(200);
      ({ messages } = await response.json());
    } finally {
      await response.dispose();
    }
    const latest = messages
      .filter(message => subjects.includes(message.Subject) &&
        Date.parse(message.Created) >= since &&
        message.To?.some(recipient => recipient.Address.toLowerCase() === email))
      .sort((left, right) => Date.parse(right.Created) - Date.parse(left.Created))[0];
    if (latest && remaining() >= 250) {
      const detail = await get(request,
        `/__mailpit/api/v1/message/${encodeURIComponent(latest.ID)}`, remaining()).catch(() => null);
      if (detail) {
        try {
          expect(detail.status(), 'Mailpit message status').toBe(200);
          const { Text, HTML } = await detail.json();
          found = extract(`${Text ?? ''}\n${HTML ?? ''}`);
        } finally {
          await detail.dispose();
        }
      }
      if (found) break;
    }
    await pause();
  } while (Date.now() < deadline);
  return found;
}

export function mailLink(body, path) {
  for (const candidate of body.matchAll(/https?:\/\/[^\s<>"']+/g)) {
    try {
      const url = new URL(candidate[0].replaceAll('&amp;', '&'));
      if ((url.origin === baseURL || ['localhost', '127.0.0.1'].includes(url.hostname)) && path.test(url.pathname)) {
        return new URL(url.pathname + url.search, baseURL).href;
      }
    } catch {
      // Mail text can contain punctuation next to the link.
    }
  }
}

export function formFor(page, path) {
  return page.locator(`main form[action="${path}"]`);
}

export async function submit(form, fields = {}) {
  for (const [name, value] of Object.entries(fields)) {
    const field = form.locator(`[name="${name}"]`);
    if (typeof value === 'boolean') await field.setChecked(value);
    else if (name === 'family') await field.selectOption(value);
    else await field.fill(value);
  }
  await form.locator('button[type=submit], button:not([type])').click();
}

export async function submitPost(page, path, fields, { timeout = 120_000, retryPage = path } = {}) {
  const deadline = Date.now() + timeout;
  for (;;) {
    let challengeLimit;
    const observe = response => {
      if (response.url() === baseURL + '/register/abuse/challenge' && response.status() === 429) {
        challengeLimit = response;
      }
    };
    // Invalid registration fields have their own validation response even
    // when the browser's request for proof was rate-limited.
    if (path.endsWith('/register') && fields.password === fields.password_confirmation) {
      page.on('response', observe);
    }
    let response;
    try {
      [response] = await Promise.all([
        page.waitForResponse(response => response.request().method() === 'POST' &&
          response.url() === baseURL + path, { timeout: Math.max(1, deadline - Date.now()) }),
        submit(formFor(page, path), fields),
      ]);
    } finally {
      page.off('response', observe);
    }
    if (response.status() !== 429 && !(challengeLimit && response.status() === 422)) return response.status();
    const limit = response.status() === 429 ? response : challengeLimit;
    const retryAfter = Number(limit.headers()['retry-after']) || 60;
    const wait = (retryAfter + 1) * 1_000;
    test.skip(Date.now() + wait + 20_000 >= deadline, 'Shared staging rate limit persisted after waiting');
    await delay(wait);
    if (typeof retryPage === 'function') fields = await retryPage();
    else await page.goto(retryPage);
  }
}

export async function capture(page, info, name) {
  const path = info.outputPath(`${name}.png`);
  await mkdir(info.outputDir, { recursive: true });
  for (let attempt = 0; attempt < 2; attempt++) {
    try {
      await page.waitForLoadState('domcontentloaded', { timeout: 5_000 });
      await page.screenshot({ path, mask: [page.locator('input, textarea')], timeout: 5_000 });
      await info.attach(name, { path, contentType: 'image/png' });
      return;
    } catch {
      // A failed navigation can still replace the document after its response arrives.
      await delay(500);
    }
  }
}

export function totp(secret, counter = Math.floor(Date.now() / 30_000)) {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  let bits = 0;
  let accumulator = 0;
  const key = [];
  for (const letter of secret.toUpperCase().replace(/[\s=]/g, '')) {
    const digit = alphabet.indexOf(letter);
    if (digit < 0) throw new Error('MFA enrollment returned an invalid Base32 secret');
    accumulator = (accumulator << 5) | digit;
    bits += 5;
    if (bits >= 8) {
      bits -= 8;
      key.push((accumulator >> bits) & 255);
    }
  }
  const step = Buffer.alloc(8);
  step.writeBigUInt64BE(BigInt(counter));
  const digest = createHmac('sha1', Buffer.from(key)).update(step).digest();
  const offset = digest.at(-1) & 15;
  return String((digest.readUInt32BE(offset) & 0x7fffffff) % 1_000_000).padStart(6, '0');
}

export async function withStagingLock(scope, work) {
  const lock = join(tmpdir(), `oteryn-game-staff-${process.env.OTERYN_SIM_GAME_STAFF_RUN}-${scope}.lock`);
  const deadline = Date.now() + 300_000;
  for (;;) {
    try {
      await mkdir(lock);
      break;
    } catch (error) {
      if (error.code !== 'EEXIST' || Date.now() >= deadline) throw error;
      await delay(1_000);
    }
  }
  try {
    return await work();
  } finally {
    await rmdir(lock);
  }
}
