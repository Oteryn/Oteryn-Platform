import { chromium, expect, test as base } from '@playwright/test';
import { createHash, createHmac, randomBytes, randomUUID, X509Certificate } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { setTimeout as delay } from 'node:timers/promises';

export const staging = 'https://staging.oteryn.com';
const mailbox = '/__mailpit/api/v1';
const accounts = {
  editor: 'staff-content-editor@example.invalid',
  support: 'staff-support@example.invalid',
};
const text = {
  en: {
    reset: 'If an account exists for that email, a password reset link has been sent.',
    resetSubject: 'Reset your Oteryn password',
    mfaSubject: 'Your Oteryn verification code',
    verified: 'Your e-mail has been verified.'
  },
  pl: {
    reset: 'Jeśli konto z tym adresem istnieje, wysłaliśmy link do zresetowania hasła.',
    resetSubject: 'Zresetuj hasło do Oteryn',
    mfaSubject: 'Twój kod weryfikacyjny Oteryn',
    verified: 'Twój adres e-mail został zweryfikowany.'
  },
};

function accessHeaders() {
  const { STAGING_CF_ACCESS_CLIENT_ID, STAGING_CF_ACCESS_CLIENT_SECRET } = process.env;
  if (!STAGING_CF_ACCESS_CLIENT_ID || !STAGING_CF_ACCESS_CLIENT_SECRET) {
    throw new Error('Both staging Cloudflare Access environment variables are required.');
  }
  return {
    'CF-Access-Client-Id': STAGING_CF_ACCESS_CLIENT_ID,
    'CF-Access-Client-Secret': STAGING_CF_ACCESS_CLIENT_SECRET,
  };
}

export function chromiumOptions() {
  const { HTTPS_PROXY, CODEX_PROXY_CERT } = process.env;
  const options = { executablePath: '/usr/bin/chromium' };
  if (HTTPS_PROXY) options.proxy = { server: HTTPS_PROXY };
  if (HTTPS_PROXY && CODEX_PROXY_CERT) {
    // Chromium handles same-origin redirect hops through the execution proxy.
    const certificate = new X509Certificate(readFileSync(CODEX_PROXY_CERT));
    const key = certificate.publicKey.export({ format: 'der', type: 'spki' });
    const pin = createHash('sha256').update(key).digest('base64');
    options.args = [`--ignore-certificate-errors-spki-list=${pin}`];
  }
  return options;
}

async function retryTransient(send, method) {
  let response = await send();
  if (response.status() === 429) {
    const remaining = Number(response.headers()['retry-after']);
    await response.dispose();
    await delay(Number.isFinite(remaining) && remaining > 0 ? Math.min(remaining, 65) * 1000 + 500 : 61_000);
    response = await send();
  } else if (method === 'GET' && response.status() === 502 && response.headers()['cf-ray']) {
    await response.dispose();
    await delay(1_000);
    response = await send();
  }
  return response;
}

export async function get(request, path, { timeout = 15_000, retry = true } = {}) {
  const url = new URL(path, staging);
  if (url.origin !== staging) throw new Error('Expected a staging request.');
  const send = () => request.get(url.href, {
    headers: accessHeaders(),
    maxRedirects: 0,
    timeout,
  }).catch(() => {
    throw new Error('Staging GET failed before receiving a response.');
  });
  return retry ? retryTransient(send, 'GET') : send();
}

async function routeStaging(context) {
  await context.route('**/*', async route => {
    const request = route.request();
    const url = new URL(request.url());
    if (url.host !== 'staging.oteryn.com' || url.protocol !== 'https:') return route.continue();
    // Playwright's continue({ headers }) can propagate those headers to redirects.
    // Fetch one hop and let Chromium handle the response without the Access headers.
    try {
      const response = await retryTransient(() => route.fetch({
        headers: { ...request.headers(), ...accessHeaders() },
        maxRedirects: 0,
        timeout: 15_000,
      }), request.method());
      const location = response.headers().location;
      if (response.status() >= 300 && response.status() < 400 && location
          && new URL(location, request.url()).origin !== staging) {
        await response.dispose();
        return route.abort();
      }
      await route.fulfill({ response });
      await response.dispose().catch(() => {});
    } catch {
      await route.abort().catch(() => {});
    }
  });
}

