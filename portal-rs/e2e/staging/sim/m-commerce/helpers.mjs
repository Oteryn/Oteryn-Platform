import { createHmac, randomBytes } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { expect } from '@playwright/test';

export const origin = 'https://staging.oteryn.com';
const host = new URL(origin).host;
export const proxy = process.env.HTTPS_PROXY ? { server: process.env.HTTPS_PROXY } : undefined;

function accessHeaders() {
  const id = process.env.STAGING_CF_ACCESS_CLIENT_ID;
  const secret = process.env.STAGING_CF_ACCESS_CLIENT_SECRET;
  if (!id || !secret) throw new Error('Staging Cloudflare Access credentials are required');
  return { 'cf-access-client-id': id, 'cf-access-client-secret': secret };
}

function stagingUrl(path) {
  const url = new URL(path, origin);
  if (url.protocol !== 'https:' || url.host !== host || url.username || url.password) {
    throw new Error('Expected an HTTPS staging URL');
  }
  return url;
}

export async function stagingBrowser(context) {
  const access = accessHeaders();
  await context.route('**/*', async route => {
    const browserRequest = route.request();
    const url = new URL(browserRequest.url());
    if (url.host !== host || url.protocol !== 'https:') {
      return route.continue();
    }

    // continue({ headers }) would carry header overrides into redirects.
    // Fetch exactly one hop and let Chromium perform redirects itself.
    try {
      const response = await route.fetch({
        headers: { ...browserRequest.headers(), ...access },
        maxRedirects: 0,
        timeout: 20_000,
      });
      try {
        const location = response.headers().location;
        if (location && response.status() >= 300 && response.status() < 400) {
          const target = new URL(location, url);
          if (target.origin !== origin) return await route.abort('blockedbyclient');
        }
        await route.fulfill({ response });
      } finally {
        await response.dispose();
      }
    } catch {
      await route.abort('failed').catch(() => {});
    }
  });
}

export async function stagingRequest(context, path, options = {}) {
  const url = stagingUrl(path);
  try {
    return await context.fetch(url.href, {
      ...options,
      headers: { ...options.headers, ...accessHeaders() },
      maxRedirects: 0,
      timeout: 15_000,
    });
  } catch {
    // Playwright transport errors may embed request headers in their call log.
    throw new Error('Staging HTTP request did not complete');
  }
}

export async function ready(context) {
  let last = 'no response';
  const until = Date.now() + 5 * 60_000;
  do {
    try {
      const response = await stagingRequest(context, '/readyz');
      last = response.status();
      await response.dispose();
      if (last === 200) return;
    } catch {
      last = 'no response';
    }
    await delay(5_000);
  } while (Date.now() < until);
  throw new Error(`Staging readiness did not reach HTTP 200 within five minutes (last: ${last})`);
}

export class AccountSetupUnavailable extends Error {}

export async function mailAvailable(context) {
  const response = await stagingRequest(context, '/__mailpit/api/v1/messages?limit=100');
  const status = response.status();
  await response.dispose();
  if (status === 404 || status === 405) {
    throw new AccountSetupUnavailable(`Mailpit GET /__mailpit/api/v1/messages returned HTTP ${status}`);
  }
  expect(status, 'Mailpit listing').toBe(200);
}

async function latestMail(context, email, since, match, subject) {
  const deadline = Date.now() + 60_000;
  const inspected = new Set();
  while (Date.now() < deadline) {
    const listing = await stagingRequest(context, '/__mailpit/api/v1/messages?limit=100');
    const status = listing.status();
    if (status === 404 || status === 405) {
      await listing.dispose();
      throw new AccountSetupUnavailable(`Mailpit disappeared during account setup (HTTP ${status})`);
    }
    expect(status, 'Mailpit listing').toBe(200);
    const { messages } = await listing.json();
    await listing.dispose();
    const newest = messages.slice(0, 100)
      .filter(message => message.To.some(recipient => recipient.Address.toLowerCase() === email) &&
        (!subject || message.Subject === subject))
      .filter(message => Date.parse(message.Created) >= since && !inspected.has(message.ID))
      .sort((left, right) => Date.parse(right.Created) - Date.parse(left.Created));
    for (const message of newest) {
      const response = await stagingRequest(context, `/__mailpit/api/v1/message/${encodeURIComponent(message.ID)}`);
      expect(response.status(), 'Mailpit message detail').toBe(200);
      const { Text, HTML } = await response.json();
      await response.dispose();
      inspected.add(message.ID);
      const result = match(Text ?? '', HTML ?? '');
      if (result) return result;
    }
    await delay(3_000);
  }
  throw new AccountSetupUnavailable('No matching message reached the test account within 60 seconds');
}

