import { createHmac, randomBytes } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { expect, test as base } from '@playwright/test';

export const origin = 'https://staging.oteryn.com';
const mail = '/__mailpit/api/v1';
const mailList = mail + '/messages?limit=100';

function accessHeaders() {
  const id = process.env.STAGING_CF_ACCESS_CLIENT_ID;
  const secret = process.env.STAGING_CF_ACCESS_CLIENT_SECRET;
  if (!id || !secret) throw new Error('Staging Cloudflare Access environment variables are required');
  return { 'CF-Access-Client-Id': id, 'CF-Access-Client-Secret': secret };
}

export async function stagingRequest(request, path, options = {}) {
  const url = new URL(path, origin);
  if (url.origin !== origin) throw new Error('Expected a staging URL');
  try {
    return await request.fetch(url.href, {
      ...options,
      headers: { ...options.headers, ...accessHeaders() },
      maxRedirects: 0,
      timeout: 15_000,
    });
  } catch {
    throw new Error('Staging HTTP request failed before an HTTP response');
  }
}

async function protect(context) {
  const headers = accessHeaders();
  await context.route('**/*', async route => {
    const request = route.request();
    const url = new URL(request.url());
    if (url.host !== 'staging.oteryn.com' || url.protocol !== 'https:') {
      await route.continue();
      return;
    }
    // Sending these headers through continue() would also send them on redirects.
    try {
      const response = await route.fetch({
        headers: { ...await request.allHeaders(), ...headers },
        maxRedirects: 0,
        timeout: 15_000,
      });
      await route.fulfill({ response });
    } catch {
      // Playwright transport errors can include request headers.
      await route.abort();
    }
  });
}

export const test = base.extend({
  actors: async ({ browser, context, baseURL, locale, viewport, proxy, ignoreHTTPSErrors }, use) => {
    await protect(context);
    const extra = [];
    try {
      await use(async () => {
        const actor = await browser.newContext({
          baseURL, locale, viewport, proxy, ignoreHTTPSErrors, serviceWorkers: 'block',
        });
        extra.push(actor);
        await protect(actor);
        return actor.newPage();
      });
    } finally {
      await Promise.all(extra.map(actor => actor.close()));
    }
  },
  page: async ({ page, actors }, use) => {
    await use(page);
  },
});

export async function waitReady(request) {
  await expect.poll(async () => {
    try {
      const response = await stagingRequest(request, '/readyz');
      const status = response.status();
      await response.dispose();
      return status;
    } catch {
      return 0;
    }
  }, { timeout: 300_000, intervals: [1_000, 5_000], message: 'Staging readiness' }).toBe(200);
}

test.beforeEach(async ({ request }) => waitReady(request));

export { expect };
export const pathOf = page => new URL(page.url()).pathname;
export const language = (lang, pl, en) => lang === 'pl' ? pl : en;

export function colleague() {
  return {
    email: 'sim-platform-admin-' + randomBytes(8).toString('hex') + '-1@example.test',
    password: 'Qa!' + randomBytes(24).toString('base64url'),
  };
}

export async function visit(page, path, status = 200) {
  const destination = new URL(path, origin);
  if (destination.origin !== origin) throw new Error('Expected a staging page');
  let response;
  try {
    response = await page.goto(destination.href, { waitUntil: 'domcontentloaded' });
  } catch {
    throw new Error('Staging navigation failed before an HTTP response');
  }
  expect(response?.status(), 'Navigation HTTP status').toBe(status);
  return response;
}

export async function form(page, selector, values, button = 'button', {
  sensitive = false, registration = false,
} = {}) {
  const target = page.locator(selector);
  const submit = async () => {
    for (const [name, value] of Object.entries(values)) {
      const field = target.locator('[name="' + name + '"]');
      if (typeof value === 'boolean') await field.setChecked(value);
      else if (await field.evaluate(node => node.tagName === 'SELECT')) await field.selectOption(value);
      else await field.fill(value);
    }
    const action = await target.evaluate(node => new URL(node.action).pathname);
    const result = page.waitForResponse(response =>
      response.request().method() === 'POST' && new URL(response.url()).pathname === action,
    { timeout: registration ? 250_000 : 25_000 });
    const [response] = await Promise.all([result, target.locator(button).click()]);
    return response;
  };
  for (let attempt = 0; attempt < 3; attempt++) {
    const formUrl = page.url();
    let rateLimit;
    const onResponse = response => {
      if (response.status() === 429 && new URL(response.url()).pathname === '/register/abuse/challenge') {
        rateLimit = response;
      }
    };
    if (registration) page.on('response', onResponse);
    let response;
    try {
      response = await submit();
    } catch (error) {
      if (!sensitive) throw error;
      throw new Error('Could not submit the credential form');
    } finally {
      if (registration) page.off('response', onResponse);
    }
    if (response.status() === 429) rateLimit = response;
    if (!rateLimit) return response;
    test.skip(attempt === 2, 'Shared staging rate limit persisted after two waits');
    const retryAfter = Number(await rateLimit.headerValue('retry-after'));
    await delay(Math.min(120, Math.max(65, retryAfter || 0)) * 1_000);
    await visit(page, formUrl);
  }
}

