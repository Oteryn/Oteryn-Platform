import {test, expect} from '../fixtures.mjs';
import {login, logout, pool, register, submit, waitForLoginBudget} from '../helpers.mjs';
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

function staffUrl(path) {
  return new URL(path, `https://${process.env.OTERYN_ADMIN_HOST}`).href;
}

async function enrollAndReadCodes(page, lang, account) {
  await enrollMfa(page, lang, account);
  const codes = page.locator('main textarea[readonly]');
  await expect(codes.nth(1)).toBeVisible();
  return [await codes.nth(0).inputValue(), await codes.nth(1).inputValue()];
}

async function finishStaffLogin(page, account, code) {
  await expect(page).toHaveURL(staffUrl('/login'));
  await waitForLoginBudget();
  await submit(page, 'main form', {email: account.email, password: account.password});
  await expect(page).toHaveURL(url =>
    url.origin === new URL(staffUrl('/')).origin && url.pathname.endsWith('/mfa/challenge')
  );
  await submit(page, 'form[action$="/mfa/challenge"]', {code});
  expect((await page.goto(staffUrl('/admin'))).status()).toBe(200);
}

async function signInWithMfa(page, lang, account) {
  const [code] = await enrollAndReadCodes(page, lang, account);
  await page.goto(staffUrl('/admin'));
  await finishStaffLogin(page, account, code);
}

async function openAccountMenu(page) {
  const mobileMenu = page.locator('#portal-menu-toggle');
  if (await mobileMenu.isVisible()) await mobileMenu.click();
  await page.locator('[data-account-link]:visible').click();
  await expect(accountMenu(page)).toBeVisible();
}

test('account menu shows a localized staff link only with admin.access', async ({page}, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await login(page, lang, account);
  const link = accountMenu(page).locator('a[href$="/admin"]');
  await expect(accountMenu(page)).toBeAttached();
  await expect(link).toHaveCount(0);
  const consents = `/${lang}/account/consents`;
  expect((await page.goto(consents)).status()).toBe(200);
  await expect(page.locator('main form[action$="/account/consents"]')).toHaveCount(2);
  await expect(accountMenu(page)).toBeAttached();
  await expect(link).toHaveCount(0);

  await grantRole(account, 'game_viewer');
  expect((await page.goto(`/${lang}/`)).status()).toBe(200);
  await expect(link).toHaveText(lang === 'pl' ? 'Panel zespołu' : 'Staff panel');
  const destination = staffUrl('/admin');
  await expect(link).toHaveAttribute('href', destination);
  expect((await page.goto(consents)).status()).toBe(200);
  await expect(page.locator('main form[action$="/account/consents"]')).toHaveCount(2);
  await expect(link).toHaveAttribute('href', destination);

  const publicOrigin = new URL(page.url()).origin;
  const [publicCode, staffCode] = await enrollAndReadCodes(page, lang, account);
  await loginWithRecoveryCode(page, lang, account, publicCode);
  expect((await page.goto(`/${lang}/`)).status()).toBe(200);
  await openAccountMenu(page);
  const landing = page.waitForResponse(response => response.url() === destination);
  await link.click();
  if (process.env.PORTAL_E2E_SPLIT_STAFF_ORIGIN === '1') {
    expect(new URL(destination).origin).not.toBe(publicOrigin);
  }
  if (new URL(destination).origin !== publicOrigin) {
    expect((await landing).status()).toBe(303);
    await finishStaffLogin(page, account, staffCode);
  } else {
    expect((await landing).status()).toBe(200);
    await expect(page).toHaveURL(destination);
  }

  expect((await page.goto(staffUrl(`/${lang}/`))).status()).toBe(200);
  await openAccountMenu(page);
  await submit(page, 'dialog form[action$="/logout"]');
  await expect(accountMenu(page)).toHaveCount(0);
  const signedOut = await page.request.get(destination, {maxRedirects: 0});
  expect(signedOut.status()).toBe(303);
  expect(signedOut.headers().location).toBe('/login');
});

