import { test, expect } from '../fixtures.mjs';
import { login, waitForLoginBudget } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { cookie, expectSession, otherBrowser, seedAccount } from './auth-fixtures.mjs';

test.use({ trace: 'off', screenshot: 'off' });

test('Chromium WebAuthn registers, signs in with, and removes a discoverable passkey', async ({ page, context, browser }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seedAccount(lang);
  const route = `/${lang}/account/passkeys`;
  const cdp = await context.newCDPSession(page);
  await cdp.send('WebAuthn.enable');
  const { authenticatorId } = await cdp.send('WebAuthn.addVirtualAuthenticator', {
    options: {
      protocol: 'ctap2',
      transport: 'internal',
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
    },
  });

  async function checkCsrf(endpoint, values = {}) {
    const response = await page.request.post(route + endpoint, {
      data: values,
      headers: { origin: new URL(page.url()).origin },
      maxRedirects: 0,
    });
    expect(response.status(), `passkey POST ${endpoint} requires CSRF`).toBe(403);
  }
  async function ceremony(button, endpoint) {
    const request = page.waitForResponse(response => response.request().method() === 'POST' &&
      new URL(response.url()).pathname === route + endpoint);
    await button.click();
    expect((await request).status()).toBe(200);
  }

  async function finish(button, endpoint) {
    let refusal;
    // The browser supplies the real in-flight WebAuthn credential and challenge.
    // Send that same body without CSRF before letting its original request through.
    await page.route(url => url.pathname === route + endpoint, async intercepted => {
      const headers = await intercepted.request().allHeaders();
      delete headers['x-csrf-token'];
      const response = await intercepted.fetch({ headers, maxRedirects: 0 });
      refusal = response.status();
      await response.dispose();
      await intercepted.continue();
    }, { times: 1 });
    const response = page.waitForResponse(result => result.request().method() === 'POST' &&
      new URL(result.url()).pathname === route + endpoint);
    await button.click();
    const completed = await response;
    expect(refusal, `valid ${endpoint} requires CSRF`).toBe(403);
    expect(completed.status()).toBe(200);
  }

  try {
    await waitForLoginBudget(9);
    await login(page, lang, account);
    expect((await page.goto(route)).status()).toBe(200);
    await checkCsrf('/register/start', { password: account.password });
    await page.locator('#passkey-password').fill(account.password);
    await finish(page.locator('[data-passkey=register]'), '/register/finish');
    await expect(page.locator('[data-passkey=authenticate]')).toBeVisible();

    const [{ id }] = await query('SELECT id FROM account_passkeys WHERE account_id=$1', [account.id]);
    const guest = await cookie(context);
    await checkCsrf('/authenticate/start');
    await finish(page.locator('[data-passkey=authenticate]'), '/authenticate/finish');
    const remove = page.locator('[data-remove]');
    await expect(remove).toHaveCount(1);
    const authenticated = await cookie(context);
    expect(authenticated !== guest, 'passkey sign-in rotated the session').toBe(true);
    await expectSession(guest);
    await expectSession(authenticated, account.id);

    const outsider = await seedAccount(lang);
    const second = await otherBrowser(browser, info);
    try {
      const foreign = await second.newPage();
      await login(foreign, lang, outsider);
      await expectSession(await cookie(second), outsider.id);
      expect((await foreign.goto(route)).status()).toBe(200);
      const csrf = await foreign.locator('#passkeys').getAttribute('data-csrf');
      expect(Boolean(csrf), 'the other account has its own CSRF token').toBe(true);
      const refused = await foreign.request.post(`${route}/${id}/remove`, {
        data: { password: outsider.password },
        headers: { origin: new URL(foreign.url()).origin, 'x-csrf-token': csrf },
        maxRedirects: 0,
      });
      expect(refused.status()).toBe(403);
      expect(await query('SELECT id FROM account_passkeys WHERE account_id=$1', [account.id]))
        .toEqual([{ id }]);
      await expectSession(authenticated, account.id);
    } finally {
      await second.close();
    }

    await checkCsrf(`/${id}/remove`, { password: account.password });
    await page.locator('#passkey-password').fill(account.password);
    await ceremony(remove, `/${id}/remove`);
    await expect(page.locator('[data-passkey=authenticate]')).toBeVisible();
    expect((await query('SELECT id FROM account_passkeys WHERE account_id=$1', [account.id])).length).toBe(0);
    await expectSession(authenticated);
  } finally {
    await cdp.send('WebAuthn.removeVirtualAuthenticator', { authenticatorId });
    await cdp.detach();
  }
});
