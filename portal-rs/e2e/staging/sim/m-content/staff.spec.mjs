import { createHmac } from 'node:crypto';
import { mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';
import { expect } from '@playwright/test';
import { inboxPath, mailLink, password, protect, safeForm, send, status, test, visit } from './helpers.mjs';

function totp(secret) {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  const characters = secret.toUpperCase().replace(/[\s=]/g, '');
  if (!/^[A-Z2-7]+$/.test(characters)) throw new Error('Invalid authenticator key format.');
  const bits = [...characters].map(character => alphabet.indexOf(character).toString(2).padStart(5, '0')).join('');
  const key = Buffer.from((bits.match(/.{8}/g) ?? []).map(byte => Number.parseInt(byte, 2)));
  const counter = Buffer.alloc(8);
  counter.writeBigUInt64BE(BigInt(Math.floor(Date.now() / 30_000)));
  const hash = createHmac('sha1', key).update(counter).digest();
  return String((hash.readUInt32BE(hash.at(-1) & 15) & 0x7fffffff) % 1_000_000).padStart(6, '0');
}

async function signIn(page, email, chosenPassword) {
  await visit(page, '/en/login', 200);
  await safeForm(page.locator('main form[action$="/login"]'), { email, password: chosenPassword });
}

async function staffSession(page, link, email) {
  const chosenPassword = password();
  try {
    const resetPage = await page.goto(link, { waitUntil: 'domcontentloaded' });
    if (resetPage.status() !== 200) throw new Error('Reset form unavailable.');
    const reset = page.locator('main form[action="/en/reset-password"], main form[action="/pl/reset-password"]');
    const [result] = await Promise.all([
      page.waitForResponse(response => response.request().method() === 'POST'
        && /\/(?:pl|en)\/reset-password$/.test(new URL(response.url()).pathname)),
      safeForm(reset, { email, password: chosenPassword, password_confirmation: chosenPassword }),
    ]);
    if (result.status() === 503) return 'password reset unavailable';
    if (result.status() === 429) return 'shared reset limit';
    if (!/\/(?:pl|en)\/login$/.test(new URL(page.url()).pathname)) throw new Error('Reset did not finish.');

    await signIn(page, email, chosenPassword);
    if (new URL(page.url()).pathname.endsWith('/mfa/challenge')) return 'existing enrollment';
    if (new URL(page.url()).pathname !== '/en/') throw new Error('Staff sign-in did not finish.');

    await visit(page, '/en/mfa', 200);
    await safeForm(page.locator('main form[action="/en/mfa/enroll"]'), { password: chosenPassword });
    const secret = await page.locator('main .op-prose').innerText();
    await safeForm(page.locator('main form[action="/en/mfa/confirm"]'), { password: chosenPassword, code: totp(secret) });
    if (!await page.locator('main textarea[readonly]').count()) throw new Error('Enrollment did not finish.');

    // Confirmation can consume the current authenticator step.
    await delay(30_000 - (Date.now() % 30_000) + 1000);
    await signIn(page, email, chosenPassword);
    if (new URL(page.url()).pathname !== '/en/mfa/challenge') throw new Error('MFA challenge unavailable.');
    await safeForm(page.locator('main form[action="/en/mfa/challenge"]'), { code: totp(secret) });
    if (new URL(page.url()).pathname !== '/en/') throw new Error('MFA sign-in did not finish.');
    return 'signed in';
  } catch {
    throw new Error('The browser did not complete staff recovery, sign-in and MFA.');
  }
}

test('Content editor recovery and CMS entry', async ({ browser, request }, info) => {
  test.skip(info.project.name !== 'en-1440', 'Recovery uses one shared staging staff account.');
  const mail = await send(request, 'GET', inboxPath);
  const mailStatus = mail.status();
  await mail.dispose();
  test.skip([404, 405].includes(mailStatus), 'Staging Mailpit is unavailable.');
  expect(mailStatus).toBe(200);

  const context = await browser.newContext({
    baseURL: info.project.use.baseURL,
    ignoreHTTPSErrors: info.project.use.ignoreHTTPSErrors,
    proxy: info.project.use.proxy,
    serviceWorkers: 'block',
    locale: 'en',
    viewport: { width: 1440, height: 900 },
  });
  try {
    await protect(context);
    const page = await context.newPage();
    const email = 'staff-content-editor@example.invalid';
    const requestedAt = Date.now() - 2000;
    await visit(page, '/en/forgot-password', 200, 'Reset password');
    await safeForm(page.locator('main form[action="/en/forgot-password"]'), { email });
    await expect(page.locator('main [role=status]')).toHaveText(
      'If an account exists for that email, a password reset link has been sent.',
    );
    const link = await mailLink(request, email, requestedAt, /Reset your Oteryn password|Zresetuj hasło do Oteryn/, /^\/(?:pl|en)\/reset-password\/[a-f0-9]+$/);
    test.skip(!link, 'The nominated staging account did not deliver reset mail within 60 seconds.');

    const session = await staffSession(page, link, email);
    // Password recovery does not grant access to a pre-existing authenticator.
    test.skip(session === 'existing enrollment', 'The account already has an authenticator.');
    test.skip(session === 'shared reset limit', 'The shared staging password-reset budget is exhausted.');
    if (session === 'password reset unavailable') {
      await expect.soft(page.locator('main h1')).toHaveText(/Temporarily unavailable|Usługa chwilowo niedostępna/);
      const folder = new URL('./artifacts/', import.meta.url);
      await mkdir(folder, { recursive: true });
      await page.screenshot({ path: fileURLToPath(new URL('staff-recovery.png', folder)), timeout: 15_000 });
    }
    expect(session).toBe('signed in');
    await visit(page, '/en/account', 200);
    for (const pagePath of ['/admin', '/admin/news', '/admin/pages', '/admin/events',
      '/admin/announcements', '/admin/media', '/admin/wiki']) {
      await status(page.request, 'GET', pagePath, 404);
    }
  } finally {
    await context.close();
  }
});
