import { test, expect } from '../fixtures.mjs';
import crypto from 'node:crypto';
import { readFileSync } from 'node:fs';
import { csrfRejected } from '../helpers.mjs';
import {
 query, seededAccount, seededLogin, seededMfaAccount,
} from '../staff-rbac-helpers.mjs';

test('account holder sets email categories and language', async ({ page }, info) => {
 const lang = info.project.metadata.locale;
 const account = await seededAccount(lang);
 await seededLogin(page, lang, account);
 const path = `/${lang}/account/notifications`;
 expect((await page.goto(`/${lang}/`)).status()).toBe(200);
 if (info.project.use.viewport.width <= 960) await page.locator('#portal-menu-toggle').click();
 await page.locator('[data-account-link]').click();
 await page.locator(`#podglad-konta a[href="${path}"]`).click();
 const form = page.locator('main form');
 const mailLanguage = lang === 'pl' ? 'en' : 'pl';
 await expect(form.locator('[name=mail_locale]')).toHaveValue(lang);
 await expect(form.locator('[name=security_email]')).toHaveCount(0);
 for (const name of ['news_email', 'support_email', 'events_email', 'forum_digest_email']) {
  await expect(form.locator(`[name=${name}]`)).not.toBeChecked();
  await form.locator(`[name=${name}]`).check();
 }
 await form.locator('[name=mail_locale]').selectOption(mailLanguage);
 await csrfRejected(page, path, { mail_locale: mailLanguage, news_email: 'true' });
 await form.locator('button[type=submit]').click();
 await expect(page).toHaveURL(new RegExp(`${path}$`));
 await page.reload();
 await expect(form.locator('[name=mail_locale]')).toHaveValue(mailLanguage);
 for (const name of ['news_email', 'support_email', 'events_email', 'forum_digest_email']) {
  await expect(form.locator(`[name=${name}]`)).toBeChecked();
 }
 expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
  await page.evaluate(() => innerWidth),
 );
});

function assertFits(page) {
 return page.evaluate(() => document.documentElement.scrollWidth <= innerWidth);
}

async function postForm(page, selector, fields = {}) {
 const form = page.locator(selector);
 const action = await form.getAttribute('action');
 const csrf = await form.locator('[name=_csrf]').inputValue();
 return page.request.post(action, {
  form: { ...fields, _csrf: csrf }, headers: { origin: new URL(page.url()).origin }, maxRedirects: 0,
 });
}

