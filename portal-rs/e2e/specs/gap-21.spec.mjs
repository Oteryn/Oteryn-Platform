import {test, expect} from '../fixtures.mjs';
import {register, login, submit, csrfRejected} from '../helpers.mjs';
import * as OTPAuth from 'otpauth';
import {readFile} from 'node:fs/promises';

const form = (path) => `main form[action$="/mfa/${path}"]`;
const recovery = (page) => page.locator('main textarea[readonly]');
const authenticator = async (page) => new OTPAuth.TOTP({
  secret: (await page.locator('main .op-prose').innerText()).trim(),
  algorithm: 'SHA1', digits: 6, period: 30,
});

async function expectContentFits(page) {
  const width = await page.evaluate(() => document.documentElement.scrollWidth);
  expect(width).toBeLessThanOrEqual(page.viewportSize().width);
}

async function manage(page, operation, fields) {
  const target = page.locator(form(operation));
  if (!await target.isVisible()) {
    const section = page.locator('main details').filter({
      has: page.locator(`form[action$="/mfa/${operation}"]`),
    });
    await section.locator('summary').click();
  }
  await expectContentFits(page);
  await submit(page, form(operation), fields);
}

async function signInWithCode(page, lang, account, code, remember = false) {
  await login(page, lang, account);
  await expect(page).toHaveURL(/\/mfa\/challenge$/);
  await submit(page, form('challenge'), {code, remember});
  await expect(page).not.toHaveURL(/\/mfa\/challenge$/);
}

test('manage two-factor authentication from account security', async ({page, browser}, info) => {
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  const password = account.password;
  await login(page, lang, account);

  await page.goto(`/${lang}/`);
  const securityLink = page.locator(`#podglad-konta a[href="/${lang}/password/change"]`);
  if (!await securityLink.isVisible()) {
    const accountMenu = page.locator('#portal-header [data-account-link]');
    if (!await accountMenu.isVisible()) await page.locator('#portal-menu-toggle').click();
    await accountMenu.click();
  }
  await securityLink.click();
  await page.locator(`main a[href="/${lang}/mfa"]`).click();
  await submit(page, form('enroll'), {password});
  const initialFactor = await authenticator(page);
  await submit(page, form('confirm'), {password, code: initialFactor.generate()});
  await expect(recovery(page)).toHaveCount(8);
  const initialCodes = await recovery(page).allTextContents();

  const otherContext = await browser.newContext({
    baseURL: new URL(page.url()).origin,
    ignoreHTTPSErrors: true,
    extraHTTPHeaders: {'x-portal-e2e-login-peer': '127.0.0.2'},
  });
  try {
    const other = await otherContext.newPage();
    await signInWithCode(other, lang, account, initialCodes[0], true);
    await signInWithCode(page, lang, account, initialCodes[1]);
    await page.goto(`/${lang}/mfa`);
    await expectContentFits(page);

    await csrfRejected(page, `/${lang}/mfa/rotate`, {password, code: initialCodes[2]});
    await manage(page, 'rotate', {password, code: initialCodes[0]});
    await expect(page.locator('main [role=alert]')).toBeVisible();
    await manage(page, 'rotate', {password, code: initialCodes[2]});
    await expect(recovery(page)).toHaveCount(8);
    const rotated = await recovery(page).allTextContents();
    expect(rotated.every((code) => !initialCodes.includes(code))).toBe(true);

    await other.goto(`/${lang}/mfa`);
    await expect(other).toHaveURL(/\/login$/);
    await login(other, lang, account);
    await expect(other).toHaveURL(/\/mfa\/challenge$/);
    await submit(other, form('challenge'), {code: initialCodes[3]});
    await expect(other.locator('main [role=alert]')).toBeVisible();

    await signInWithCode(page, lang, account, rotated[0]);
    await page.goto(`/${lang}/mfa`);
    await manage(page, 'replace', {password, code: rotated[1]});
    const newFactor = await authenticator(page);
    await expectContentFits(page);
    const confirm = page.locator(form('replace/confirm'));
    const fields = { _csrf: await confirm.locator('[name=_csrf]').inputValue(), code: 'invalid' };
    const malformed = await page.request.post(`/${lang}/mfa/replace/confirm`, {
      form: fields, headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
    });
    expect(malformed.status()).toBe(302);
    expect(malformed.headers().location).toBe(`/${lang}/mfa/replace`);
    await page.reload();
    await expect(page.locator('main [role=alert]')).toBeVisible();
    await submit(page, form('replace/confirm'), {code: newFactor.generate()});
    await expect(recovery(page)).toHaveCount(8);
    const replacementCodes = await recovery(page).allTextContents();
    await expectContentFits(page);

    // Use the next counter accepted by the new factor after confirmation.
    await login(page, lang, account);
    await expect(page).toHaveURL(/\/mfa\/challenge$/);
    const next = newFactor.generate({timestamp: Date.now() + 30_000});
    await submit(page, form('challenge'), {code: next});
    await expect(page).not.toHaveURL(/\/mfa\/challenge$/);

    await page.goto(`/${lang}/mfa`);
    await manage(page, 'disable', {password, code: replacementCodes[0]});
    await expect(page).toHaveURL(/\/login$/);
    await login(page, lang, account);
    await expect(page).not.toHaveURL(/\/mfa\/challenge$/);
    await page.goto(`/${lang}/mfa`);
    await expect(page.locator(form('enroll'))).toBeVisible();
    await expect.poll(async () => {
      const response = await page.request.get('/__mail');
      const messages = await response.json();
      return messages.filter((message) => message.to.includes(account.email)
        && /^Subject: Oteryn 2FA\r?$/m.test(message.raw)).length;
    }, {timeout: 30_000}).toBe(3);
  } finally {
    await otherContext.close();
  }
});

test('i18n-parity for account security and its change notifications', async () => {
  for (const [directory, prefix, featureKey] of [
    ['../../templates/account/mfa/', '', 'mfa-disable'],
    ['../../crates/identity/src/', 'credentials_', 'mfa'],
    ['../../crates/notify/locales/', '', 'mfa-subject'],
  ]) {
    const keys = async (lang) => {
      const source = await readFile(new URL(`${directory}${prefix}${lang}.ftl`, import.meta.url), 'utf8');
      return [...source.matchAll(/^([a-z][a-z0-9-]*)\s*=/gm)].map((match) => match[1]).sort();
    };
    const english = await keys('en');
    expect(english).toContain(featureKey);
    expect(await keys('pl')).toEqual(english);
  }
});