export async function waitForPath(page, path) {
  await expect.poll(() => pathOf(page)).toBe(path);
}

export async function checkMail(request) {
  const response = await stagingRequest(request, mailList);
  const status = response.status();
  await response.dispose();
  test.skip(status === 404 || status === 405, 'Staging Mailpit HTTP ' + status);
  expect(status, 'Mailpit list status').toBe(200);
}

async function mailMessage(request, email, subject, since) {
  const deadline = Date.now() + 60_000;
  do {
    const response = await stagingRequest(request, mailList);
    const status = response.status();
    if (status !== 200) {
      await response.dispose();
      test.skip(status === 404 || status === 405, 'Staging Mailpit HTTP ' + status);
      expect(status, 'Mailpit list status').toBe(200);
    }
    const { messages } = await response.json();
    await response.dispose();
    const message = messages
      .filter(entry => subject.test(entry.Subject) && Date.parse(entry.Created) >= since &&
        entry.To.some(recipient => recipient.Address.toLowerCase() === email.toLowerCase()))
      .sort((a, b) => Date.parse(b.Created) - Date.parse(a.Created))
      .at(0);
    if (message) {
      const detail = await stagingRequest(request, mail + '/message/' + encodeURIComponent(message.ID));
      const status = detail.status();
      expect(status, 'Mailpit message status').toBe(200);
      const { Text, HTML } = await detail.json();
      await detail.dispose();
      return (Text ?? '') + '\n' + (HTML ?? '');
    }
    await delay(Math.max(0, Math.min(2_000, deadline - Date.now())));
  } while (Date.now() < deadline);
}

