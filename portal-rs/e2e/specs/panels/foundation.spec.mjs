import {exportOptions} from '../../helpers.mjs';
import {expect, test} from '../../fixtures.mjs';
import {createHash, randomBytes} from 'node:crypto';
import {csrfRejected, pool} from '../../helpers.mjs';
import {confirmStaffPassword, isolatedAdmin, membership, player, query, role, rolePage, unique} from '../../staff-rbac-helpers.mjs';

const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
const visit = (page, url) => page.goto(url, {waitUntil: 'domcontentloaded'});
const digest = text => createHash('sha256').update(text).digest();

// Seed durable browser state in the disposable database using the same proof
// fields as admin/src/test_staff.rs. Requests still pass the real route guards.
async function panelActor(context, lang, staff = false) {
  const session = randomBytes(32).toString('hex');
  const csrf = randomBytes(32).toString('hex');
  const grant = randomBytes(32).toString('hex');
  const db = pool();
  const connection = await db.connect();
  try {
    await connection.query('BEGIN');
    const {rows: [account]} = await connection.query(
      `INSERT INTO accounts(email,password_hash,status,email_verified_at,locale)
       VALUES ($1,'panel-test-only','active',now(),$2)
       RETURNING id,password_hash`,
      [`${unique()}@example.test`, lang],
    );
    if (staff) {
      const membership = await connection.query(
        "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'",
        [account.id],
      );
      expect(membership.rowCount).toBe(1);
      const {rows: [{generation}]} = await connection.query(
        'INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,$2,true) RETURNING generation',
        [account.id, randomBytes(32)],
      );
      await connection.query(
        `INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at)
         VALUES($1,$2,$3,$4,'grant',clock_timestamp()+interval '15 minutes')`,
        [digest(grant), account.id, generation, digest(account.password_hash)],
      );
    }
    await connection.query(
      `INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,data,authorization_generation)
       SELECT $1,id,now()+interval '15 minutes',now()+interval '1 hour','panel test',$3,$4,native_security_generation
       FROM accounts WHERE id=$2`,
      [digest(session), account.id, csrf, JSON.stringify(staff ? {mfa_grant: grant} : {})],
    );
    await connection.query('COMMIT');
    await context.addCookies([{
      name: '__Host-oteryn_session', value: session, url: origin, secure: true, httpOnly: true,
    }]);
    return account.id;
  } catch (error) {
    await connection.query('ROLLBACK');
    throw error;
  } finally {
    connection.release();
    await db.end();
  }
}

const labels = {
  en: {
    player: 'Player panel', staff: 'Staff dashboard', email: 'Email settings',
    export: 'Export account data', deletion: 'Delete account',
    accounts: 'Account search', bazaar: 'Bazaar moderation', gm: 'GM console',
    worlds: 'Worlds', reason: 'Action reason',
    invalidReason: 'Choose an approved reason.',
  },
  pl: {
    player: 'Panel gracza', staff: 'Panel administracyjny', email: 'Ustawienia adresu e-mail',
    export: 'Eksport danych konta', deletion: 'Usuń konto',
    accounts: 'Wyszukiwanie kont', bazaar: 'Moderacja Bazaru', gm: 'Konsola GM',
    worlds: 'Światy', reason: 'Powód działania',
    invalidReason: 'Wybierz zatwierdzony powód.',
  },
};

