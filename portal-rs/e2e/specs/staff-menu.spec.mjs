import {test, expect} from '../fixtures.mjs';
import {login, seedAccount} from '../helpers.mjs';
import {enrollMfa, loginWithRecoveryCode, query} from '../staff-rbac-helpers.mjs';

async function grantRole(account, role) {
  const grants = await query(
    'INSERT INTO account_roles(account_id, role_id) SELECT a.id, r.id FROM accounts a CROSS JOIN roles r WHERE a.email=$1 AND r.key=$2 RETURNING account_id',
    [account.email, role],
  );
  expect(grants).toHaveLength(1);
}

function accountMenu(page) {
  return page.locator('dialog').filter({has: page.locator('a.account-overview')});
}

test('staff link opens the localized panel and disappears when the account is suspended', async ({page}, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const account = seedAccount(lang);
  await login(page, lang, account);
  const link = accountMenu(page).locator('a[href="/admin"]');
  await expect(accountMenu(page)).toBeAttached();
  await expect(link).toHaveCount(0);

  await grantRole(account, 'game_viewer');
  expect((await page.goto(`/${lang}/`)).status()).toBe(200);
  const title = lang === 'pl' ? 'Panel zespołu' : 'Staff panel';
  await expect(link).toHaveCount(0);
  expect((await page.request.get('/admin', {maxRedirects: 0})).status()).toBe(403);

  const [code] = await enrollMfa(page, lang, account);
  await loginWithRecoveryCode(page, lang, account, code);
  expect((await page.goto(`/${lang}/`)).status()).toBe(200);
  const menuToggle = page.locator('#portal-menu-toggle');
  if (await menuToggle.isVisible()) await menuToggle.click();
  await page.locator('[data-account-link]').click();
  await expect(link).toHaveText(title);
  await expect(link).toBeVisible();
  const panelURL = new URL('/admin', page.url()).href;
  const [response] = await Promise.all([
    page.waitForResponse(response => response.url() === panelURL && response.request().isNavigationRequest()),
    link.click(),
  ]);
  expect(response.status()).toBe(200);
  expect(await response.headerValue('content-type')).toMatch(/^text\/html\b/);
  await expect(page).toHaveURL(panelURL);
  const heading = page.locator('main h1');
  await expect(heading).toHaveText(title);
  await expect(heading).toBeVisible();

  const suspended = await query(
    "UPDATE accounts SET status='suspended' WHERE email=$1 RETURNING id",
    [account.email],
  );
  expect(suspended).toHaveLength(1);
  expect((await page.goto(`/${lang}/`)).status()).toBe(200);
  await expect(page.locator(`a[href="/${lang}/login"]`).first()).toBeAttached();
  await expect(page.locator('a[href="/admin"]')).toHaveCount(0);
  const refused = await page.request.get(panelURL, {maxRedirects: 0});
  expect(refused.status()).toBe(303);
  expect(refused.headers().location).toBe('/login');
});
