import { randomBytes } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { atPath, expect, password, submit, test, visit, waitUntilReady } from './fixtures.mjs';
import { copy, login, logout, register, verifyEmail } from './identity.mjs';
import { mailLink } from './mail.mjs';
import { safePath } from './transport.mjs';

async function requestReset(page, lang, email) {
  const path = `/${lang}/forgot-password`;
  await visit(page, path);
  await submit(page, path, 302, { email });
  await atPath(page, path);
  const notice = page.locator('main [role=status]');
  await expect(notice, `POST ${path} neutral response`).toHaveText(copy[lang].resetNotice);
  return notice.innerText();
}

async function completeReset(page, lang, link, email, next, accepted) {
  await visit(page, link);
  await submit(page, `/${lang}/reset-password`, 302, {
    email, password: next, password_confirmation: next,
  });
  await atPath(page, accepted ? `/${lang}/login` : safePath(link));
  if (!accepted) {
    await expect.poll(() => new URL(page.url()).pathname === new URL(link, page.url()).pathname, {
      message: 'Reset refusal kept the same form link',
    }).toBe(true);
    await expect(page.locator('main [role=alert]'), `POST /${lang}/reset-password refusal`).toHaveText(copy[lang].resetError);
  }
}

async function resetLink(page, lang, account, mail) {
  await requestReset(page, lang, account.email);
  return mailLink(mail, account.email, copy[lang].resetSubject, lang, 'reset');
}

test('3.1 Password reset switches the usable password and refuses a reused link', { tag: '@mail' }, async ({ mail, page, lang, account }) => {
  await register(page, lang, account);
  await verifyEmail(page, lang, account, mail);
  const link = await resetLink(page, lang, account, mail);
  const next = password();
  await completeReset(page, lang, link, account.email, next, true);
  await completeReset(page, lang, link, account.email, password(), false);
  await login(page, lang, account, 'login');
  await login(page, lang, { ...account, password: next });
});

test('3.2 A random reset token is refused', async ({ page, lang, account }) => {
  const link = `/${lang}/reset-password/${randomBytes(32).toString('hex')}`;
  await completeReset(page, lang, link, account.email, password(), false);
});

test('3.4 A known and an unknown email receive the same neutral reset response', async ({ page, lang, account }) => {
  await register(page, lang, account);
  const known = await requestReset(page, lang, account.email);
  const unknown = await requestReset(page, lang, `bb-${randomBytes(12).toString('hex')}@example.test`);
  expect(unknown).toBe(known);
});

test('3.5 An unknown email receives the neutral reset response', async ({ page, lang, account }) => {
  await requestReset(page, lang, account.email);
});

test('3.3 An expired unused reset token is refused', { tag: ['@mail', '@expiry'] }, async ({ mail, page, lang, account }) => {
  test.setTimeout(70 * 60_000);
  await register(page, lang, account);
  await verifyEmail(page, lang, account, mail);
  const link = await resetLink(page, lang, account, mail);
  // The delivered token has a 60-minute server lifetime. Wait in real time.
  await delay(61 * 60_000);
  await waitUntilReady(mail);
  await login(page, lang, account);
  await logout(page, lang);
  await completeReset(page, lang, link, account.email, password(), false);
});
