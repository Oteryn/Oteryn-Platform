import { expect, test as base } from '@playwright/test';
import { createHmac, randomBytes, randomUUID } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';

const origin = 'https://staging.oteryn.com';
const inbox = '/__mailpit/api/v1';
const mailUnavailable = [404, 405];

function accessHeaders() {
  const id = process.env.STAGING_CF_ACCESS_CLIENT_ID;
  const secret = process.env.STAGING_CF_ACCESS_CLIENT_SECRET;
  if (!id || !secret) throw new Error('The two staging Cloudflare Access variables are required.');
  return { 'CF-Access-Client-Id': id, 'CF-Access-Client-Secret': secret };
}

export async function routeStaging(context) {
  const access = accessHeaders();
  await context.route('**/*', async route => {
    const request = route.request();
    const url = new URL(request.url());
    if (url.host !== 'staging.oteryn.com' || url.protocol !== 'https:') {
      await route.continue();
      return;
    }

    // Forward only this hop. Playwright can otherwise reuse override headers on redirects.
    let response;
    try {
      response = await route.fetch({
        headers: { ...request.headers(), ...access },
        maxRedirects: 0,
        timeout: 20_000,
      });
      if (response.status() >= 300 && response.status() < 400 && response.headers().location) {
        const destination = new URL(response.headers().location, request.url());
        if (destination.origin !== origin) {
          await route.abort();
          return;
        }
      }
      await route.fulfill({ response });
    } catch {
      // Route fetch errors may contain request headers.
      await route.abort().catch(() => {});
    } finally {
      await response?.dispose().catch(() => {});
    }
  });
}

export { expect };
export const localized = (lang, pl, en) => lang === 'pl' ? pl : en;
const newPassword = () => `Sim!${randomBytes(18).toString('hex')}`;

export const freshPlayer = () => ({
  email: `sim-security-ops-${randomUUID().replaceAll('-', '')}-1@example.test`,
  password: newPassword(),
});

export async function get(request, path, method = 'GET') {
  const url = new URL(path, origin);
  if (url.origin !== origin) throw new Error('Staging HTTP requests must stay on the staging origin');
  const headers = accessHeaders();
  try {
    return await request.fetch(url.href, { method, headers, maxRedirects: 0, timeout: 10_000 });
  } catch {
    // Playwright's request exceptions can contain outgoing headers.
    throw new Error(`${method} ${path.split('?')[0]} did not receive an HTTP response`);
  }
}

export const test = base.extend({
  context: async ({ context }, use) => {
    await routeStaging(context);
    await use(context);
  },
  readiness: [async ({ request }, use) => {
    await expect.poll(async () => {
      try {
        return (await get(request, '/readyz')).status();
      } catch {
        return 0;
      }
    }, {
      timeout: 300_000,
      intervals: [1_000, 3_000, 5_000],
      message: 'Staging readiness returns HTTP 200',
    }).toBe(200);
    await use();
  }, { auto: true }],
});

export async function navigate(page, path) {
  const url = new URL(path, origin);
  if (url.origin !== origin) throw new Error('Browser navigation must stay on the staging origin');
  try {
    return (await page.goto(url.href, { waitUntil: 'domcontentloaded', timeout: 20_000 })).status();
  } catch {
    // A recovery link can contain a one-use token in the path or query.
    throw new Error('Staging page navigation did not complete');
  }
}

export async function status(page, actual, expected, route) {
  if (actual === 500) {
    const path = test.info().outputPath(`${route.replace(/[^a-z0-9]+/gi, '-').replace(/^-/, '')}-500.png`);
    await page.screenshot({
      path,
      mask: [page.locator('input, textarea, .op-prose')],
      timeout: 5_000,
    });
    test.info().annotations.push({ type: 'product defect', description: `HTTP 500 on ${route} (${path})` });
  }
  expect(actual, `${route} HTTP status`).toBe(expected);
}

export async function submit(page, selector, values, timeout = 25_000) {
  const form = page.locator(selector);
  const target = new URL(await form.evaluate(node => node.action));
  if (target.origin !== origin) throw new Error('Form submission must stay on the staging origin');
  const action = target.pathname;
  try {
    for (const [field, value] of Object.entries(values)) {
      const input = form.locator(`[name="${field}"]`);
      if (typeof value === 'boolean') await input.setChecked(value);
      else await input.fill(value);
    }
    const method = await form.getAttribute('method') ?? 'get';
    const [response] = await Promise.all([
      page.waitForResponse(response => response.request().isNavigationRequest()
        && response.request().method() === method.toUpperCase()
        && new URL(response.url()).pathname === action, { timeout }),
      form.locator('button[type="submit"], button:not([type])').click(),
    ]);
    await page.waitForLoadState('domcontentloaded');
    return response;
  } catch {
    throw new Error(`Form submission did not complete for ${action}`);
  }
}

