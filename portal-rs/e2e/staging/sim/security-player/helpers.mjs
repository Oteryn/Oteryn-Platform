import { test as base, expect, request } from '@playwright/test';
import { createHmac, randomBytes } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { mkdir } from 'node:fs/promises';

export { expect };
export const origin = 'https://staging.oteryn.com';
const mailApi = '/__mailpit/api/v1';
const proxyServer = process.env.HTTPS_PROXY || process.env.https_proxy;
const proxy = proxyServer ? { server: proxyServer } : undefined;
const runId = randomBytes(6).toString('hex');
let accountNumber = 0;

function accessHeaders() {
  const id = process.env.STAGING_CF_ACCESS_CLIENT_ID;
  const secret = process.env.STAGING_CF_ACCESS_CLIENT_SECRET;
  if (!id || !secret) throw new Error('Cloudflare Access credentials are missing');
  return { 'CF-Access-Client-Id': id, 'CF-Access-Client-Secret': secret };
}

// Avoid API transport errors that could include a request header dump.
export async function http(client, path, options = {}) {
  const target = new URL(path, origin);
  if (target.origin !== origin) throw new Error('Staging request points to another origin');
  const headers = { ...options.headers, ...accessHeaders() };
  try {
    return await client.fetch(target.href, {
      timeout: 12_000, ...options, headers, maxRedirects: 0,
    });
  } catch {
    throw new Error(`Staging HTTP transport failed (${options.method ?? 'GET'})`);
  }
}

export const tr = (lang, pl, en) => lang === 'pl' ? pl : en;
export const newPassword = () => `Sim!${randomBytes(16).toString('hex')}`;
export const newEmail = () => `sim-security-player-${runId}-${++accountNumber}@example.test`;
export const mainForm = page => page.locator('main form');

export async function open(page, path, status = 200) {
  const result = await page.goto(path, { waitUntil: 'domcontentloaded' });
  expect(result.status(), `GET ${new URL(path, origin).pathname.replace(/\/[a-f0-9]{64}$/, '/[token]')}`).toBe(status);
}

export async function submit(form, fields = {}, timeout = 30_000) {
  for (const [name, value] of Object.entries(fields)) {
    const input = form.locator(`[name="${name}"]`);
    if (typeof value === 'boolean') await input.setChecked(value);
    else if (await input.evaluate(element => element.tagName === 'SELECT')) await input.selectOption(value);
    else await input.fill(value);
  }
  const page = form.page();
  const action = new URL(await form.getAttribute('action') || page.url(), page.url());
  const [response] = await Promise.all([
    page.waitForResponse(result => result.request().method() === 'POST' && result.url() === action.href, { timeout }),
    form.locator('button[type=submit], button:not([type])').first().click(),
  ]);
  return response;
}

export async function postForm(form, fields = {}, path = null) {
  const page = form.page();
  const action = path || await form.getAttribute('action') || page.url();
  const defaults = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  return http(page.request, action, {
    method: 'POST', headers: { origin }, form: { ...defaults, ...fields },
  });
}

export async function withSharedQuota(attempt) {
  for (let index = 0; index < 3; index++) {
    const response = await attempt();
    if (response.status() !== 429) return response;
    const seconds = Number(response.headers()['retry-after']) || 60;
    if (index === 2 || seconds > 65) break;
    await delay((seconds + 1) * 1000);
  }
  base.skip(true, 'STAGING LIMITATION: shared ingress rate limit persisted after waiting');
}

export async function login(page, lang, account) {
  const response = await withSharedQuota(async () => {
    await open(page, `/${lang}/login`);
    return submit(mainForm(page), account);
  });
  expect(response.status()).toBe(302);
}

export async function authenticated(page, lang) {
  await open(page, `/${lang}/account/privacy`);
  await expect(page.getByRole('heading', { name: tr(lang, 'Prywatność i język', 'Privacy and language') })).toBeVisible();
}

export async function signedOut(page, lang) {
  await open(page, `/${lang}/account/privacy`);
  await expect(page).toHaveURL(`${origin}/${lang}/login`);
  await expect(page.locator('main h1')).toHaveText(tr(lang, 'Zaloguj się', 'Sign in'));
}

export async function sudo(page, lang, password) {
  await open(page, `/${lang}/account/sudo`);
  const response = await submit(mainForm(page), { current_password: password });
  expect(response.status()).toBe(302);
  expect(response.headers().location).toBe(`/${lang}/account/security`);
}

async function mailResponse(client, options) {
  return http(client, `${mailApi}/messages?limit=100`, options);
}

export async function requireMail(client, info) {
  const response = await mailResponse(client);
  const status = response.status();
  await response.dispose();
  info.skip([404, 405].includes(status), `STAGING LIMITATION: Mailpit messages returned HTTP ${status}; fresh player verification is blocked`);
  expect(status, 'Mailpit message listing').toBe(200);
}

export async function mail(client, address, since, subject, match) {
  const deadline = Date.now() + 60_000;
  do {
    const response = await mailResponse(client, { timeout: Math.min(12_000, Math.max(1, deadline - Date.now())) });
    expect(response.status(), 'Mailpit message listing').toBe(200);
    const { messages } = await response.json();
    await response.dispose();
    const matching = messages
      .filter(message => message.To.some(to => to.Address.toLowerCase() === address.toLowerCase())
        && Date.parse(message.Created) >= since && (!subject || message.Subject === subject))
      .sort((left, right) => Date.parse(right.Created) - Date.parse(left.Created))
      .slice(0, 100);
    for (const message of matching) {
      const remaining = deadline - Date.now();
      if (remaining <= 0) break;
      const detail = await http(client, `${mailApi}/message/${encodeURIComponent(message.ID)}`,
        { timeout: Math.min(12_000, remaining) });
      expect(detail.status(), 'Mailpit message detail').toBe(200);
      const { Text, HTML } = await detail.json();
      await detail.dispose();
      const found = `${Text ?? ''}\n${HTML ?? ''}`.match(match);
      if (found) return found[0];
    }
    await delay(Math.min(2000, Math.max(0, deadline - Date.now())));
  } while (Date.now() < deadline);
  throw new Error('Expected message to the test-owned address did not arrive within 60 seconds');
}