async function mailLink(request, email, subject, since, path) {
  const message = await mailMessage(request, email, subject, since);
  if (!message) return;
  const links = message.match(/https?:\/\/[^\s<>"']+/g) ?? [];
  for (const candidate of links) {
    let link;
    try {
      link = new URL(candidate.replaceAll('&amp;', '&'));
    } catch {
      continue;
    }
    if (['staging.oteryn.com', 'localhost', '127.0.0.1'].includes(link.hostname) &&
        path.test(link.pathname)) {
      return link.pathname + link.search;
    }
  }
  throw new Error('The newest matching message did not contain the expected portal link');
}

export async function requestReset(page, lang, email) {
  await visit(page, '/' + lang + '/forgot-password');
  const response = await form(page, 'main form', { email });
  expect(response.status()).toBe(302);
  await waitForPath(page, '/' + lang + '/forgot-password');
  await expect(page.getByRole('status')).toHaveText(language(lang,
    'Jeśli konto z tym adresem istnieje, wysłaliśmy link do zresetowania hasła.',
    'If an account exists for that email, a password reset link has been sent.'));
}

export async function signIn(page, lang, account, request) {
  await visit(page, '/' + lang + '/login');
  const response = await form(page, 'main form[action$="/login"]',
    { email: account.email, password: account.password }, 'button', { sensitive: true });
  expect(response.status(), 'Login accepted').toBe(302);
  if (/\/mfa\/challenge$/.test(await response.headerValue('location'))) {
    await waitForPath(page, '/' + lang + '/mfa/challenge');
    let code = account.codes?.shift() ?? (account.secret && await totp(account.secret));
    if (!code && request) {
      const since = Date.now() - 2_000;
      const email = await form(page, 'main form[action$="/mfa/email"]', {});
      expect(email.status(), 'Email challenge requested').toBe(302);
      const message = await mailMessage(request, account.email,
        /^(Your Oteryn verification code|Twój kod weryfikacyjny Oteryn)$/, since);
      code = message?.match(/\b[0-9]{6}\b/)?.[0];
    }
    expect(Boolean(code), 'MFA credential available').toBe(true);
    const result = await form(page, 'main form[action$="/mfa/challenge"]', { code },
      'button', { sensitive: true });
    expect(result.status(), 'MFA challenge accepted').toBe(302);
  }
  await waitForPath(page, '/' + lang + '/');
}

export async function founder(page, request, lang) {
  await checkMail(request);
  const account = { ...colleague(), email: 'founder@example.invalid' };
  // Reset delivery language follows the account preference, which may differ from this project.
  const since = Date.now() - 2_000;
  await requestReset(page, lang, account.email);
  const linkPattern = /^\/(pl|en)\/reset-password(?:\/|$)/;
  const link = await mailLink(request, account.email,
    /^Reset your Oteryn password$|^Zresetuj hasło do Oteryn$/, since, linkPattern);
  test.skip(!link, 'No founder reset mail arrived within 60 seconds');
  const resetLang = link.split('/')[1];
  await visit(page, link);
  const reset = await form(page, 'main form', {
    email: account.email, password: account.password, password_confirmation: account.password,
  }, 'button', { sensitive: true });
  expect(reset.status()).toBe(302);
  expect((await reset.headerValue('location')) === '/' + resetLang + '/login',
    'Password reset returned to sign-in').toBe(true);
  await signIn(page, lang, account, request);
  if ((await stagingRequest(page.request, '/admin')).status() === 403) {
    await enroll(page, lang, account);
  }
  await visit(page, '/admin');
  return account;
}

export async function submitRegistration(page, account, password = account.password) {
  return form(page, 'main form[action$="/register"]', {
    email: account.email, password: account.password, password_confirmation: password,
    terms: true, privacy: true,
  }, 'button', { sensitive: true, registration: true });
}

export async function register(page, request, lang) {
  const account = colleague();
  const since = Date.now() - 2_000;
  await visit(page, '/' + lang + '/register');
  const response = await submitRegistration(page, account);
  expect(response.status(), 'Registration accepted').toBe(303);
  await waitForPath(page, '/' + lang + '/register/check-inbox');
  const link = await mailLink(request, account.email,
    /.+/, since, /^\/(pl|en)\/verify-email$/);
  expect(Boolean(link), 'Registration verification mail arrived').toBe(true);
  await visit(page, link);
  const verification = await form(page, 'main form[action*="/verify-email?"]', {},
    'button', { sensitive: true });
  expect(verification.status(), 'Verification accepted').toBe(200);
  await expect(page.getByRole('status')).toHaveText(language(lang,
    'Twój adres e-mail został zweryfikowany.', 'Your e-mail has been verified.'));
  return account;
}

async function totp(secret) {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  let bits = 0;
  let buffer = 0;
  const bytes = [];
  for (const char of secret.toUpperCase().replace(/[=\s]/g, '')) {
    const value = alphabet.indexOf(char);
    if (value < 0) throw new Error('Invalid authenticator setup key');
    buffer = (buffer << 5) | value;
    bits += 5;
    if (bits >= 8) {
      bits -= 8;
      bytes.push((buffer >> bits) & 255);
    }
  }
  const remaining = 30_000 - Date.now() % 30_000;
  if (remaining < 5_000) await delay(remaining + 150);
  const time = Buffer.alloc(8);
  time.writeBigUInt64BE(BigInt(Math.floor(Date.now() / 30_000)));
  const digest = createHmac('sha1', Buffer.from(bytes)).update(time).digest();
  return String((digest.readUInt32BE(digest.at(-1) & 15) & 0x7fffffff) % 1_000_000)
    .padStart(6, '0');
}

export async function enroll(page, lang, account) {
  await visit(page, '/' + lang + '/mfa');
  const start = await form(page, 'main form[action$="/mfa/enroll"]',
    { password: account.password }, 'button', { sensitive: true });
  expect(start.status(), 'Authenticator enrollment started').toBe(200);
  account.secret = await page.locator('main .op-prose').innerText();
  const confirm = await form(page, 'main form[action$="/mfa/confirm"]',
    { password: account.password, code: await totp(account.secret) }, 'button', { sensitive: true });
  expect(confirm.status(), 'Authenticator enrollment accepted').toBe(200);
  const codes = page.locator('main textarea[readonly]');
  await expect(codes).toHaveCount(8);
  account.codes = await codes.allTextContents();
  await signIn(page, lang, account);
}