function mailLink(context, email, since, pathPattern) {
  return latestMail(context, email, since, (text, html) => {
    for (const candidate of (text + '\n' + html).match(/https?:\/\/[^\s<>"']+/g) ?? []) {
      let url;
      try {
        url = new URL(candidate.replaceAll('&amp;', '&'));
      } catch {
        continue;
      }
      if (pathPattern.test(url.pathname) &&
          (url.origin === origin || ['localhost', '127.0.0.1'].includes(url.hostname))) {
        return stagingUrl(url.pathname + url.search).href;
      }
    }
  });
}

async function form(page, path, fields) {
  const current = page.url();
  let status = await submit(page, path, fields);
  if (status === 429) {
    await delay(65_000);
    await page.goto(current);
    status = await submit(page, path, fields);
  }
  if (status === 429) {
    throw new AccountSetupUnavailable(`Account setup ${path} rate-limited both attempts (HTTP 429)`);
  }
  return status;
}

async function submit(page, path, fields) {
  const target = page.locator(`main form[action$="${path}"], main form[action*="${path}?"]`);
  for (const [name, value] of Object.entries(fields)) {
    const field = target.locator(`[name="${name}"]`);
    if (value === true) await field.check();
    else await field.fill(value);
  }
  const [submitted] = await Promise.all([
    page.waitForResponse(reply => reply.request().method() === 'POST' &&
      new URL(reply.url()).origin === origin && new URL(reply.url()).pathname.endsWith(path),
    { timeout: 45_000 }),
    target.locator('button:not([type=button])').first().click({ timeout: 15_000 }),
  ]);
  await page.waitForLoadState('domcontentloaded');
  return submitted.status();
}

function password() {
  return `Aa1!${randomBytes(20).toString('hex')}`;
}

async function guardedStep(label, action) {
  try {
    return await action();
  } catch (error) {
    if (error instanceof AccountSetupUnavailable) throw error;
    // Credentials, reset links and form values are kept out of assertion output.
    throw new Error(label);
  }
}

function totp(provisioning) {
  const secret = provisioning.startsWith('otpauth:')
    ? new URL(provisioning).searchParams.get('secret') : provisioning;
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  let remaining = 0;
  let bits = 0;
  const bytes = [];
  for (const char of secret.replace(/[=\s]/g, '').toUpperCase()) {
    const value = alphabet.indexOf(char);
    if (value < 0) throw new Error('Unexpected authenticator key format');
    remaining = (remaining << 5) | value;
    bits += 5;
    if (bits >= 8) {
      bits -= 8;
      bytes.push((remaining >> bits) & 255);
    }
  }
  const counter = Buffer.alloc(8);
  counter.writeBigUInt64BE(BigInt(Math.floor(Date.now() / 30_000)));
  const digest = createHmac('sha1', Buffer.from(bytes)).update(counter).digest();
  const offset = digest.at(-1) & 15;
  return ((digest.readUInt32BE(offset) & 0x7fffffff) % 1_000_000).toString().padStart(6, '0');
}

async function login(page, lang, email, pass) {
  await guardedStep('Login did not establish an account session', async () => {
    expect((await page.goto(`/${lang}/login`)).status()).toBe(200);
    expect(await form(page, '/login', { email, password: pass })).toBe(302);
    expect(new URL(page.url()).pathname.endsWith('/login')).toBe(false);
  });
}

export async function registerPlayer(page, lang, suffix) {
  const email = `sim-m-commerce-${randomBytes(6).toString('hex')}-${suffix}@example.test`;
  const pass = password();
  const since = Math.floor(Date.now() / 1000) * 1000;
  expect((await page.goto(`/${lang}/register`)).status(), 'registration form').toBe(200);
  const status = await guardedStep('Registration form did not submit', () => form(page, '/register', {
    email, password: pass, password_confirmation: pass, terms: true, privacy: true,
  }));
  expect(status, 'registration submission').toBe(303);
  await expect(page).toHaveURL(`/${lang}/register/check-inbox`);
  await expect(page.locator('main')).toContainText(lang === 'pl'
    ? 'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.'
    : 'Check your inbox for an e-mail with the next steps.');
  const link = await mailLink(page.request, email, since, /^\/(pl|en)\/verify-email$/);
  await guardedStep('Player email verification did not complete', async () => {
    expect((await page.goto(link)).status()).toBe(200);
    expect(await form(page, '/verify-email', {})).toBe(200);
    await expect(page.locator('main')).toContainText(lang === 'pl'
      ? 'Twój adres e-mail został zweryfikowany.' : 'Your e-mail has been verified.');
  });
  await login(page, lang, email, pass);
  await expect(page.locator('#portal-navigation a[data-account-link]')).toBeAttached();
}

export async function resetStaff(page, lang, role) {
  await mailAvailable(page.request);
  const email = `staff-${role}@example.invalid`;
  const pass = password();
  const since = Math.floor(Date.now() / 1000) * 1000;
  expect((await page.goto(`/${lang}/forgot-password`)).status(), 'public password reset form').toBe(200);
  const status = await guardedStep('Staff password reset request did not submit',
    () => form(page, '/forgot-password', { email }));
  expect(status, 'password reset request submission').toBe(302);
  const link = await mailLink(page.request, email, since, /^\/(pl|en)\/reset-password\/[a-f0-9]+$/);
  await guardedStep('Staff password reset or MFA setup did not complete', async () => {
    expect((await page.goto(link)).status()).toBe(200);
    expect(await form(page, '/reset-password', {
      email, password: pass, password_confirmation: pass,
    })).toBe(302);
    await expect(page).toHaveURL(`/${lang}/login`);
    await login(page, lang, email, pass);
    if (new URL(page.url()).pathname.endsWith('/mfa/challenge')) {
      // A pre-existing enrollment can be challenged by the built-in mail factor if offered.
      const challengeSince = Math.floor(Date.now() / 1000) * 1000;
      const sent = await form(page, '/mfa/email', {});
      expect(sent).toBe(302);
      const subject = lang === 'pl' ? 'Twój kod weryfikacyjny Oteryn' : 'Your Oteryn verification code';
      const code = await latestMail(page.request, email, challengeSince,
        text => text.match(/(?:^|\n)([a-f0-9]{64})\s*$/)?.[1], subject);
      await form(page, '/mfa/challenge', { code });
    }
    await page.goto(`/${lang}/mfa`);
    const enroll = page.locator('main form[action$="/mfa/enroll"]');
    if (await enroll.count()) {
      await form(page, '/mfa/enroll', { password: pass });
      const provisioning = (await page.locator('main .op-prose').innerText()).trim();
      await form(page, '/mfa/confirm', { password: pass, code: totp(provisioning) });
      await expect(page.locator('main textarea[readonly]').first()).toBeVisible();
      await login(page, lang, email, pass);
      await expect(page).toHaveURL(`/${lang}/mfa/challenge`);
      // The enrollment consumes the current counter.
      await delay(30_000 - (Date.now() % 30_000) + 500);
      await form(page, '/mfa/challenge', { code: totp(provisioning) });
    }
    expect(new URL(page.url()).pathname.endsWith('/mfa/challenge')).toBe(false);
    await expect(page.locator('#portal-navigation a[data-account-link]')).toBeAttached();
  });
}