export function mailPath(link) {
  const url = new URL(link);
  expect(url.origin === origin || ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname),
    'mail link uses staging or a loopback host').toBe(true);
  return url.pathname + url.search;
}

export async function submitRegistration(page, lang, fields) {
  return withSharedQuota(async () => {
    await open(page, `/${lang}/register`);
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    let challenge;
    const observe = response => {
      if (new URL(response.url()).pathname === '/register/abuse/challenge') challenge = response;
    };
    page.on('response', observe);
    try {
      const result = await submit(mainForm(page), { ...fields, terms: true, privacy: true }, 90_000);
      // The browser falls back to the registration POST after a refused
      // challenge, so its final page can mask the shared 429.
      return result.status() < 500 && challenge?.status() === 429 ? challenge : result;
    } finally {
      page.off('response', observe);
    }
  });
}

export async function signup(page, lang) {
  const account = { email: newEmail(), password: newPassword() };
  const since = Date.now() - 2000;
  const sent = await submitRegistration(page, lang, { ...account, password_confirmation: account.password });
  expect(sent.status(), 'registration submit').toBe(303);
  await expect(page).toHaveURL(`${origin}/${lang}/register/check-inbox`);
  await expect(page.locator('main [role=status]')).toHaveText(tr(lang,
    'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.',
    'Check your inbox for an e-mail with the next steps.'));
  return { account, since };
}

export async function verifyPlayer(page, lang, { account, since }) {
  const link = await mail(page.request, account.email, since, null,
    /https:\/\/[^\s"'<>]+\/verify-email\?token=[a-f0-9]{64}/);
  await open(page, mailPath(link));
  await submit(mainForm(page));
  await expect(page.locator('main [role=status]')).toHaveText(tr(lang,
    'Twój adres e-mail został zweryfikowany.', 'Your e-mail has been verified.'));
  await login(page, lang, account);
  await authenticated(page, lang);
  return account;
}

export const totpCounter = () => Math.floor(Date.now() / 30_000);

export function totp(provisioning, counter = totpCounter()) {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  const secret = provisioning.startsWith('otpauth:')
    ? new URL(provisioning).searchParams.get('secret') : provisioning;
  const bits = [...secret.replace(/=+$/, '').toUpperCase()].map(letter => {
    const index = alphabet.indexOf(letter);
    if (index < 0) throw new Error('Invalid authenticator secret');
    return index.toString(2).padStart(5, '0');
  }).join('');
  const key = Buffer.from((bits.match(/.{8}/g) || []).map(byte => parseInt(byte, 2)));
  const step = Buffer.alloc(8);
  step.writeBigUInt64BE(BigInt(counter));
  const hash = createHmac('sha1', key).update(step).digest();
  const value = hash.readUInt32BE(hash[hash.length - 1] & 15) & 0x7fffffff;
  return String(value % 1_000_000).padStart(6, '0');
}

export const test = base.extend({
  staging: async ({ browser }, use, info) => {
    const contextOptions = {
      baseURL: origin, proxy, serviceWorkers: 'block',
      locale: info.project.use.locale,
      viewport: info.project.use.viewport,
    };
    const api = await request.newContext({ baseURL: origin, proxy });
    const contexts = [];
    try {
      await expect.poll(async () => {
        try {
          const ready = await http(api, '/readyz');
          const status = ready.status();
          await ready.dispose();
          return status;
        } catch {
          return 0;
        }
      }, { timeout: 300_000, intervals: [2000, 5000, 10_000], message: 'staging /readyz' }).toBe(200);
      await use({
        api,
        openPage: async ({ userAgent } = {}) => {
          const context = await browser.newContext({ ...contextOptions, userAgent });
          contexts.push(context);
          await context.route('**/*', async route => {
            const request = route.request();
            const { host, protocol } = new URL(request.url());
            if (host !== 'staging.oteryn.com' || protocol !== 'https:') return route.continue();
            // Keep Access headers off Chromium's outgoing redirect chain. Fetch
            // a single hop and deliver its original status and Location.
            const response = await route.fetch({
              headers: { ...await request.allHeaders(), ...accessHeaders() },
              maxRedirects: 0, maxRetries: 0, timeout: 15_000,
            }).catch(() => null);
            if (!response) return route.abort();
            try {
              await route.fulfill({ response });
            } finally {
              await response.dispose();
            }
          });
          return context.newPage();
        },
      });
    } finally {
      await Promise.all(contexts.map(context => context.close()));
      await api.dispose();
    }
  },
  page: async ({ staging }, use, info) => {
    const page = await staging.openPage();
    await use(page);
    if (info.status !== info.expectedStatus) {
      await captureFailure(page, info).catch(() => {});
    }
  },
  player: async ({ page, staging }, use, info) => {
    await requireMail(staging.api, info);
    await use(await verifyPlayer(page, info.project.metadata.locale,
      await signup(page, info.project.metadata.locale)));
  },
});

export async function captureFailure(page, info) {
  if (new URL(page.url()).origin !== origin) return;
  await mkdir(info.outputDir, { recursive: true });
  await page.screenshot({
    path: info.outputPath('failure.png'), timeout: 10_000,
    style: 'input, textarea, .op-code, a[href*="token"], .op-prose, main dd { visibility: hidden !important; }',
  });
}
