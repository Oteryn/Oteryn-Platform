import crypto from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {readFile} from 'node:fs/promises';
import {test, expect} from '../fixtures.mjs';
import * as OTPAuth from 'otpauth';
import {login, password, submit, pool} from '../helpers.mjs';

function seedAccount(lang) {
  const account = {email: `account-actions-${crypto.randomUUID()}@example.invalid`, password: password(), locale: lang};
  const seed = spawnSync(process.env.PORTAL_E2E_BINARY, ['seed', 'account', JSON.stringify(account)]);
  expect(seed.status, 'create verified local account').toBe(0);
  return account;
}

test('account actions are reachable and return to the correct account page', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const account = seedAccount(lang);
  await login(page, lang, account);
  await page.goto(`/${lang}/`);

  const prefix = `/${lang}`;
  const overview = '/account';
  const exportPath = `${prefix}/account/privacy/export`;
  const closurePath = `${prefix}/account/privacy/delete`;
  const sessions = `${prefix}/account/sessions`;
  const settings = `${prefix}/mfa`;
  const paths = [settings, `${settings}#remembered-devices`, `${prefix}/account/security/email`,
    `${prefix}/account/passkeys`, sessions, exportPath, closurePath];
  const overviewLink = page.locator('main a[href="/account"]');
  const action = path => page.locator(`main a[href="${path}"]`);
  const navigate = async (link, path) => {
    await link.click();
    await expect(page).toHaveURL(url => url.pathname + url.hash === path);
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  };

  const menuToggle = page.locator('#portal-menu-toggle');
  if (await menuToggle.isVisible()) await menuToggle.click();
  await page.locator('#portal-header [data-account-link]').click();
  const menu = page.locator('#podglad-konta');
  await expect(menu).toBeVisible();
  for (const route of paths) {
    await expect(menu.locator(`a[href="${route}"]`)).toBeVisible();
  }
  await navigate(menu.locator('a[href="/account"]'), overview);
  for (const route of paths.slice(0, -2)) {
    await navigate(action(route), route);
    if (route === sessions) {
      await submit(page, 'main form[action$="/others"]');
      await expect(page).toHaveURL(url => url.pathname === sessions);
    }
    await navigate(overviewLink, overview);
  }

  const sudo = (destination) => `${prefix}/account/sudo?return_to=${destination}`;
  await navigate(action(exportPath), exportPath);
  await action(sudo('export')).click();
  const invalid = await page.request.get(`${prefix}/account/sudo?return_to=https://example.invalid`, {maxRedirects: 0});
  expect(invalid.status()).toBe(400);
  await submit(page, 'main form', {current_password: account.password});
  await expect(page).toHaveURL(url => url.pathname === exportPath);
  const downloadEvent = page.waitForEvent('download');
  await page.locator('main form button').click();
  const download = await downloadEvent;
  expect(download.suggestedFilename()).toBe('account-data.json');
  expect(await readFile(await download.path(), 'utf8')).toContain(account.email);
  await navigate(overviewLink, overview);

  await navigate(action(closurePath), closurePath);
  await action(sudo('closure')).click();
  await submit(page, 'main form', {current_password: account.password});
  await expect(page).toHaveURL(url => url.pathname === closurePath);
  await submit(page, 'main form', {current_password: account.password, confirmation: 'TERMINATE'});
  await expect(page).toHaveURL(url => url.pathname === `${prefix}/login`);
  await login(page, lang, account);
  await page.goto(overview);
  await expect(action(closurePath)).toHaveText(lang === 'pl'
    ? 'Oczekujące zamknięcie konta' : 'Pending account closure');
  await navigate(action(closurePath), closurePath);
  await action(sudo('closure')).click();
  await submit(page, 'main form', {current_password: account.password});
  await expect(page).toHaveURL(url => url.pathname === closurePath);
  await submit(page, 'main form[action$="/cancel"]', {current_password: account.password});
  await expect(page).toHaveURL(url => url.pathname === closurePath);
  await expect(page.locator('main input[name="confirmation"]')).toBeVisible();
  await navigate(overviewLink, overview);
});

