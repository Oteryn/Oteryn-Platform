import { createHmac, randomBytes } from 'node:crypto';
import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';
import { expect, request } from '@playwright/test';

export const origin = 'https://staging.oteryn.com';
export const projects = ['pl', 'en'].flatMap(lang => [1440, 390].map(width => ({
  name: `${lang}-${width}`, lang, viewport: { width, height: width === 390 ? 844 : 900 },
})));
const clientId = process.env.STAGING_CF_ACCESS_CLIENT_ID;
const clientSecret = process.env.STAGING_CF_ACCESS_CLIENT_SECRET;
const extraHTTPHeaders = {
  'CF-Access-Client-Id': clientId,
  'CF-Access-Client-Secret': clientSecret,
};
const proxyURL = process.env.HTTPS_PROXY || process.env.https_proxy;
const mailPath = '/__mailpit/api/v1';

class SweepError extends Error {}
export class StagingBlocked extends SweepError {}

export function redact(value) {
  let result = String(value);
  for (const secret of [clientId, clientSecret]) {
    if (secret) result = result.replaceAll(secret, '[redacted]');
  }
  return result.replace(/[\w.+-]+(?:@|%40)[\w.-]+\.[a-z]{2,}/gi, '[email]')
    .replace(/\b[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}\b/gi, '[id]')
    .replace(/\b[0-9a-f]{32,}\b/gi, '[token]');
}

export function containsAccessValue(value) {
  return [clientId, clientSecret].some(secret => secret && value.includes(secret));
}

export function routeKey(value) {
  const url = new URL(value, origin);
  const params = new URLSearchParams();
  for (const [name, val] of url.searchParams) {
    params.append(redact(name), /^(page|kind|type|sort|period|tab|lang)$/.test(name) &&
      /^[a-z0-9_-]{1,32}$/i.test(val) ? val : '[value]');
  }
  return redact(url.pathname) + (params.size ? '?' + params.toString() : '');
}

export function languageOf(value) {
  const { pathname } = new URL(value, origin);
  return /^\/(?:language\/)?(pl|en)(?:\/|$)/.exec(pathname)?.[1]
    || /^\/static\/thais\/shop-(pl|en)\.html$/.exec(pathname)?.[1];
}

export async function apiContext() {
  if (!clientId || !clientSecret) throw new SweepError('Missing staging Cloudflare Access environment variables');
  return request.newContext({
    baseURL: origin, maxRedirects: 0,
    ...(proxyURL && { proxy: { server: proxyURL } }),
  });
}

export async function get(api, path, timeout = 15_000) {
  try {
    const url = new URL(path, origin);
    if (url.origin !== origin) throw new SweepError('Request outside staging');
    return await api.get(url.href, {
      headers: extraHTTPHeaders, failOnStatusCode: false, maxRedirects: 0, timeout,
    });
  } catch (error) {
    if (error instanceof SweepError) throw error;
    throw new SweepError(`GET ${routeKey(path)} failed (${error.name})`);
  }
}

export async function ready(api) {
  const deadline = Date.now() + 300_000;
  let observed;
  do {
    try {
      const response = await get(api, '/readyz', Math.min(15_000, Math.max(1, deadline - Date.now())));
      observed = response.status();
      await response.dispose();
      if (observed === 200) return;
    } catch {
      observed = 'transport error';
    }
    await delay(Math.min(5_000, Math.max(0, deadline - Date.now())));
  } while (Date.now() < deadline);
  const Failure = [502, 503, 504, 'transport error'].includes(observed) ? StagingBlocked : SweepError;
  throw new Failure(`/readyz did not return 200 within five minutes (last result ${observed})`);
}

const redirects = new WeakMap();

