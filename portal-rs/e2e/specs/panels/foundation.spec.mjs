import {expect, test} from '../../fixtures.mjs';
import {createHash, randomBytes} from 'node:crypto';
import {pool} from '../../helpers.mjs';
import {assertShell} from '../../shell.mjs';
import {query, rolePage, unique} from '../../staff-rbac-helpers.mjs';

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
    for (const section of ['dashboard', 'worlds', 'nodes', 'database']) {
      expect((await visit(admin, `/console/${section}?lang=${lang}`)).status(), section).toBe(403);
      if (section === 'dashboard') {
        expect(new URL(admin.url()).pathname).toBe('/console/dashboard');
        await expect(admin.locator('html')).toHaveAttribute('lang',lang);
        await assertShell(admin,{status:403});
      }
    }
    await query(
      "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='game_viewer'",
      [staff],
    );

    for (const section of ['dashboard', 'worlds', 'nodes', 'database']) {
      expect((await visit(admin, `/console/${section}?lang=${lang}`)).status(), section).toBe(200);
    }
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