test('panel navigation enforces scope and role grants require a reason', async ({page, browser}, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const label = labels[lang];
  const recipient = await panelActor(page.context(), lang);
  expect((await visit(page, '/account')).status()).toBe(200);
  const playerNav = page.getByRole('navigation', {name: label.player, exact: true});
  for (const [name, suffix] of [
    [label.email, 'security/email'],
    [label.export, 'privacy/export'],
    [label.deletion, 'privacy/delete'],
  ]) {
    await expect(playerNav.getByRole('link', {name, exact: true})).toHaveAttribute('href', `/${lang}/account/${suffix}`);
  }
  const exportResponse = await page.request.get(`/${lang}/account/privacy/export`, {maxRedirects: 0});
  expect(exportResponse.status()).toBe(303);
  expect(exportResponse.headers().location).toBe(`/${lang}/account/sudo?return_to=export`);
  await playerNav.getByRole('link', {name: label.email, exact: true}).click();
  await expect(page).toHaveURL(new RegExp(`/${lang}/account/security/email$`));
  await expect(page.locator('[data-staff-dashboard]')).toHaveCount(0);
  expect((await visit(page, '/admin')).status()).toBe(403);
  expect((await visit(page, '/admin/players')).status()).toBe(403);
  expect((await visit(page, '/console')).status()).toBe(403);

  const staffContext = await browser.newContext({
    baseURL: origin, ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport,
  });
  try {
    const staff = await panelActor(staffContext, lang, true);
    const admin = await staffContext.newPage();
    expect((await visit(admin, '/account')).status()).toBe(200);
    const dashboard = admin.locator('#podglad-konta [data-staff-dashboard]');
    await expect(dashboard).toHaveText(label.staff);
    const dashboardUrl = await dashboard.getAttribute('href');
    expect(new URL(dashboardUrl).pathname).toBe('/admin');
    expect((await visit(admin, dashboardUrl)).status()).toBe(200);

    const adminNav = admin.locator('nav[data-panel="admin"]');
    await expect(adminNav.getByRole('link', {name: label.accounts})).toHaveAttribute('href', '/admin/players');
    await expect(adminNav.getByRole('link', {name: label.bazaar})).toHaveAttribute('href', `/admin/${lang}/bazaar`);
    await expect(adminNav.getByRole('link', {name: label.gm})).toHaveCount(0);
    const alternate = lang === 'pl' ? 'en' : 'pl';
    await staffContext.addCookies([{name: 'portal_language', value: alternate, url: origin}]);
    expect((await visit(admin, '/admin/roles')).status()).toBe(200);
    await expect(admin.locator('html')).toHaveAttribute('lang', alternate);
    await expect(adminNav.getByRole('link', {name: labels[alternate].bazaar}))
      .toHaveAttribute('href', `/admin/${alternate}/bazaar`);
    await expect(admin.getByLabel(labels[alternate].reason, {exact: true}).first()).toBeVisible();
    await expect(admin.locator('main a[href$="/admin/roles/confirm"]'))
      .toHaveAttribute('href', `/${alternate}/admin/roles/confirm`);
    for (const locale of ['pl', 'en']) {
      await staffContext.addCookies([{name: 'portal_language', value: locale === 'pl' ? 'en' : 'pl', url: origin}]);
      expect((await visit(admin, `/console/dashboard?lang=${locale}`)).status()).toBe(403);
      await expect(admin.locator('html')).toHaveAttribute('lang', locale);
      await expect(admin.locator('main h1')).toHaveText(locale === 'pl' ? 'Brak dostępu' : 'Access denied');
      await expect(admin.locator('#portal-navigation')).toHaveAttribute(
        'aria-label', locale === 'pl' ? 'Nawigacja główna' : 'Main navigation',
      );
      const news = admin.locator(`#portal-navigation a[href="/${locale}/news"]`);
      await expect(news.first()).toHaveText(locale === 'pl' ? 'Aktualności' : 'News');
    }
    await staffContext.addCookies([{name: 'portal_language', value: lang, url: origin}]);
    expect((await visit(admin, '/console/worlds')).status()).toBe(403);
    await query(
      "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='game_viewer'",
      [staff],
    );

    await visit(admin, dashboardUrl);
    await adminNav.getByRole('link', {name: label.gm}).click();
    const gmNav = admin.getByRole('navigation', {name: label.gm, exact: true});
    await gmNav.getByRole('link', {name: label.worlds, exact: true}).click();
    await expect(admin).toHaveURL(new RegExp(`/console/worlds\\?lang=${lang}$`));
    await expect(admin.locator('html')).toHaveAttribute('lang', lang);

    const preferred = lang === 'pl' ? 'en' : 'pl';
    await staffContext.addCookies([{name: 'portal_language', value: preferred, url: origin}]);
    expect((await visit(admin, '/console/worlds')).status()).toBe(200);
    await expect(admin.locator('html')).toHaveAttribute('lang', preferred);
    await expect(admin.locator('main h1')).toHaveText(labels[preferred].worlds);
    await expect(admin.locator('#portal-navigation')).toHaveAttribute(
      'aria-label', preferred === 'pl' ? 'Nawigacja główna' : 'Main navigation',
    );
    await staffContext.addCookies([{name: 'portal_language', value: lang, url: origin}]);
    const saved = lang === 'pl' ? 'en' : 'pl';
    expect((await visit(admin, `/language/${saved}`)).status()).toBe(200);
    expect((await visit(admin, '/console')).status()).toBe(200);
    await expect(admin).toHaveURL(new RegExp(`/console/dashboard\\?lang=${saved}$`));
    await expect(admin.getByRole('navigation', {name: labels[saved].gm, exact: true})).toBeVisible();
    expect((await visit(admin, `/language/${lang}`)).status()).toBe(200);

    const [{id: roleId}] = await query('INSERT INTO roles(key) VALUES($1) RETURNING id', [unique()]);
    await rolePage(admin, roleId);
    const grant = admin.locator('form:has(button[value=grant])');
    const csrf = await grant.locator('[name=_csrf]').inputValue();
    const missing = await admin.request.post('/admin/roles', {
      form: {_csrf: csrf, action: 'grant', account: recipient, role: roleId},
      headers: {'accept-language': lang},
      maxRedirects: 0,
    });
    expect(missing.status()).toBe(422);
    expect(await missing.text()).toContain(label.invalidReason);
    const membership = () => query('SELECT role_id FROM account_roles WHERE account_id=$1 AND role_id=$2', [recipient, roleId]);
    const audit = () => query("SELECT before, after, staff_reason FROM audit_log WHERE action='rbac.role.granted' AND target=$1", [recipient]);
    expect(await membership()).toEqual([]);
    expect(await audit()).toEqual([]);

    const reason = 'role_assignment';
    await query("UPDATE web_sessions SET sudo_until=clock_timestamp()-interval '1 second' WHERE account_id=$1", [staff]);
    const unconfirmed = await admin.request.post('/admin/roles', {
      form: {_csrf: csrf, action: 'grant', account: recipient, role: roleId, reason},
      maxRedirects: 0,
    });
    expect(unconfirmed.status()).toBe(403);
    expect(await membership()).toEqual([]);
    expect(await audit()).toEqual([]);
    await query("UPDATE web_sessions SET sudo_until=clock_timestamp()+interval '15 minutes' WHERE account_id=$1", [staff]);
    await grant.locator('[name=account]').fill(recipient);
    await grant.locator('[name=role]').selectOption(roleId);
    await grant.getByLabel(label.reason).selectOption(reason);
    const [granted] = await Promise.all([
      admin.waitForResponse(response =>
        new URL(response.url()).pathname === '/admin/roles'
          && response.request().method() === 'POST'),
      grant.locator('button[value=grant]').click(),
    ]);
    expect(granted.status()).toBe(303);
    await expect(admin).toHaveURL(/\/admin\/roles$/);
    expect(await membership()).toEqual([{role_id: roleId}]);
    expect(await audit()).toEqual([{
      before: {role: roleId, assigned: false},
      after: {role: roleId, assigned: true},
      staff_reason: reason,
    }]);
  } finally {
    await staffContext.close();
  }
});

