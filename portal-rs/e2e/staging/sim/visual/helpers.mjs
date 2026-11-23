import { createHmac, randomBytes } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { request } from '@playwright/test';

const accessHost = 'staging.oteryn.com';
export const origin = new URL(process.env.STAGING_ORIGIN || `https://${accessHost}`).origin;
const proxyServer = process.env.STAGING_PROXY;
export const proxy = proxyServer ? { server: proxyServer } : undefined;

function accessHeaders(url) {
  const parsed = new URL(url);
  if (parsed.host !== accessHost || parsed.protocol !== 'https:') return {};
  const { STAGING_CF_ACCESS_CLIENT_ID: id, STAGING_CF_ACCESS_CLIENT_SECRET: secret } = process.env;
  if (!id || !secret) throw new Error('Staging Cloudflare Access environment variables are required');
  return { 'CF-Access-Client-Id': id, 'CF-Access-Client-Secret': secret };
}

export async function protectContext(context) {
  accessHeaders(origin);
  await context.route('**/*', async route => {
    const url = new URL(route.request().url());
    if (url.origin !== origin || url.host !== accessHost || url.protocol !== 'https:') return route.continue();

    // A header override on continue() also applies to redirects. Fetch a single
    // hop and let the browser make a new request without the override.
    let response;
    try {
      response = await route.fetch({
        headers: { ...await route.request().allHeaders(), ...accessHeaders(url) },
        maxRedirects: 0,
        timeout: 15_000,
      });
      const location = response.headers().location;
      if (response.status() >= 300 && response.status() < 400 && location
        && new URL(location, url).origin !== origin && route.request().isNavigationRequest()) {
        return await route.abort('blockedbyclient');
      }
      await route.fulfill({ response });
    } catch {
      await route.abort().catch(() => {});
    } finally {
      await response?.dispose();
    }
  });
}

export async function withApi(use) {
  const api = await request.newContext({
    proxy,
    ignoreHTTPSErrors: process.env.STAGING_IGNORE_HTTPS_ERRORS === '1',
  });
  const get = async path => {
    const target = new URL(path, origin);
    if (target.origin !== origin) throw new Error('API request left the portal origin');
    try {
      return await api.get(target.href, {
        headers: accessHeaders(target),
        timeout: 10_000,
        maxRedirects: 0,
      });
    } catch {
      // Playwright request errors can print request headers.
      throw new Error('Portal API request failed at the transport layer');
    }
  };
  try {
    return await use(get);
  } finally {
    await api.dispose();
  }
}

export async function ready() {
  await withApi(async get => {
    const deadline = Date.now() + 300_000;
    let status;
    do {
      const response = await get('/readyz').catch(() => null);
      status = response?.status();
      await response?.dispose();
      if (status === 200) return;
      await delay(5_000);
    } while (Date.now() < deadline);
    throw new Error(`/readyz did not return HTTP 200 within 5 minutes (last: ${status ?? 'transport failure'})`);
  });
}

const mailList = '/__mailpit/api/v1/messages?limit=100';
export async function mailStatus() {
  return withApi(async get => (await get(mailList)).status());
}

export class MailUnavailable extends Error {}

async function selectedMail(email, since, subject, extract) {
  return withApi(async get => {
    const deadline = Date.now() + 60_000;
    do {
      const response = await get(mailList);
      if ([404, 405].includes(response.status())) {
        throw new MailUnavailable(`Mailpit returned HTTP ${response.status()}`);
      }
      if (!response.ok()) throw new Error(`Mailpit list HTTP ${response.status()}`);
      const { messages } = await response.json();
      await response.dispose();
      const recent = messages.sort((a, b) => Date.parse(b.Created) - Date.parse(a.Created)).slice(0, 100);
      const message = recent.find(candidate => Date.parse(candidate.Created) >= since
        && (!subject || candidate.Subject === subject) && candidate.To.some(to => to.Address.toLowerCase() === email));
      if (message) {
        const detail = await get(`/__mailpit/api/v1/message/${encodeURIComponent(message.ID)}`);
        if (!detail.ok()) throw new Error(`Mailpit message HTTP ${detail.status()}`);
        const content = await detail.json();
        await detail.dispose();
        const value = extract(content);
        if (value) return value;
        throw new Error('Latest matching mail did not contain the expected handoff');
      }
      await delay(3_000);
    } while (Date.now() < deadline);
    throw new MailUnavailable('No matching mail arrived within 60 seconds');
  });
}

export async function mailLink(email, since, subject, matches) {
  return selectedMail(email, since, subject, ({ Text, HTML }) => {
    for (const string of `${Text ?? ''}\n${HTML ?? ''}`.match(/https?:\/\/[^\s<>"']+/g) ?? []) {
      const source = URL.parse(string.replaceAll('&amp;', '&'));
      if (source && (source.origin === origin || ['localhost', '127.0.0.1'].includes(source.hostname))
        && matches.test(source.pathname + source.search)) {
        return new URL(source.pathname + source.search, origin).href;
      }
    }
  });
}

export async function emailCode(email, since) {
  return selectedMail(email, since, 'Your Oteryn verification code',
    ({ Text }) => Text?.match(/^\s*(\d{6})\s*$/m)?.[1]);
}

export const unique = () => randomBytes(5).toString('hex');
export const password = () => `Visual!${randomBytes(18).toString('hex')}`;

export async function fill(form, values) {
  for (const [name, value] of Object.entries(values)) {
    const field = form.locator(`[name="${name}"]`);
    if (typeof value === 'boolean') await field.setChecked(value);
    else if (await field.evaluate(el => el.tagName === 'SELECT')) await field.selectOption(value);
    else await field.fill(value);
  }
}
export async function submit(form, values = {}) {
  await fill(form, values);
  await form.locator('button[type=submit], button:not([type]), input[type=submit]').first().click();
}

export async function goto(page, path) {
  const url = new URL(path, origin);
  if (url.origin !== origin) throw new Error('Navigation left the portal origin');
  for (let attempt = 0; attempt < 2; attempt++) {
    let response;
    try {
      response = await page.goto(url.href, { waitUntil: 'domcontentloaded' });
    } catch {
      throw new Error('Portal navigation could not complete');
    }
    if (response?.status() !== 429 || attempt) return response;
    await delay(65_000);
  }
}

// RFC 6238, HMAC-SHA1, six digits, 30-second period.
export function totp(provisioning) {
  const secret = provisioning.startsWith('otpauth:')
    ? new URL(provisioning).searchParams.get('secret') : provisioning;
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  const bytes = [];
  let value = 0;
  let bits = 0;
  for (const character of secret.toUpperCase().replace(/[\s=]/g, '')) {
    const index = alphabet.indexOf(character);
    if (index < 0) throw new Error('Invalid authenticator provisioning format');
    value = (value << 5) | index;
    bits += 5;
    if (bits >= 8) {
      bits -= 8;
      bytes.push((value >> bits) & 255);
    }
  }
  const counter = Buffer.alloc(8);
  counter.writeBigUInt64BE(BigInt(Math.floor(Date.now() / 30_000)));
  const digest = createHmac('sha1', Buffer.from(bytes)).update(counter).digest();
  const offset = digest.at(-1) & 15;
  return String((digest.readUInt32BE(offset) & 0x7fffffff) % 1_000_000).padStart(6, '0');
}