export async function ready(request) {
  await expect.poll(async () => {
    try {
      const response = await get(request, '/readyz', { timeout: 10_000, retry: false });
      return response.status();
    } catch {
      return 'staging transport unavailable';
    }
  }, { timeout: 300_000, intervals: [5_000], message: 'GET /readyz' }).toBe(200);
}

export async function visit(page, path, status = 200) {
  const response = await page.goto(path, { waitUntil: 'domcontentloaded' });
  expect(response?.status(), `GET ${path}`).toBe(status);
  return response;
}

export async function submit(page, selector, fields = {}, button = 'button', timeout = 90_000) {
  const form = page.locator(selector).first();
  for (const [name, value] of Object.entries(fields)) {
    const input = form.locator(`[name="${name}"]`);
    if (typeof value === 'boolean') await input.setChecked(value);
    else await input.fill(value);
  }
  const [response] = await Promise.all([
    page.waitForResponse(response => response.request().isNavigationRequest()
      && response.request().method() === 'POST', { timeout }),
    form.locator(button).first().click(),
  ]);
  await page.waitForLoadState('domcontentloaded');
  return response.status();
}

export async function mailStatus(request) {
  const response = await get(request, `${mailbox}/messages`);
  return response.status();
}

async function readMail(request, address, matchesSubject, since) {
  const deadline = Date.now() + 60_000;
  while (Date.now() < deadline) {
    const timeout = Math.min(10_000, deadline - Date.now());
    const listing = await get(request, `${mailbox}/messages?limit=100`, { timeout, retry: false });
    if (listing.status() === 429) {
      await listing.dispose();
      await delay(Math.min(5_000, Math.max(0, deadline - Date.now())));
      continue;
    }
    if (listing.status() !== 200) throw new Error(`Mailpit listing returned HTTP ${listing.status()}.`);
    const { messages } = await listing.json();
    await listing.dispose();
    const newest = messages.slice(0, 100).reduce((current, message) => {
      const created = Date.parse(message.Created);
      if (created < since || !Number.isFinite(created) || !matchesSubject(message.Subject)
          || !message.To.some(to => to.Address.toLowerCase() === address.toLowerCase())) return current;
      return !current || created > Date.parse(current.Created) ? message : current;
    }, null);
    if (newest) {
      const detail = await get(request, `${mailbox}/message/${encodeURIComponent(newest.ID)}`, {
        timeout: Math.max(1, Math.min(10_000, deadline - Date.now())),
        retry: false,
      });
      if (detail.status() !== 200) throw new Error(`Mailpit message returned HTTP ${detail.status()}.`);
      const { Text, HTML } = await detail.json();
      await detail.dispose();
      return `${Text ?? ''}\n${HTML ?? ''}`;
    }
    await delay(Math.min(2_000, Math.max(0, deadline - Date.now())));
  }
  return null;
}

async function openMailLink(page, body, pattern) {
  const link = body?.replaceAll('&amp;', '&').match(pattern)?.[0];
  if (!link) throw new Error('Expected a staging link in the email.');
  const { pathname, search } = new URL(link);
  const target = new URL(staging);
  target.pathname = pathname;
  target.search = search;
  const status = await page.goto(target.href, { waitUntil: 'domcontentloaded' })
    .then(response => response?.status(), () => undefined);
  expect(status, 'Opening the link from the email').toBe(200);
  return pathname.split('/')[1];
}

function freshPassword() {
  return `Sim!${randomBytes(24).toString('base64url')}`;
}

