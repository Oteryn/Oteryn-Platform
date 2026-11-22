import {test, expect} from '../fixtures.mjs';
import {login, register} from '../helpers.mjs';
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

test('account menu shows a localized staff link only with admin.access', async ({page}, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await login(page, lang, account);
  const link = accountMenu(page).locator('a[href="/admin"]');
  await expect(accountMenu(page)).toBeAttached();
  await expect(link).toHaveCount(0);

  await grantRole(account, 'game_viewer');
  expect((await page.goto(`/${lang}/`)).status()).toBe(200);
  await expect(link).toHaveText(lang === 'pl' ? 'Panel zespołu' : 'Staff panel');
});

test('cold atlas sends no-store for document and direct GET requests', async ({request}, info) => {
  const lang = info.project.metadata.locale;
  for (const accept of ['text/html', '*/*']) {
    const atlas = await request.get(`/${lang}/atlas`, {headers: {accept}});
    expect(atlas.status()).toBe(503);
    expect(atlas.headers()['cache-control']).toBe('no-store');
  }
});