export async function protect(context) {
  await context.route('**/*', async route => {
    const req = route.request();
    const url = new URL(req.url());
    if (url.host !== 'staging.oteryn.com' || url.origin !== origin) return route.continue();
    try {
      // Playwright carries route.continue header overrides into redirected requests.
      // Fetch one hop so that credentials never follow a cross-host Location.
      const response = await route.fetch({
        headers: { ...await req.allHeaders(), ...extraHTTPHeaders }, maxRedirects: 0, timeout: 20_000,
      });
      try {
        const headers = response.headers();
        if (req.isNavigationRequest() && [301, 302, 303, 307, 308].includes(response.status()) && headers.location) {
          redirects.set(req, { status: response.status(), location: headers.location });
          // The caller starts each document redirect as a fresh, independently scoped navigation.
          delete headers.location;
          delete headers['content-length'];
          delete headers['content-encoding'];
          delete headers['transfer-encoding'];
          await route.fulfill({ status: 200, headers, body: '' });
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
  return context;
}

async function resolveRedirects(page, first) {
  let response = first;
  const chain = [];
  while (response && redirects.has(response.request())) {
    const { status, location } = redirects.get(response.request());
    const url = new URL(location, response.url());
    if (url.origin !== origin) throw new SweepError(`HTTP ${status} from ${routeKey(response.url())} points outside staging`);
    if (chain.length >= 8) throw new SweepError('Document redirect exceeded eight hops');
    if ([307, 308].includes(status) && response.request().method() !== 'GET') {
      throw new SweepError(`HTTP ${status} requires preserving a submitted form method`);
    }
    chain.push({ status, to: routeKey(url) });
    response = await page.goto(url.href, { waitUntil: 'load', timeout: 25_000 });
  }
  return { response, chain };
}

export async function open(page, target) {
  const url = new URL(target, origin);
  if (url.origin !== origin) throw new SweepError('Navigation outside staging');
  try {
    let result;
    for (let attempt = 0; attempt < 3; attempt++) {
      result = await resolveRedirects(page, await page.goto(url.href, { waitUntil: 'load', timeout: 25_000 }));
      if (result.response?.status() !== 429 || attempt === 2) return result;
      const seconds = Number(result.response.headers()['retry-after']);
      await delay(Math.min(60, Math.max(10, seconds || 30)) * 1000);
    }
  } catch (error) {
    if (error instanceof SweepError) throw error;
    throw new SweepError(`Navigation to ${routeKey(url)} failed (${error.name})`);
  }
}

export async function submit(page, selector, fields = {}, { doubleClick = false } = {}) {
  const form = page.locator(selector);
  const formURL = page.url();
  try {
    const method = (await form.getAttribute('method', { timeout: 10_000 }))?.toUpperCase() || 'GET';
    for (let attempt = 0; attempt < 3; attempt++) {
      for (const [name, value] of Object.entries(fields)) {
        const field = form.locator(`[name="${name}"]`);
        if (typeof value === 'boolean') await field.setChecked(value);
        else if (await field.evaluate(element => element.tagName === 'SELECT')) await field.selectOption(value);
        else await field.fill(value);
      }
      let sharedLimit;
      const watchChallenge = response => {
        if (response.status() === 429 && new URL(response.url()).href === origin + '/register/abuse/challenge') {
          sharedLimit = response;
        }
      };
      page.on('response', watchChallenge);
      const completed = page.waitForNavigation({ waitUntil: 'load', timeout: 45_000 });
      const button = form.locator('button:not([type]), button[type=submit], input[type=submit]').first();
      let response;
      try { [response] = await Promise.all([completed, doubleClick ? button.dblclick() : button.click()]); }
      finally { page.off('response', watchChallenge); }
      if (!response || response.request().method() !== method) {
        throw new SweepError(`The form at ${routeKey(formURL)} did not navigate with ${method}`);
      }
      const result = await resolveRedirects(page, response);
      const limit = response.status() === 429 ? response : response.status() >= 400 && sharedLimit;
      if (!limit) {
        return { ...result, status: result.chain[0]?.status || response.status() };
      }
      if (attempt === 2) throw new StagingBlocked(`Shared egress rate limit persisted at ${routeKey(formURL)}`);
      const seconds = Number(limit.headers()['retry-after']);
      await delay(Math.min(60, Math.max(10, seconds || 60)) * 1000);
      await open(page, formURL);
    }
  } catch (error) {
    if (error instanceof SweepError) throw error;
    throw new SweepError(`Form at ${routeKey(page.url())} using ${selector} did not complete (${error.name})`);
  }
}

const portal = fileURLToPath(new URL('../../../../', import.meta.url));
const localeFiles = readdirSync(portal + 'crates', { recursive: true })
  .map(name => 'crates/' + name)
  .concat(readdirSync(portal + 'templates', { recursive: true }).map(name => 'templates/' + name))
  .filter(name => /(?:^|[/_.])(?:en|pl)\.ftl$/.test(name));

const normalized = text => text.normalize('NFC').replace(/\s+/gu, ' ').trim();
const catalogs = new Map(localeFiles.map(file => [file, new Map(
  readFileSync(portal + file, 'utf8').split('\n').flatMap((line, index) => {
    const parts = /^([\w-]+)\s*=\s*(.+)$/.exec(line);
    return parts ? [[parts[1], { value: normalized(parts[2]), source: `portal-rs/${file}:${index + 1}` }]] : [];
  }),
)]));
const catalogValues = { en: new Set(), pl: new Set() };
const catalogKeys = new Map();
const counterparts = { en: new Map(), pl: new Map() };
for (const [file, entries] of catalogs) {
  const lang = /([a-z]{2})\.ftl$/.exec(file)[1];
  const other = lang === 'pl' ? 'en' : 'pl';
  const translated = catalogs.get(file.replace(new RegExp(`${lang}\\.ftl$`), other + '.ftl'));
  for (const [id, entry] of entries) {
    catalogKeys.set(id, entry.source);
    catalogValues[lang].add(entry.value);
    const target = translated?.get(id);
    if (target && target.value !== entry.value && !/[{}]/.test(entry.value) &&
        (/\s/u.test(entry.value) || /[ąćęłńóśźż]/i.test(entry.value) || /^[\p{L}]{5,}$/u.test(entry.value))) {
      counterparts[other].set(entry.value, { ...entry, target, id });
    }
  }
}

export function localeText(file, lang, id) {
  const item = catalogs.get(file.replace('{lang}', lang))?.get(id);
  if (!item) throw new SweepError(`Missing locale entry ${file} ${id}`);
  return item.value;
}

export function localization(segments, lang) {
  const findings = [];
  for (const text of new Set(segments.map(normalized))) {
    if (text.includes('-') && catalogKeys.has(text) && !catalogValues[lang].has(text)) {
      findings.push({ check: 'locale key', detail: text, source: catalogKeys.get(text) });
    }
    const foreign = counterparts[lang].get(text);
    if (foreign && !catalogValues[lang].has(text)) {
      findings.push({
        check: 'mixed language',
        detail: `${text} → ${foreign.target.value}`,
        source: foreign.target.source,
      });
    }
  }
  return findings;
}

export async function mailStatus(api) {
  const response = await get(api, mailPath + '/messages');
  const status = response.status();
  await response.dispose();
  return status;
}

async function newestMail(api, { email, since, subject }) {
  const deadline = Date.now() + 60_000;
  do {
    const remaining = () => Math.min(15_000, Math.max(1, deadline - Date.now()));
    const response = await get(api, mailPath + '/messages?limit=100', remaining());
    const status = response.status();
    if (status !== 200) {
      await response.dispose();
      if ([404, 405].includes(status)) throw new StagingBlocked(`Mailpit list returned HTTP ${status}`);
      if ([429, 502, 503, 504].includes(status)) {
        await delay(Math.min(2_000, Math.max(0, deadline - Date.now())));
        continue;
      }
      throw new SweepError(`Mailpit list returned HTTP ${status}`);
    }
    const { messages } = await response.json();
    await response.dispose();
    const entry = messages.sort((a, b) => Date.parse(b.Created) - Date.parse(a.Created))
      .slice(0, 100).find(message => message.To?.some(recipient =>
      recipient.Address.toLowerCase() === email.toLowerCase()) &&
      Date.parse(message.Created) >= since - 2_000 && subject.includes(message.Subject));
    if (entry) {
      const detail = await get(api, mailPath + '/message/' + encodeURIComponent(entry.ID), remaining());
      if (!detail.ok()) {
        const code = detail.status();
        await detail.dispose();
        if ([404, 429, 502, 503, 504].includes(code)) {
          await delay(Math.min(2_000, Math.max(0, deadline - Date.now())));
          continue;
        }
        throw new SweepError(`Newest addressed Mailpit message returned HTTP ${code}`);
      }
      try { return await detail.json(); } finally { await detail.dispose(); }
    }
    await delay(Math.min(2_000, Math.max(0, deadline - Date.now())));
  } while (Date.now() < deadline);
  throw new StagingBlocked('No matching message arrived within 60 seconds');
}

export async function mailLink(api, { route, ...filters }) {
  const message = await newestMail(api, filters);
  for (const match of `${message.Text || ''}\n${message.HTML || ''}`.matchAll(/https?:\/\/[^\s<>'"]+/g)) {
    let url;
    try { url = new URL(match[0].replaceAll('&amp;', '&')); } catch { continue; }
    const loopback = ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname);
    if ((url.origin === origin || loopback) && route.test(url.pathname + url.search)) {
      return new URL(url.pathname + url.search, origin);
    }
  }
  throw new SweepError('Newest addressed Mailpit message has no expected staging link');
}

export async function mailCode(api, filters) {
  const message = await newestMail(api, filters);
  const code = /^\s*(\d{6})\s*$/m.exec(message.Text || '')?.[1];
  if (!code) throw new SweepError('Newest addressed Mailpit message has no expected verification code');
  return code;
}

export const password = () => 'Zq9!' + randomBytes(18).toString('base64url');
export const playerEmail = () => `sim-ux-sweep-${randomBytes(7).toString('hex')}-1@example.test`;

export function totp(provisioning, time = Date.now()) {
  const secret = provisioning.startsWith('otpauth:')
    ? new URL(provisioning).searchParams.get('secret') : provisioning;
  let bits = 0;
  let buffer = 0;
  const bytes = [];
  for (const char of secret.toUpperCase().replace(/[=\s]/g, '')) {
    const digit = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'.indexOf(char);
    if (digit === -1) throw new SweepError('Invalid TOTP provisioning format');
    buffer = (buffer << 5) | digit;
    bits += 5;
    if (bits >= 8) {
      bits -= 8;
      bytes.push((buffer >>> bits) & 255);
    }
  }
  const counter = Buffer.alloc(8);
  counter.writeBigUInt64BE(BigInt(Math.floor(time / 30_000)));
  const digest = createHmac('sha1', Buffer.from(bytes)).update(counter).digest();
  return String((digest.readUInt32BE(digest.at(-1) & 15) & 0x7fffffff) % 1_000_000).padStart(6, '0');
}

export async function expectPath(page, path, reason) {
  await expect.poll(() => new URL(page.url()).pathname === path, { message: reason }).toBe(true);
}

export async function visibleText(locator, text, reason) {
  await expect.poll(async () => {
    try {
      const content = await locator.filter({ visible: true }).allTextContents();
      return content.some(value => normalized(value).includes(normalized(text)));
    } catch (error) {
      if (!/Execution context was destroyed/.test(error.message)) throw error;
      return false;
    }
  }, { message: reason }).toBe(true);
}

export function safeError(error) {
  if (error instanceof SweepError) return redact(error.message);
  return redact(String(error.message || error).replace(/\x1b\[[\d;]*m/g, '').split('\n')[0].slice(0, 300));
}
