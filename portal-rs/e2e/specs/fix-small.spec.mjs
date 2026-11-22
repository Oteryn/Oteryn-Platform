import {test, expect} from '../fixtures.mjs';
import {login, register, submit} from '../helpers.mjs';
import {action, enrollMfa, loginWithRecoveryCode, query, unique} from '../staff-rbac-helpers.mjs';

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

async function signInWithMfa(page, lang, account) {
  await enrollMfa(page, lang, account);
  const recoveryCode = page.locator('main textarea[readonly]').first();
  await expect(recoveryCode).toBeVisible();
  await loginWithRecoveryCode(page, lang, account, await recoveryCode.inputValue());
}

test('account menu shows a localized staff link only with admin.access', async ({page}, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await login(page, lang, account);
  const link = accountMenu(page).locator('a[href$="/admin"]');
  await expect(accountMenu(page)).toBeAttached();
  await expect(link).toHaveCount(0);

  await grantRole(account, 'game_viewer');
  expect((await page.goto(`/${lang}/`)).status()).toBe(200);
  await expect(link).toHaveText(lang === 'pl' ? 'Panel zespołu' : 'Staff panel');
  await expect(link).toHaveAttribute('href', `https://${process.env.OTERYN_ADMIN_HOST}/admin`);
});

test('cold atlas sends no-store for document and direct GET requests', async ({request}, info) => {
  const lang = info.project.metadata.locale;
  for (const accept of ['text/html', '*/*']) {
    const atlas = await request.get(`/${lang}/atlas`, {headers: {accept}});
    expect(atlas.status()).toBe(503);
    expect(atlas.headers()['cache-control']).toBe('no-store');
  }
});

test('passkey management link follows mandatory login MFA eligibility', async ({page}, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await grantRole(account, 'moderator');
  await login(page, lang, account);
  const path = `/${lang}/account/passkeys`;
  const link = accountMenu(page).locator(`a[href="${path}"]`);
  await expect(link).toHaveCount(1);
  expect((await page.request.get(path)).status()).toBe(200);

  await signInWithMfa(page, lang, account);
  expect((await page.goto(`/${lang}/`)).status()).toBe(200);
  await expect(accountMenu(page)).toBeAttached();
  expect((await page.request.get(path)).status()).toBe(403);
  await expect(link).toHaveCount(0);
});

test('moderator can review reports and is refused Platform admin areas', async ({page}, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await login(page, lang, account);
  const [{id: accountId}] = await query('SELECT id FROM accounts WHERE email=$1', [account.email]);
  expect((await page.goto(`/${lang}/support/reports/create`)).status()).toBe(200);
  const evidence = unique();
  await submit(page, 'main form', {target_reference: accountId, evidence_summary: evidence});
  await expect(page).toHaveURL(/support\/reports\/[a-f0-9-]+$/);
  const reportId = new URL(page.url()).pathname.split('/').at(-1);

  await grantRole(account, 'moderator');
  await signInWithMfa(page, lang, account);
  expect((await page.request.get('/admin')).status()).toBe(200);

  for (const path of ['/admin/roles', '/admin/console/access', '/admin/news', '/admin/analytics']) {
    expect((await page.request.get(path)).status(), path).toBe(403);
  }
  expect((await page.request.get(`/admin/${lang}/party`, {maxRedirects: 0})).status()).toBe(403);
  const queue = '/admin/moderation/reports';
  expect((await page.request.get(queue, {maxRedirects: 0})).status()).toBe(200);
  expect((await page.goto(`${queue}/${reportId}`)).status()).toBe(200);
  await expect(page.locator('main')).toContainText(evidence);

  const outcome = unique();
  await action(page, 'form:has(input[value=moderate])', {
    status: 'actioned', public_outcome: outcome, moderator_notes: evidence,
  });
  await expect.poll(() => query(
    'SELECT status, public_outcome, moderator_notes FROM player_reports WHERE id=$1',
    [reportId],
  )).toEqual([{status: 'actioned', public_outcome: outcome, moderator_notes: evidence}]);
});