test('staff previews, tests and queues a newsletter for the selected subscribers', async ({ page, browser }, info) => {
 test.setTimeout(180_000);
 const lang = info.project.metadata.locale;
 const targetLanguage = lang === 'pl' ? 'en' : 'pl';
 const staff = await seededMfaAccount(lang);
 const reader = await seededAccount(lang);
 const otherLanguage = await seededAccount(lang);
 const optedOut = await seededAccount(targetLanguage);
 await query(
  "INSERT INTO account_roles (account_id, role_id) SELECT $1, id FROM roles WHERE key = 'platform_admin'",
  [staff.id],
 );
 await query(
  "INSERT INTO account_notification_prefs (account_id, category, email_enabled) VALUES ($1, 'news', true)",
  [otherLanguage.id],
 );
 const readerContext = await browser.newContext({
  baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443',
  locale: lang, viewport: info.project.use.viewport, ignoreHTTPSErrors: true,
 });
 try {
  const readerPage = await readerContext.newPage();
  await seededLogin(readerPage, lang, reader);
  expect((await readerPage.goto(`/${lang}/account/notifications`)).status()).toBe(200);
  await readerPage.locator('[name=mail_locale]').selectOption(targetLanguage);
  await readerPage.locator('[name=news_email]').check();
  await readerPage.locator('[name=support_email]').check();
  await readerPage.locator('main form button').click();
  await expect(readerPage.locator('[name=news_email]')).toBeChecked();

  await seededLogin(page, lang, staff);
  expect((await page.goto('/admin')).status()).toBe(200);
  await page.locator('main a[href="/admin/newsletter"]').click();
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  expect(await assertFits(page)).toBe(true);

  const marker = crypto.randomUUID();
  const drafts = {
   audience: targetLanguage,
   subject_pl: `Oteryn PL ${marker}`,
   body_pl: 'Witaj, **podróżniku**! <i>Aktualności</i>',
   subject_en: `News ${marker}`,
   body_en: 'Hello, **traveler**! <i>Updates</i>',
  };
  const compose = 'main form[action="/admin/newsletter"]';
  await csrfRejected(page, '/admin/newsletter', drafts);
  expect((await postForm(page, compose, { ...drafts, subject_en: ' ' })).status()).toBe(422);
  const saved = await postForm(page, compose, drafts);
  expect(saved.status()).toBe(303);
  const campaign = saved.headers().location;
  await page.goto(campaign);
  await expect(page.locator('main section[lang=pl]')).toContainText('Witaj, podróżniku!');
  await expect(page.locator('main section[lang=en]')).toContainText('Hello, traveler!');
  await expect(page.locator('main section i')).toHaveCount(0);
  expect(await assertFits(page)).toBe(true);

  const testForm = 'main form[action$="/test"]';
  expect((await postForm(page, testForm, { locale: lang })).status()).toBe(409);
  await page.goto(`/${lang}/account/notifications`);
  await page.locator('[name=news_email]').check();
  await page.locator('main form button').click();
  await page.goto(campaign);
  await csrfRejected(page, `${campaign}/test`, { locale: lang });
  expect((await postForm(page, testForm, { locale: lang })).status()).toBe(303);
  expect((await postForm(page, testForm, { locale: lang })).status()).toBe(303);

  const sink = async email => {
   const messages = await (await page.request.get('/__mail')).json();
   return messages.filter(message => message.to.includes(email) && message.raw.includes(marker));
  };
  await expect.poll(async () => (await sink(staff.email)).length, { timeout: 60_000 }).toBe(1);
  await page.reload();
  const send = 'main form[action$="/send"]';
  await csrfRejected(page, `${campaign}/send`);
  expect((await postForm(page, send)).status()).toBe(403);
  expect((await readerPage.request.get(campaign)).status()).toBe(403);

  const confirm = { current_password: staff.password };
  const results = await Promise.all([postForm(page, send, confirm), postForm(page, send, confirm)]);
  expect(results.map(response => response.status())).toEqual([303, 303]);
  await page.reload();
  await expect(page.locator('main form[action$="/send"]')).toHaveCount(0);
  await expect.poll(async () => (await sink(reader.email)).length, { timeout: 60_000 }).toBe(1);
  expect((await sink(staff.email)).length).toBe(1);

  const emails = await query(
   'SELECT recipient, subject, text_body FROM email_outbox WHERE subject IN ($1, $2)',
   [drafts.subject_pl, drafts.subject_en],
  );
  expect(emails.filter(mail => mail.recipient === reader.email).map(mail => mail.subject)).toEqual([
   drafts[`subject_${targetLanguage}`],
  ]);
  expect(emails.some(mail => [otherLanguage.email, optedOut.email].includes(mail.recipient))).toBe(false);
  const events = await query('SELECT action FROM audit_log WHERE target = $1 AND actor = $2 ORDER BY id', [
   campaign.split('/').at(-1), staff.id,
  ]);
  expect(events.map(event => event.action)).toEqual([
   'newsletter.campaign.saved', 'newsletter.campaign.test_queued', 'newsletter.campaign.batch_queued',
  ]);

  const message = (await sink(reader.email))[0];
  expect(message.raw.includes('List-Unsubscribe:')).toBe(true);
  expect(message.raw.includes('List-Unsubscribe-Post: List-Unsubscribe=One-Click')).toBe(true);
  const footer = emails.find(mail => mail.recipient === reader.email).text_body;
  const link = footer.match(/https:\/\/[^\s]+\/unsubscribe\?token=[^\s]+/)?.[0];
  expect(Boolean(link)).toBe(true);
  const anonymous = await browser.newContext({ ignoreHTTPSErrors: true });
  try {
   const screen = await anonymous.newPage();
   expect((await screen.goto(link)).status()).toBe(200);
   await readerPage.reload();
   await expect(readerPage.locator('[name=news_email]')).toBeChecked();
   expect((await screen.request.post(link, { form: { 'List-Unsubscribe': 'One-Click' } })).status()).toBe(200);
   await readerPage.reload();
   await expect(readerPage.locator('[name=news_email]')).not.toBeChecked();
   await expect(readerPage.locator('[name=support_email]')).toBeChecked();
  } finally {
   await anonymous.close();
  }
 } finally {
  await readerContext.close();
 }
});

test('i18n parity for newsletter and notification preferences', () => {
 const localeKeys = file => [...readFileSync(new URL(file, import.meta.url), 'utf8')
  .matchAll(/^([a-z][a-z0-9-]*) = \S/gm)].map(match => match[1]).sort();
 for (const path of ['../../crates/admin/src/newsletter/{lang}.ftl',
  '../../crates/identity/src/account_lifecycle_{lang}.ftl']) {
  const pl = localeKeys(path.replace('{lang}', 'pl'));
  const en = localeKeys(path.replace('{lang}', 'en'));
  expect(pl.length).toBeGreaterThan(0);
  expect(new Set(pl).size).toBe(pl.length);
  expect(en).toEqual(pl);
 }
});