function totp(secret) {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  const bits = [...secret.trim().toUpperCase().replace(/=+$/, '')]
    .map(character => alphabet.indexOf(character));
  if (!bits.length || bits.some(value => value < 0)) throw new Error('Invalid TOTP setup key.');
  const binary = bits.map(value => value.toString(2).padStart(5, '0')).join('');
  const key = Buffer.from((binary.match(/.{8}/g) ?? []).map(byte => parseInt(byte, 2)));
  const counter = Buffer.alloc(8);
  counter.writeBigUInt64BE(BigInt(Math.floor(Date.now() / 30_000)));
  const digest = createHmac('sha1', key).update(counter).digest();
  const offset = digest.at(-1) & 15;
  return String((digest.readUInt32BE(offset) & 0x7fffffff) % 1_000_000).padStart(6, '0');
}

export async function login(page, locale, email, password) {
  await visit(page, `/${locale}/login`);
  expect(await submit(page, 'main form', { email, password }), 'Sign-in response').toBe(302);
}

async function recoverStaff(page, role, locale) {
  const email = accounts[role];
  const password = freshPassword();
  const since = Date.now() - 1_000;
  await visit(page, `/${locale}/forgot-password`);
  expect(await submit(page, 'main form', { email }), 'Password reset request').toBe(302);
  await expect(page.getByRole('status')).toHaveText(text[locale].reset);
  const reset = await readMail(page.request, email,
    subject => Object.values(text).some(copy => subject === copy.resetSubject), since);
  if (!reset) return { blocked: `No password reset email for the ${role} account within 60 seconds.` };

  locale = await openMailLink(page, reset, /https?:\/\/(?:staging\.oteryn\.com|localhost|127\.0\.0\.1)(?::\d+)?\/(?:en|pl)\/reset-password\/[a-f0-9]+(?:\?[^\s"'<>)]*)?/);
  const formStatus = await submit(page, 'main form', { email, password, password_confirmation: password })
    .catch(() => undefined);
  if (formStatus === 503) {
    return { blocked: `The ${role} password reset form returned HTTP 503 after opening the recovery email.` };
  }
  expect(formStatus, 'Submitting the password reset form').toBe(302);
  expect(new URL(page.url()).pathname === `/${locale}/login`, 'Reset redirects to sign-in').toBe(true);
  await login(page, locale, email, password);

  if (new URL(page.url()).pathname.endsWith('/mfa/challenge')) {
    const sent = Date.now() - 1_000;
    expect(await submit(page, 'main form[action$="/mfa/email"]')).toBe(302);
    const message = await readMail(page.request, email, subject => subject === text[locale].mfaSubject, sent);
    const code = message?.match(/\b[a-f0-9]{64}\b/)?.[0];
    if (!code) return { blocked: `No MFA email for the ${role} account within 60 seconds.` };
    expect(await submit(page, 'main form[action$="/mfa/challenge"]', { code })).toBe(302);
  } else {
    await expect(page).toHaveURL(new RegExp(`/${locale}/$`));
    await visit(page, `/${locale}/mfa`);
    expect(await submit(page, 'main form[action$="/mfa/enroll"]', { password })).toBe(200);
    const secret = await page.locator('main .op-prose').textContent();
    if (Date.now() % 30_000 > 26_000) await delay(4_000);
    expect(await submit(page, 'main form[action$="/mfa/confirm"]', {
      password, code: totp(secret),
    })).toBe(200);
    await login(page, locale, email, password);
    await expect(page).toHaveURL(new RegExp(`/${locale}/mfa/challenge$`));
    // The enrollment code may already have consumed the current TOTP time step.
    await delay(30_000 - (Date.now() % 30_000) + 500);
    expect(await submit(page, 'main form[action$="/mfa/challenge"]', { code: totp(secret) })).toBe(302);
  }
  await expect(page).toHaveURL(new RegExp(`/${locale}/$`));
  const { cookies } = await page.context().storageState();
  const session = cookies.find(cookie => cookie.name === '__Host-oteryn_session');
  if (!session) throw new Error('Staff sign-in ended without a portal session.');
  return { storageState: { cookies: [session], origins: [] } };
}

export default async function setup() {
  const browser = await chromium.launch(chromiumOptions());
  const context = await browser.newContext({
    baseURL: staging,
    locale: 'en-US',
    serviceWorkers: 'block',
  });
  try {
    await routeStaging(context);
    await ready(context.request);
    const mail = await mailStatus(context.request);
    const admin = await get(context.request, '/admin/news/create');
    process.env.OTERYN_SIM_MAIL_STATUS = String(mail);
    process.env.OTERYN_SIM_ADMIN_STATUS = String(admin.status());
    for (const role of Object.keys(accounts)) {
      let state;
      if (![200, 404, 405].includes(mail)) {
        state = { failed: `GET ${mailbox}/messages returned unexpected HTTP ${mail}.` };
      } else if (![303, 404].includes(admin.status())) {
        state = { failed: `The editor preflight returned unexpected HTTP ${admin.status()}.` };
      } else if (mail !== 200) {
        state = { blocked: `GET ${mailbox}/messages returned HTTP ${mail}.` };
      } else if (role === 'support' && admin.status() === 404) {
        state = { blocked: 'The staging host returned HTTP 404 for /admin/news/create before login.' };
      } else {
        const staff = await browser.newContext({
          baseURL: staging,
          locale: 'en-US',
          serviceWorkers: 'block',
        });
        try {
          await routeStaging(staff);
          state = await recoverStaff(await staff.newPage(), role, 'en');
        } finally {
          await staff.close();
        }
      }
      // Global setup runs once before the two workers. Share session state in memory.
      process.env[`OTERYN_SIM_${role.toUpperCase()}`] = JSON.stringify(state);
      if (state.blocked) console.log(`Staging ${role} setup: ${state.blocked}`);
    }
  } finally {
    await browser.close();
  }
}

export const test = base.extend({
  context: async ({ context }, use) => {
    await routeStaging(context);
    await use(context);
  },
  staffPage: async ({ browser, contextOptions, locale, viewport, isMobile, hasTouch }, use) => {
    await use(async (role, { needsPanel = true } = {}) => {
      const state = JSON.parse(process.env[`OTERYN_SIM_${role.toUpperCase()}`]);
      if (state.failed) throw new Error(state.failed);
      test.skip(Boolean(state.blocked), state.blocked);
      test.skip(needsPanel && process.env.OTERYN_SIM_ADMIN_STATUS === '404',
        'The staging host hides the CMS.');
      const staff = await browser.newContext({
        ...contextOptions,
        locale, viewport, isMobile, hasTouch,
        baseURL: staging,
        storageState: state.storageState,
        serviceWorkers: 'block',
      });
      await routeStaging(staff);
      return { context: staff, page: await staff.newPage() };
    });
  },
});

export const unique = () => randomUUID().replaceAll('-', '').slice(0, 16);

export async function register(page, locale) {
  const email = `sim-content-editor-${unique()}-1@example.test`;
  const password = freshPassword();
  const since = Date.now() - 1_000;
  await visit(page, `/${locale}/register`);
  return { email, password, since };
}

export async function verifyPlayer(page, locale, account) {
  const message = await readMail(page.request, account.email, () => true, account.since);
  if (!message) throw new Error('No email verification message for the new player within 60 seconds.');
  await openMailLink(page, message, /https?:\/\/(?:staging\.oteryn\.com|localhost|127\.0\.0\.1)(?::\d+)?\/(?:en|pl)\/verify-email\?token=[a-f0-9]+[^\s"'<>)]*/);
  const status = await submit(page, 'main form').catch(() => undefined);
  expect(status, 'Email verification confirmation').toBe(200);
  await expect(page.getByRole('status')).toHaveText(text[locale].verified);
  await login(page, locale, account.email, account.password);
  await expect(page).toHaveURL(new RegExp(`/${locale}/$`));
}

export { expect };