test('security actions update the signed-in account and provide a way back', async ({page, context}, info) => {
  const lang = info.project.metadata.locale;
  const account = seedAccount(lang);
  await login(page, lang, account);
  await page.goto('/account');
  const clickAction = path => page.locator(`main a[href="${path}"]`).click();
  const overview = () => clickAction('/account');
  const passkeys = `/${lang}/account/passkeys`;
  const loginPath = `/${lang}/login`;
  const mfa = `/${lang}/mfa`;

  const cdp = await context.newCDPSession(page);
  await cdp.send('WebAuthn.enable');
  const {authenticatorId} = await cdp.send('WebAuthn.addVirtualAuthenticator', {
    options: {
      protocol: 'ctap2', transport: 'internal', hasResidentKey: true,
      hasUserVerification: true, isUserVerified: true, automaticPresenceSimulation: true,
    },
  });
  try {
    await clickAction(passkeys);
    await page.locator('#passkey-password').fill(account.password);
    await page.locator('[data-passkey="register"]').click();
    await expect(page).toHaveURL(url => url.pathname === loginPath);
    await login(page, lang, account);
    await page.goto('/account');
    await clickAction(passkeys);
    await expect(page.locator('[data-remove]')).toHaveCount(1);
    await page.locator('#passkey-password').fill(account.password);
    await page.locator('[data-remove]').click();
    await expect(page).toHaveURL(url => url.pathname === loginPath);
    await login(page, lang, account);
    await page.goto('/account');
    await clickAction(passkeys);
    await expect(page.locator('[data-remove]')).toHaveCount(0);
    await overview();
  } finally {
    await cdp.send('WebAuthn.removeVirtualAuthenticator', {authenticatorId});
    await cdp.detach();
  }

  await clickAction(mfa);
  await submit(page, 'main form[action$="/mfa/enroll"]', {password: account.password});
  const provisioning = await page.locator('main .op-prose').innerText();
  const totp = provisioning.startsWith('otpauth:')
    ? OTPAuth.URI.parse(provisioning) : new OTPAuth.TOTP({secret: provisioning});
  const enrollmentCode = totp.generate();
  await submit(page, 'main form[action$="/mfa/confirm"]', {password: account.password, code: enrollmentCode});
  const code = page.locator('main textarea[readonly]').first();
  await expect(code).toBeVisible();
  const recovery = await code.inputValue();
  await clickAction(loginPath);
  await login(page, lang, account);
  await page.waitForURL(url => url.pathname.endsWith('/mfa/challenge'));
  await submit(page, 'main form[action$="/mfa/challenge"]', {code: recovery, remember: true});
  await page.goto('/account');
  await clickAction(`${mfa}#remembered-devices`);
  const devices = 'main form[action$="/mfa/devices"]';
  await expect(page.locator(devices)).toHaveCount(1);
  await submit(page, devices);
  await expect(page).toHaveURL(url => url.pathname === mfa && url.hash === '#remembered-devices');
  await expect(page.locator(devices)).toHaveCount(0);
  await overview();

  const email = `changed-${crypto.randomUUID()}@example.invalid`;
  const emailPath = `/${lang}/account/security/email`;
  await clickAction(emailPath);
  // Enrollment consumed its TOTP. Wait for the next code before requesting a change.
  await expect.poll(() => totp.generate(), {timeout: 32_000}).not.toBe(enrollmentCode);
  await submit(page, 'main form', {email, email_confirmation: email, current_password: account.password, proof: totp.generate()});
  await expect(page).toHaveURL(url => url.pathname === emailPath);
  const db = pool();
  try {
    const result = await db.query(
      'SELECT new_email FROM account_email_changes WHERE old_email=$1 AND new_email=$2',
      [account.email, email],
    );
    expect(result.rowCount).toBe(1);
  } finally {
    await db.end();
  }
  await overview();
});

test('i18n-parity for account action catalogues', async () => {
  const keys = source => [...source.matchAll(/^([a-z][\w-]*)\s*=/gm)].map(([, key]) => key).sort();
  for (const file of [
    'crates/ui/i18n/{}.ftl', 'crates/identity/src/credentials_{}.ftl',
    'templates/account/mfa/{}.ftl', 'templates/account/overview.{}.ftl',
  ]) {
    const [pl, en] = await Promise.all(['pl', 'en'].map(lang =>
      readFile(new URL(`../../${file.replace('{}', lang)}`, import.meta.url), 'utf8'),
    ));
    const scope = file.startsWith('crates/ui') ? key => key.startsWith('shell-account-') : () => true;
    expect(keys(pl).filter(scope), file).toEqual(keys(en).filter(scope));
  }
});