export async function submitFrom(page, selector, values, { timeout, challenge } = {}) {
  const path = new URL(page.url()).pathname;
  let sharedLimit;
  const onResponse = response => {
    if (challenge && new URL(response.url()).pathname === challenge && response.status() === 429) {
      sharedLimit = response;
    }
  };
  page.on('response', onResponse);
  try {
    let response = await submit(page, selector, values, timeout);
    const limit = response.status() === 429 ? response
      : response.status() === 422 ? sharedLimit : undefined;
    if (!limit) return response;

    const wait = Number(limit.headers()['retry-after']);
    await delay(Number.isFinite(wait) && wait > 0 ? Math.min(65_000, wait * 1000 + 1000) : 61_000);
    await status(page, await navigate(page, path), 200, 'reload form after shared rate limit');
    sharedLimit = undefined;
    response = await submit(page, selector, values, timeout);
    const stillLimited = response.status() === 429 || (response.status() === 422 && sharedLimit);
    test.skip(Boolean(stillLimited), `Shared staging rate limit persists for ${path}`);
    return response;
  } finally {
    page.off('response', onResponse);
  }
}

async function mailStatus(request) {
  return (await get(request, `${inbox}/messages?limit=100`)).status();
}

export async function requireMail(request) {
  const code = await mailStatus(request);
  test.skip(mailUnavailable.includes(code), `Staging Mailpit is unavailable (HTTP ${code})`);
  expect(code, 'Mailpit list HTTP status').toBe(200);
}

async function readMailJson(response) {
  try {
    return await response.json();
  } catch {
    throw new Error('Mailpit returned invalid JSON');
  }
}

async function newestMail(request, email, subjects, since) {
  const expires = Date.now() + 60_000;
  while (Date.now() < expires) {
    const listing = await get(request, `${inbox}/messages?limit=100`);
    const code = listing.status();
    if (mailUnavailable.includes(code)) {
      test.skip(true, `Staging Mailpit became unavailable (HTTP ${code})`);
    }
    expect(code, 'Mailpit list HTTP status').toBe(200);
    const { messages } = await readMailJson(listing);
    const [match] = (messages ?? []).slice(0, 100)
      .filter(message => subjects.includes(message.Subject)
        && message.To.some(to => to.Address.toLowerCase() === email)
        && Date.parse(message.Created) >= since)
      .sort((a, b) => Date.parse(b.Created) - Date.parse(a.Created) || b.ID.localeCompare(a.ID));
    if (match) {
      const message = await get(request, `${inbox}/message/${encodeURIComponent(match.ID)}`);
      expect(message.status(), 'Mailpit message HTTP status').toBe(200);
      return readMailJson(message);
    }
    await delay(Math.min(3_000, Math.max(0, expires - Date.now())));
  }
  return null;
}

