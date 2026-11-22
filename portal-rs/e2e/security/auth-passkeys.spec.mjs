import { test, expect } from '../fixtures.mjs';
import { login, waitForLoginBudget } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { cookie, expectSession, seedAccount } from './auth-fixtures.mjs';

test('Chromium WebAuthn registers, signs in with, and removes a discoverable passkey', async ({ page, context }, info) => {
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

  try {
    await waitForLoginBudget(7);
    await login(page, lang, account);
    expect((await page.goto(route)).status()).toBe(200);
    await checkCsrf('/register/start', { password: account.password });
    await checkCsrf('/register/finish', { token: '', credential: {} });
    await page.locator('#passkey-password').fill(account.password);
    await ceremony(page.locator('[data-passkey=register]'), '/register/finish');
    await expect(page.locator('[data-passkey=authenticate]')).toBeVisible();

    const [{ id }] = await query('SELECT id FROM account_passkeys WHERE account_id=$1', [account.id]);
    const guest = await cookie(context);
    await checkCsrf('/authenticate/start');
    await checkCsrf('/authenticate/finish', { token: '', credential: {} });
    await ceremony(page.locator('[data-passkey=authenticate]'), '/authenticate/finish');
    const remove = page.locator('[data-remove]');
    await expect(remove).toHaveCount(1);
    const authenticated = await cookie(context);
    expect(authenticated !== guest, 'passkey sign-in rotated the session').toBe(true);
    await expectSession(guest);
    await expectSession(authenticated, account.id);

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