test('staff confirmation and role creation keep their own credentials boundary', async ({page, browser}, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const staff = await isolatedAdmin(browser, info);
  try {
    const owner = await player(page, lang);
    expect((await visit(page, `/${lang}/admin/roles/confirm`)).status()).toBe(403);
    const playerCredentials = process.env.OTERYN_ACCOUNT_CREDENTIALS === 'true';
    expect((await visit(staff.page, `/${lang}/password/change`)).status()).toBe(playerCredentials ? 200 : 404);
    await confirmStaffPassword(staff.page, lang, staff.account.password);

    const ordinary = staff.page.locator('form:has(button[value=create])');
    const privileged = staff.page.locator('form:has(button[value=create-grant-role])');
    await expect(ordinary.locator('[name=reason]')).toHaveCount(0);
    await expect(privileged.getByLabel(labels[lang].reason)).toBeVisible();
    const key = unique();
    const token = await privileged.locator('[name=_csrf]').inputValue();
    const fields = {action: 'create-grant-role', key, permissions: 'admin.roles.grant'};
    await csrfRejected(staff.page, '/admin/roles', {...fields, reason: 'role_assignment'});
    const post = (form) => staff.page.request.post('/admin/roles', {
      form: {...form, _csrf: token}, headers: {origin}, maxRedirects: 0,
    });
    expect((await post(fields)).status()).toBe(422);
    expect(await query('SELECT id FROM roles WHERE key=$1', [key])).toEqual([]);
    expect((await post({...fields, reason: 'role_assignment'})).status()).toBe(303);
    expect(await query(`SELECT staff_reason FROM audit_log JOIN roles ON target=roles.id::text
      WHERE action='rbac.role.created' AND roles.key=$1`, [key]))
      .toEqual([{staff_reason: 'role_assignment'}]);

    const ordinaryRole = await role(staff.page);
    await membership(staff.page, owner.id, ordinaryRole, 'grant');
  } finally {
    await staff.context.close();
  }
});