test('cold atlas sends no-store for document and direct GET requests', async ({request}, info) => {
  const lang = info.project.metadata.locale;
  for (const accept of ['text/html', '*/*']) {
    const atlas = await request.get(`/${lang}/atlas`, {headers: {accept}});
    expect(atlas.status()).toBe(503);
    expect(atlas.headers()['cache-control']).toBe('no-store');
  }
});

test('browser preserves private cache directives on a non-HTML analytics 503', async ({page}, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const staff = await register(page, lang);
  await grantRole(staff, 'platform_admin');
  await login(page, lang, staff);
  await signInWithMfa(page, lang, staff);
  expect((await page.goto(staffUrl('/admin/analytics'))).status()).toBe(200);
  const endpoint = '/admin/analytics/data';

  const db = pool();
  const client = await db.connect();
  try {
    await client.query('BEGIN');
    // Hold the real ingestion fence so the analytics data route refuses the read immediately.
    await client.query('SELECT pg_advisory_xact_lock(71140001)');
    const response = await page.evaluate(async path => {
      const result = await fetch(path, {headers: {accept: 'application/json'}, redirect: 'manual'});
      return {
        status: result.status,
        cacheControl: result.headers.get('cache-control'),
        pragma: result.headers.get('pragma'),
        contentType: result.headers.get('content-type') ?? '',
      };
    }, endpoint);
    expect(response.status).toBe(503);
    expect(response.contentType).not.toContain('text/html');
    expect(response.cacheControl).toBe('no-store, private');
    expect(response.pragma).toBe('no-cache');
    const document = await page.goto(staffUrl(endpoint));
    expect(document.status()).toBe(503);
    expect(document.headers()['content-type']).toContain('text/html');
    expect(document.headers()['cache-control']).toBe('no-store, private');
    expect(document.headers().pragma).toBe('no-cache');
  } finally {
    try {
      await client.query('ROLLBACK');
    } finally {
      client.release();
      await db.end();
    }
  }
});

test('moderator can review reports and is refused Platform admin areas', async ({page}, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const reporter = await register(page, lang);
  await login(page, lang, reporter);
  const [{id: reporterId}] = await query('SELECT id FROM accounts WHERE email=$1', [reporter.email]);
  expect((await page.goto(`/${lang}/support/reports/create`)).status()).toBe(200);
  const evidence = unique();
  await submit(page, 'main form', {target_reference: reporterId, evidence_summary: evidence});
  await expect(page).toHaveURL(/support\/reports\/[a-f0-9-]+$/);
  const reportId = new URL(page.url()).pathname.split('/').at(-1);
  await logout(page, lang);

  const moderator = await register(page, lang);
  const [{id: moderatorId}] = await query('SELECT id FROM accounts WHERE email=$1', [moderator.email]);
  expect(moderatorId).not.toBe(reporterId);
  await grantRole(moderator, 'moderator');
  await login(page, lang, moderator);
  await signInWithMfa(page, lang, moderator);
  expect((await page.request.get(staffUrl('/admin'))).status()).toBe(200);

  for (const path of ['/admin/roles', '/admin/console/access', '/admin/news', '/admin/analytics']) {
    expect((await page.request.get(staffUrl(path))).status(), path).toBe(403);
  }
  expect((await page.request.get(staffUrl(`/admin/${lang}/party`), {maxRedirects: 0})).status()).toBe(403);
  const queue = '/admin/moderation/reports';
  expect((await page.request.get(staffUrl(queue), {maxRedirects: 0})).status()).toBe(200);
  expect((await page.goto(staffUrl(`${queue}/${reportId}`))).status()).toBe(200);
  await expect(page.locator('main')).toContainText(evidence);

  const outcome = unique();
  await action(page, 'form:has(input[value=moderate])', {
    status: 'actioned', public_outcome: outcome, moderator_notes: evidence,
  });
  await expect.poll(() => query(
    'SELECT status, public_outcome, moderator_notes, assigned_to, reporter_account_id FROM player_reports WHERE id=$1',
    [reportId],
  )).toEqual([{
    status: 'actioned', public_outcome: outcome, moderator_notes: evidence,
    assigned_to: moderatorId, reporter_account_id: reporterId,
  }]);
});