function mailLink(message, path) {
  const text = `${message.Text ?? ''}\n${message.HTML ?? ''}`.replaceAll('&amp;', '&');
  for (const link of text.matchAll(/https?:\/\/[^\s<>"')]+/g)) {
    const url = new URL(link[0]);
    const allowedOrigin = url.origin === origin
      || ['localhost', '127.0.0.1'].includes(url.hostname);
    if (allowedOrigin && path.test(url.pathname)) {
      const stagingUrl = new URL(origin);
      stagingUrl.pathname = url.pathname;
      stagingUrl.search = url.search;
      return stagingUrl.href;
    }
  }
  throw new Error('Mail did not contain the expected same-origin staging link');
}

export async function register(page, lang, account) {
  await status(page, await navigate(page, `/${lang}/register`), 200, 'registration form');
  const since = Math.floor(Date.now() / 1000) * 1000;
  const response = await submitFrom(page, 'main form', {
    email: account.email, password: account.password, password_confirmation: account.password,
    terms: true, privacy: true,
  }, { timeout: 245_000, challenge: '/register/abuse/challenge' });
  if (response.status() === 503) {
    await expect(page.locator('main')).toContainText(localized(lang,
      'Rejestracja jest obecnie niedostępna. Spróbuj ponownie później.',
      'Registration is currently unavailable. Please try again later.'));
    test.skip(true, 'Staging registration is unavailable (HTTP 503)');
  }
  await status(page, response.status(), 303, 'registration submission');
  await expect(page).toHaveURL(`/${lang}/register/check-inbox`);
  await expect(page.locator('main [role="status"]')).toHaveText(localized(lang,
    'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.',
    'Check your inbox for an e-mail with the next steps.'));
  await requireMail(page.request);
  const message = await newestMail(page.request, account.email, [localized(lang,
    'Potwierdź adres e-mail', 'Verify your email address')], since);
  expect(Boolean(message), 'Registration verification mail arrived within 60 seconds').toBe(true);
  const link = mailLink(message, new RegExp(`^/${lang}/verify-email$`));
  await status(page, await navigate(page, link), 200, 'player email verification');
  await status(page, (await submit(page, 'main form[action*="/verify-email?token="]', {})).status(),
    200, 'player email verification submission');
  await expect(page.locator('main')).toContainText(localized(lang,
    'Twój adres e-mail został zweryfikowany.', 'Your e-mail has been verified.'));
}

export async function login(page, lang, account) {
  await status(page, await navigate(page, `/${lang}/login`), 200, 'sign-in form');
  const response = await submitFrom(page, 'main form[action$="/login"]', account);
  await status(page, response.status(), 302, 'sign-in submission');
  return new URL(page.url()).pathname;
}

function totp(secret, step = Math.floor(Date.now() / 30_000)) {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  const bytes = [];
  let bits = 0;
  let value = 0;
  for (const character of secret.toUpperCase().replace(/\s|=/g, '')) {
    const index = alphabet.indexOf(character);
    if (index < 0) throw new Error('MFA enrollment returned an invalid Base32 secret');
    value = (value << 5) | index;
    bits += 5;
    if (bits >= 8) {
      bits -= 8;
      bytes.push((value >>> bits) & 255);
    }
  }
  const counter = Buffer.alloc(8);
  counter.writeBigUInt64BE(BigInt(step));
  const digest = createHmac('sha1', Buffer.from(bytes)).update(counter).digest();
  const offset = digest.at(-1) & 15;
  return ((digest.readUInt32BE(offset) & 0x7fffffff) % 1_000_000).toString().padStart(6, '0');
}

const roleAccounts = {
  security: 'staff-security-admin@example.invalid',
  operations: 'staff-operations@example.invalid',
};

export async function staffLogin(page, lang, role) {
  const account = { email: roleAccounts[role], password: newPassword() };
  await status(page, await navigate(page, `/${lang}/forgot-password`), 200, 'staff recovery form');
  const since = Math.floor(Date.now() / 1000) * 1000;
  const response = await submitFrom(page, 'main form', { email: account.email });
  await status(page, response.status(), 302, 'staff recovery request');
  await expect(page.locator('main [role="status"]')).toHaveText(localized(lang,
    'Jeśli konto z tym adresem istnieje, wysłaliśmy link do zresetowania hasła.',
    'If an account exists for that email, a password reset link has been sent.'));
  const message = await newestMail(page.request, account.email,
    ['Zresetuj hasło do Oteryn', 'Reset your Oteryn password'], since);
  test.skip(!message, `No reset mail within 60 seconds for the named ${role} role`);
  const link = mailLink(message, /^\/(pl|en)\/reset-password(?:\/[a-f0-9]{64})?$/);
  const linkLanguage = new URL(link).pathname.split('/')[1];
  await status(page, await navigate(page, link), 200, 'staff password reset form');
  await status(page, (await submit(page, 'main form', {
    ...account, password_confirmation: account.password,
  })).status(), 302, 'staff password reset submission');
  expect(new URL(page.url()).pathname === `/${linkLanguage}/login`, 'completed reset returns to sign-in').toBe(true);

  let secret;
  let enrollmentStep;
  const destination = await login(page, lang, account);
  if (destination === `/${lang}/`) {
    await status(page, await navigate(page, `/${lang}/mfa`), 200, 'staff MFA settings');
    await status(page, (await submit(page, 'main form[action$="/mfa/enroll"]',
      { password: account.password })).status(), 200, 'staff MFA enrollment');
    secret = await page.locator('main .op-prose').innerText();
    enrollmentStep = Math.floor(Date.now() / 30_000);
    await status(page, (await submit(page, 'main form[action$="/mfa/confirm"]',
      { password: account.password, code: totp(secret, enrollmentStep) })).status(), 200, 'staff MFA confirmation');
    await expect(page.locator('main textarea').first()).toBeVisible();
    expect(await login(page, lang, account)).toBe(`/${lang}/mfa/challenge`);
  } else {
    expect(destination).toBe(`/${lang}/mfa/challenge`);
  }

  let code;
  if (secret) {
    const remaining = 30_000 - Date.now() % 30_000;
    if (Math.floor(Date.now() / 30_000) <= enrollmentStep || remaining < 8_000) {
      await delay(remaining + 500);
    }
    code = totp(secret);
  } else {
    const since = Math.floor(Date.now() / 1000) * 1000;
    await status(page, (await submit(page, 'main form[action$="/mfa/email"]', {})).status(),
      302, 'staff MFA email request');
    const message = await newestMail(page.request, account.email, [localized(lang,
      'Twój kod weryfikacyjny Oteryn', 'Your Oteryn verification code')], since);
    code = message?.Text?.match(/\b[a-f0-9]{64}\b/)?.[0];
    expect(Boolean(code), 'Staff MFA fallback email contained a code').toBe(true);
  }
  await status(page, (await submit(page, 'main form[action$="/mfa/challenge"]', { code })).status(),
    302, 'staff MFA challenge');
  expect(new URL(page.url()).pathname).toBe(`/${lang}/`);
}