test('owner panel links follow export and cancellation eligibility', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const owner = await panelActor(page.context(), lang);
  const credentials = process.env.OTERYN_ACCOUNT_CREDENTIALS === 'true';
  const intake = process.env.PORTAL_ACCOUNT_TERMINATION_ENABLED === 'true';
  const exportPath = `/${lang}/account/privacy/export`;
  const deletePath = `/${lang}/account/privacy/delete`;
  const nav = page.getByRole('navigation', {name: labels[lang].player, exact: true});
  const exportLink = nav.getByRole('link', {name: labels[lang].export, exact: true});
  const deleteLink = nav.getByRole('link', {name: labels[lang].deletion, exact: true});
  expect((await visit(page, '/account')).status()).toBe(200);
  await expect(exportLink).toHaveCount(credentials ? 1 : 0);
  await expect(deleteLink).toHaveCount(intake ? 1 : 0);
  expect((await page.request.get(exportPath, {maxRedirects: 0})).status()).toBe(credentials ? 303 : 403);
  expect((await page.request.get(deletePath, {maxRedirects: 0})).status()).toBe(intake ? 200 : 404);

  await query("UPDATE web_sessions SET sudo_until=clock_timestamp()+interval '15 minutes' WHERE account_id=$1", [owner]);
  await query("INSERT INTO account_terminations(account_id,scheduled_for) VALUES($1,now()+interval '7 days')", [owner]);
  expect((await visit(page, '/account')).status()).toBe(200);
  await expect(exportLink).toHaveAttribute('href', exportPath);
  await expect(deleteLink).toHaveAttribute('href', deletePath);
  const exported = await page.request.post(exportPath, await exportOptions(page, lang));
  expect(exported.status()).toBe(200);
  expect((await exported.json()).account.id).toBe(owner);
  await deleteLink.click();
  await expect(page).toHaveURL(new URL(deletePath, origin).href);
  await expect(page.locator(`main form[action="${deletePath}/cancel"]`)).toBeVisible();
});
