import * as OTPAuth from 'otpauth';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login, submit } from '../helpers.mjs';
import { loginWithRecoveryCode, query, rolePage } from '../staff-rbac-helpers.mjs';
import { withSoleRoleHolder } from '../staff-fixture-database.mjs';
import { accountRoles, adminContext, builtInPermissions, expectAuditEntry, permissionCatalog, postAdmin, roleAction, roleAudit, seedAdminAccount, seedAdminMfa } from '../admin-helpers.mjs';

const platformRoles = ['platform_admin', 'security_admin', 'content_editor', 'moderator', 'support', 'operations'];
const gameRoles = [
  'game_admin', 'game_operator', 'game_dba', 'game_master_senior', 'game_release',
  'game_security', 'game_support', 'game_analyst', 'game_viewer',
];
const roleManagers = ['platform_admin', 'security_admin'];
const consolePages = ['dashboard', 'worlds', 'nodes', 'database'];
const consolePermissions = {
  dashboard: 'game.ops.read', worlds: 'game.world.read',
  nodes: 'game.node.read', database: 'game.db.read',
};
const readConsole = ['players', 'audit', 'access', 'approvals'];
const analyticsPages = ['', '/creatures', '/activity', '/levels', '/health', '/economy'];
const mfaHeading = { pl: 'Wymagane uwierzytelnianie wieloskładnikowe', en: 'Multi-factor authentication required' };
const panelHeading = { pl: 'Panel zespołu', en: 'Staff panel' };

async function enrollAndSignIn(page, lang, account) {
  await page.goto(`/${lang}/mfa`);
  await submit(page, 'form[action$="/mfa/enroll"]', { password: account.password });
  const totp = new OTPAuth.TOTP({
    secret: await page.locator('main .op-prose').innerText(), algorithm: 'SHA1', digits: 6, period: 30,
  });
  const enrollmentCounter = totp.counter();
  await submit(page, 'form[action$="/mfa/confirm"]', { password: account.password, code: totp.generate() });
  const recovery = page.locator('textarea[readonly]').first();
  await expect(recovery).toBeVisible();
  const code = await recovery.inputValue();
  await loginWithRecoveryCode(page, lang, account, code);
  await expect(page).not.toHaveURL(/mfa\/challenge/);
  return { totp, enrollmentCounter };
}

let manager;
let roles;
const matrixActors = [];

test.beforeAll(async ({ browser }, info) => {
  test.setTimeout(120_000);
  const lang = info.project.metadata.locale;
  const context = await adminContext(browser, info);
  const page = await context.newPage();
  const account = seedAdminAccount(lang);
  roles = Object.fromEntries((await query('SELECT id,key FROM roles WHERE built_in')).map(row => [row.key, row.id]));
  expect(Object.keys(roles).sort()).toEqual([...platformRoles, ...gameRoles].sort());
  manager = { context, page, ...account };
  // First-staff bootstrap, as in the shared staff fixture.
  await query('INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)', [account.id, roles.platform_admin]);
  await login(page, lang, account);
  expect((await page.goto('/admin')).status()).toBe(403);
  await expect(page.getByRole('heading', { name: mfaHeading[lang], exact: true })).toBeVisible();
  const { totp, enrollmentCounter } = await enrollAndSignIn(page, lang, account);
  Object.assign(manager, { totp, enrollmentCounter });
});

test.afterAll(async () => {
  if (!manager) return;
  await manager.context.close();
  await query('DELETE FROM account_roles WHERE account_id=ANY($1::uuid[])', [
    [manager.id, ...matrixActors.filter(Boolean).map(account => account.id)],
  ]);
});

test('roles page labels, validation, self-grant and last administrator protection', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const staff = manager.page;
  const role = roles.platform_admin;
  const response = await staff.goto('/admin');
  expect(response.status()).toBe(200);
  await expect(staff.getByRole('heading', { name: panelHeading[lang], exact: true })).toBeVisible();
  await staff.getByRole('link', { name: lang === 'pl' ? 'Role' : 'Roles', exact: true }).click();
  await expect(staff.locator('html')).toHaveAttribute('lang', lang);
  await expect(staff.getByText(lang === 'pl' ? 'Rola' : 'Role', { exact: true })).toHaveCount(2);
  const labels = lang === 'pl' ? [
    'UUID konta członka zespołu', 'Przypisz rolę', 'Odbierz rolę', 'Dostępne uprawnienia',
    'Zmiany ról uprzywilejowanych wymagają odrębnego zatwierdzenia.', 'Klucz nowej roli',
    'Uprawnienia (oddzielone spacjami)', 'Utwórz rolę', 'Nowy zestaw uprawnień (oddzielone spacjami)',
    'Zapisz uprawnienia',
  ] : [
    'Staff account UUID', 'Assign role', 'Revoke role', 'Available permissions',
    'Privileged role changes require separate approval.', 'New role key',
    'Permissions (separated by spaces)', 'Create role', 'Replacement permissions (separated by spaces)',
    'Save permissions',
  ];
  for (const label of labels) await expect(staff.getByText(label, { exact: true })).toBeVisible();
  await staff.locator('main details summary').click();
  expect((await staff.locator('main details code').allTextContents()).sort()).toEqual(permissionCatalog);
  expect(await accountRoles(manager.id)).toEqual(['platform_admin']);
  await withSoleRoleHolder(role, manager.id, async () => {
    const [{ count }] = await query('SELECT count(*)::int AS count FROM account_roles WHERE role_id=$1', [role]);
    expect(count).toBe(1);
    await roleAction(staff, 'revoke', { account: manager.id, role }, 403);
    expect(await accountRoles(manager.id)).toEqual(['platform_admin']);
  });
  await roleAudit(staff, lang, {
    action: 'admin.access.denied', actor: manager.id, target: '/admin/roles',
    before: null, after: { allowed: false },
  });

  await rolePage(staff, roles.support);
  await roleAction(staff, 'grant', { account: manager.id, role: roles.support }, 403);
  expect(await accountRoles(manager.id)).toEqual(['platform_admin']);
  await roleAudit(staff, lang, {
    action: 'admin.access.denied', actor: manager.id, target: '/admin/roles',
    before: null, after: { allowed: false },
  });

  await staff.goto('/admin/roles');
  await roleAction(staff, 'create', { key: 'invalid key', permissions: 'unknown.permission' }, 400);
  await staff.goto('/admin/roles');
  const nil = '00000000-0000-0000-0000-000000000000';
  await roleAction(staff, 'grant', { account: nil, role }, 400);
  await staff.goto('/admin/roles');
  await roleAction(staff, 'revoke', { account: nil, role }, 400);
  await staff.goto('/admin/roles');
  await roleAction(staff, 'set-permissions', { role: roles.support, permissions: 'unknown.permission' }, 400);
  const guest = await page.request.get('/admin/roles', { maxRedirects: 0 });
  expect(guest.status()).toBe(303);
  expect(guest.headers().location).toBe('/login');
  await page.goto(`/${lang}/login`);
  const token = await page.locator('main input[name=_csrf]').inputValue();
  for (const action of ['grant', 'revoke', 'create', 'set-permissions']) {
    const result = await postAdmin(page, '/admin/roles', { _csrf: token, action, account: manager.id, role });
    expect(result.status(), `anonymous ${action}`).toBe(303);
  }
});

for (const [index, key] of [...platformRoles, ...gameRoles].entries()) {
  test(`only ${key} · grant, MFA, permission matrix and revoke`, async ({ page }, info) => {
    const lang = info.project.metadata.locale;
    const staff = manager.page;
    // Each actor has a distinct unused recovery code for each login.
    const actorSlot = Math.floor(index / 8);
    if (!matrixActors[actorSlot]) {
      const seeded = seedAdminAccount(lang);
      matrixActors[actorSlot] = { ...seeded, codes: seedAdminMfa(seeded) };
    }
    const account = matrixActors[actorSlot];
    const role = roles[key];
    const protectedGrant = roleManagers.includes(key);
    expect(await accountRoles(account.id)).toEqual([]);

    await rolePage(staff, role);
    await roleAction(staff, 'grant', { account: account.id, role }, protectedGrant ? 403 : 303);
    if (protectedGrant) {
      expect(await accountRoles(account.id)).toEqual([]);
      await expectAuditEntry({
        action: 'admin.access.denied', actor: manager.id, target: '/admin/roles',
        before: null, after: { allowed: false },
      });
      // Protected grants have no direct success path in the browser panel.
      await query('INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)', [account.id, role]);
    } else {
      await expectAuditEntry({
        action: 'rbac.role.granted', actor: manager.id, target: account.id,
        before: { role, assigned: false }, after: { role, assigned: true },
      });
    }
    expect(await accountRoles(account.id)).toEqual([key]);

    await login(page, lang, account);
    await expect(page).toHaveURL(/mfa\/challenge/);
    const pending = await page.request.get('/admin', { maxRedirects: 0 });
    expect(pending.status()).toBe(303);
    expect(pending.headers().location).toBe('/login');
    const code = account.codes[index % 8];
    expect(Boolean(code), 'seeded recovery code for this login').toBe(true);
    await submit(page, 'form[action$="/mfa/challenge"]', { code });
    await expect(page).not.toHaveURL(/mfa\/challenge/);

    const permissions = builtInPermissions[key].split(/\s+/);
    const has = permission => permissions.includes(permission);
    const managesRoles = roleManagers.includes(key);
    expect((await page.goto('/admin')).status()).toBe(200);
    await expect(page.getByRole('heading', { name: panelHeading[lang], exact: true })).toBeVisible();
    await expect(page.locator('main nav a')).toHaveCount(managesRoles ? 1 : 0);
    await expect(page.locator('main form button')).toHaveCount(managesRoles ? 4 : 0);
    const canOpenReadConsole = {
      players: has('game.player.read') || has('support.tickets.manage'),
      audit: has('audit.view') || has('game.audit.read'),
      access: managesRoles,
      approvals: managesRoles,
    };
    const routes = [
      ['/admin/roles', managesRoles],
      ['/admin/control/step-up', true],
      ['/admin/players', ['platform_admin', 'support'].includes(key)],
      ['/admin/support/tickets', has('support.tickets.manage')],
      ['/admin/moderation/reports', has('support.reports.manage')],
      ['/admin/moderation/enforcement', has('support.enforcement.manage')],
      ['/admin/moderation/enforcement/create', has('support.enforcement.manage')],
      [`/admin/${lang}/bazaar`, ['platform_admin', 'moderator'].includes(key)],
      ['/admin/commerce', ['platform_admin', 'moderator'].includes(key)],
      ...consolePages.map(name => [`/console/${name}`, true]),
      ...readConsole.map(name => [`/admin/console/${name}?lang=${lang}`, canOpenReadConsole[name]]),
      ...analyticsPages.flatMap(suffix => ['', '/data'].map(data =>
        [`/admin/analytics${suffix}${data}`, key === 'platform_admin' || has('game.analytics.read')])),
    ];
    for (const [path, allowed] of routes) {
      const screen = allowed && ['/admin/commerce', '/console/dashboard'].includes(path);
      const response = screen ? await page.goto(path) : await page.request.get(path);
      expect(response.status(), `${key} GET ${path}`).toBe(allowed ? 200 : 403);
      if (allowed && path === '/admin/commerce') {
        const actions = await page.locator('main form[action^="/admin/commerce/"]').evaluateAll(
          forms => forms.map(form => form.getAttribute('action')).sort(),
        );
        const expected = key === 'platform_admin' ? ['adjust', 'grant', 'reconcile', 'refund', 'revoke']
          : key === 'moderator' ? ['adjust'] : [];
        expect(actions).toEqual(expected.map(action => `/admin/commerce/${action}`));
      }
      if (allowed && path.startsWith('/console/')) {
        if (screen) {
          await expect(page.locator('header nav.op-tabs a')).toHaveCount(
            Object.values(consolePermissions).filter(has).length,
          );
          await expect(page.locator('main form[method=post]')).toHaveCount(0);
        }
        expect((await page.request.get(`${path}?agent=disconnected`)).status())
          .toBe(has(consolePermissions[path.split('/').at(-1)]) ? 200 : 403);
      }
    }

    if (managesRoles) {
      await page.goto('/admin/roles');
      await expect(page.locator('main form button')).toHaveCount(4);
    } else {
      await page.goto(`/${lang}/`);
    }
    const token = await page.locator('input[name=_csrf]').first().inputValue();
    if (!managesRoles) {
      for (const action of ['grant', 'revoke', 'create', 'set-permissions']) {
        const response = await postAdmin(page, '/admin/roles', { action, _csrf: token, account: account.id, role });
        expect(response.status(), `${key} POST ${action}`).toBe(403);
      }
    }
    for (const [section, operations] of Object.entries(consoleOperations)) {
      for (const operation of operations) {
        const target = ['world.get', 'node.get'].includes(operation) ? 'example' : '';
        const response = await postAdmin(page, `/console/${section}`, {
          operation, target, agent: 'disconnected', lang, _csrf: token,
        });
        expect(response.status(), `${key} POST ${operation}`).toBe(has(consolePermissions[section]) ? 503 : 403);
      }
    }
    const search = await postAdmin(page, '/admin/console/players', { q: 'Example', lang, _csrf: token });
    expect(search.status(), `${key} POST player search`).toBe(has('game.player.read') ? 303 : 403);

    await rolePage(staff, role);
    await roleAction(staff, 'revoke', { account: account.id, role }, 303);
    expect(await accountRoles(account.id)).toEqual([]);
    const denied = await page.request.get('/admin', { maxRedirects: 0 });
    expect(denied.status()).toBe(303);
    expect(denied.headers().location).toBe('/login');
    await expectAuditEntry({
      action: 'rbac.role.revoked', actor: manager.id, target: account.id,
      before: { role, assigned: true }, after: { role, assigned: false },
    });
  });
}

test('custom roles create and replace permissions with an audit trail', async ({}, info) => {
  const lang = info.project.metadata.locale;
  const staff = manager.page;
  const key = `e2e_${manager.id.replaceAll('-', '')}`;
  await staff.goto('/admin/roles');
  await roleAction(staff, 'create', { key, permissions: 'admin.access audit.view' }, 303);
  const [{ id }] = await query('SELECT id FROM roles WHERE key=$1', [key]);
  await rolePage(staff, id);
  await expect(staff.locator('main section > ul > li').filter({ hasText: new RegExp(`^${key}$`) })).toBeVisible();
  await roleAudit(staff, lang, {
    action: 'rbac.role.created', actor: manager.id, target: id,
    before: null, after: { key, permissions: ['admin.access', 'audit.view'] },
  });

  await rolePage(staff, id);
  await roleAction(staff, 'set-permissions', { role: id, permissions: 'admin.access support.tickets.manage' }, 303);
  await roleAudit(staff, lang, {
    action: 'rbac.role.permissions_changed', actor: manager.id, target: id,
    before: { permissions: ['admin.access', 'audit.view'] },
    after: { permissions: ['admin.access', 'support.tickets.manage'] },
  });

  await rolePage(staff, id);
  await roleAction(staff, 'set-permissions', { role: id, permissions: 'admin.roles.manage' }, 403);
  expect((await query('SELECT permission FROM role_permissions WHERE role_id=$1 ORDER BY permission', [id]))
    .map(row => row.permission)).toEqual(['admin.access', 'support.tickets.manage']);
});

test('read console navigation and filters preserve unavailable Game states', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const staff = manager.page;
  const polish = lang === 'pl';
  const names = polish ? ['Gracze', 'Audyt', 'Dostęp', 'Zatwierdzenia'] : ['Players', 'Audit', 'Access', 'Approvals'];
  const unavailable = polish ? 'Niedostępne' : 'Unavailable';
  const sectionTitles = polish ? {
    players: ['Postacie i dossier', 'Ekwipunek', 'Kary'],
    audit: ['Platforma · audyt, ostatnie 100 dopasowań', 'Agent gry · czas obserwacji'],
    access: ['Role i uprawnienia platformy', 'Identyfikatory kluczy podpisujących i okresy ważności (UTC)',
      'Połączeni agenci, czasy obserwacji i ogłaszane operacje', 'Polityka gry'],
    approvals: ['Oczekujące żądania R4 i podglądy próbne', 'Zatwierdzanie'],
  } : {
    players: ['Characters and dossier', 'Inventory', 'Sanctions'],
    audit: ['Platform · audit log, latest 100 matches', 'Game agent · observed at'],
    access: ['Platform roles and permissions', 'Signer key IDs and validity windows (UTC)',
      'Connected agents, observation times and advertised operations', 'Game policy'],
    approvals: ['Pending R4 requests and dry-run previews', 'Approval controls'],
  };
  await staff.goto(`/admin/console/audit?lang=${lang}`);
  for (const [index, area] of readConsole.entries()) {
    await staff.getByRole('navigation').getByRole('link', { name: names[index], exact: true }).click();
    await expect(staff).toHaveURL(new RegExp(`/admin/console/${area}\\?lang=${lang}$`));
    await expect(staff.getByRole('heading', { level: 1, name: names[index], exact: true })).toBeVisible();
    await expect(staff.locator('main').getByRole('heading', { level: 2 })).toHaveText(sectionTitles[area]);
    if (area === 'audit') {
      const auditForm = staff.locator('main form[method=get]');
      const filters = polish ? ['Identyfikator żądania (UUID)', 'Identyfikator administratora (UUID)', 'Operacja', 'Cel']
        : ['Request ID (UUID)', 'Admin ID (UUID)', 'Operation', 'Target'];
      await expect(auditForm.locator('label')).toHaveText(filters);
      await expect(auditForm.getByRole('button')).toHaveText(polish ? 'Wyświetl' : 'View');
    }
    if (area !== 'approvals') await expect(staff.getByText(unavailable, { exact: true }).first()).toBeVisible();
    if (area === 'access') {
      for (const role of [...platformRoles, ...gameRoles]) {
        const entries = await staff.locator('main li p').filter({ hasText: new RegExp(`^${role}: `) }).allTextContents();
        expect(entries.sort()).toEqual(builtInPermissions[role].split(/\s+/).sort().map(permission => `${role}: ${permission}`));
      }
    }
    expect((await page.request.get(`/admin/console/${area}`, { maxRedirects: 0 })).status()).toBe(303);
  }
  await expect(staff.getByText(polish
    ? 'Tylko odczyt. Zatwierdzanie i odrzucanie są niedostępne do czasu integracji usługi zatwierdzeń.'
    : 'Read-only. Approve and reject are unavailable until the approval service is integrated.', { exact: true })).toBeVisible();
  expect((await staff.request.get('/admin/console/audit?request_id=invalid')).status()).toBe(400);
  for (const filter of ['q=Example', 'character=Example']) {
    const response = await staff.request.get(`/admin/console/players?${filter}`, {
      headers: { 'sec-fetch-site': 'same-origin' },
    });
    expect(response.status()).toBe(403);
  }
  await staff.goto('/admin/roles');
  const token = await staff.locator('main input[name=_csrf]').first().inputValue();
  const search = '/admin/console/players';
  await csrfRejected(staff, search, { q: 'Example', lang });
  expect((await postAdmin(staff, search, { _csrf: token, q: 'Example', lang })).status()).toBe(403);
  expect((await postAdmin(staff, search, { _csrf: token, q: 'x', lang })).status()).toBe(400);
  await page.goto(`/${lang}/login`);
  const guestToken = await page.locator('main input[name=_csrf]').inputValue();
  expect((await postAdmin(page, search, { _csrf: guestToken, q: 'Example', lang })).status()).toBe(303);
});

const consoleOperations = {
  dashboard: ['ops.overview.get'],
  worlds: ['world.list', 'world.get'],
  nodes: ['node.list', 'node.get'],
  database: ['db.status', 'db.migrations.list'],
};
test('Game console selection, refresh and query refusal on all four pages', async ({ page, browser }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const polish = lang === 'pl';
  const staff = manager.page;
  const guestContext = await adminContext(browser, info);
  try {
    const guest = await guestContext.newPage();
    await guest.goto(`/${lang}/login`);
    const csrf = await guest.locator('main input[name=_csrf]').inputValue();
    for (const [section, operations] of Object.entries(consoleOperations)) {
      const path = `/console/${section}`;
      expect((await guest.request.get(path, { maxRedirects: 0 })).status()).toBe(303);
      for (const operation of operations) {
        const target = ['world.get', 'node.get'].includes(operation) ? 'example' : '';
        const result = await postAdmin(guest, path, { operation, target, agent: 'disconnected', lang, _csrf: csrf });
        expect(result.status(), `anonymous ${operation}`).toBe(303);
      }
    }
  } finally {
    await guestContext.close();
  }
  const titles = polish ? ['Pulpit', 'Światy', 'Węzły i hosty', 'Baza danych'] : ['Dashboard', 'Worlds', 'Nodes and hosts', 'Database'];
  const key = `console_${manager.id.replaceAll('-', '')}`;
  await staff.goto('/admin/roles');
  await roleAction(staff, 'create', {
    key, permissions: 'admin.access game.ops.read game.world.read game.node.read game.db.read game.player.read support.tickets.manage',
  }, 303);
  const [{ id }] = await query('SELECT id FROM roles WHERE key=$1', [key]);
  const actor = seedAdminAccount(lang);
  await rolePage(staff, id);
  await roleAction(staff, 'grant', { account: actor.id, role: id }, 303);
  await login(page, lang, actor);
  await enrollAndSignIn(page, lang, actor);
  await page.goto(`/${lang}/`);
  const token = await page.locator('input[name=_csrf]').first().inputValue();
  await page.goto(`/console/dashboard?lang=${lang}`);

  for (const [index, [section, operations]] of Object.entries(consoleOperations).entries()) {
    const path = `/console/${section}`;
    const nav = page.locator('header nav.op-tabs');
    await expect(nav.getByRole('link')).toHaveText(titles);
    await nav.getByRole('link', { name: titles[index], exact: true }).click();
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(titles[index]);
    const skip = page.locator('a[href="#content"]');
    await expect(skip).toHaveText(polish ? 'Przejdź do treści' : 'Skip to content');
    await skip.click();
    await expect(page).toHaveURL(/#content$/);
    const form = page.locator('main form[method=get]');
    await expect(form.locator('label')).toHaveText(polish
      ? ['Identyfikator agenta', 'Identyfikator świata lub węzła (opcjonalny, tylko szczegóły)']
      : ['Agent identifier', 'World or node identifier (optional, details only)']);
    await expect(form.locator('button')).toHaveText(polish ? 'Pokaż zapisane dane' : 'View stored data');
    const agentField = form.locator('[name=agent]');
    await agentField.fill('bad agent');
    expect(await agentField.evaluate(field => field.validity.patternMismatch)).toBe(true);
    await agentField.fill('disconnected-agent');
    expect(await agentField.evaluate(field => field.validity.valid)).toBe(true);
    await agentField.fill('disconnected');
    const details = section === 'worlds' || section === 'nodes';
    if (details) await form.locator('[name=target]').fill('example');
    const [response] = await Promise.all([page.waitForNavigation(), form.locator('button').click()]);
    expect(response.status()).toBe(200);
    await expect(page.locator('main .op-badge').first()).toHaveText(polish ? 'R0 · Niedostępne' : 'R0 · Unavailable');
    await expect(page.locator('main section').first().locator('dt')).toHaveText(polish
      ? ['Źródło danych', 'Świeżość'] : ['Data source', 'Freshness']);
    await expect(page.locator('main section').first().locator('dd')).toHaveText(polish
      ? ['Agent gry', 'Nieznana'] : ['Game agent', 'Unknown']);
    const detailTitle = section === 'worlds' ? (polish ? 'Szczegóły świata' : 'World details')
      : section === 'nodes' ? (polish ? 'Szczegóły węzła' : 'Node details') : titles[index];
    await expect(page.locator('main section').first().getByRole('heading', { level: 2 })).toHaveText(detailTitle);
    await expect(page.locator('main form[method=post]')).toHaveCount(0);
    const poll = page.locator('main > a[href*="agent=disconnected"]');
    await expect(poll).toHaveText(polish ? 'Sprawdź zapisane wyniki' : 'Check stored results');
    await poll.click();
    await expect(form.locator('[name=agent]')).toHaveValue('disconnected');
    if (details) await expect(form.locator('[name=target]')).toHaveValue('example');
    await page.locator(`header a[hreflang="${polish ? 'en' : 'pl'}"]`).click();
    await expect(page.locator('html')).toHaveAttribute('lang', polish ? 'en' : 'pl');
    await page.locator(`header a[hreflang="${lang}"]`).click();

    // A disconnected agent hides the POST form, but the endpoint must still refuse it.
    for (const operation of operations) {
      const target = operation === 'world.get' || operation === 'node.get' ? 'example' : '';
      const fields = { operation, agent: 'disconnected', target, lang };
      await csrfRejected(page, path, fields);
      const unavailable = await postAdmin(page, path, { ...fields, _csrf: token });
      expect(unavailable.status(), operation).toBe(503);
      expect(unavailable.headers()['cache-control']).toBe('no-store');
      expect((await postAdmin(page, path, { ...fields, _csrf: token, agent: 'bad agent' })).status()).toBe(400);
    }
  }
  const players = '/admin/console/players';
  await page.goto(`${players}?lang=${lang}`);
  const dossier = page.locator('main form[method=get]');
  await dossier.getByLabel(polish ? 'Identyfikator postaci' : 'Character ID').fill('example');
  const [result] = await Promise.all([page.waitForNavigation(), dossier.getByRole('button').click()]);
  expect(result.status()).toBe(200);
  await expect(page.locator('main').getByText(polish ? 'Niedostępne' : 'Unavailable', { exact: true })).toHaveCount(3);
  await expect(page.locator('main form[method=post]')).toHaveCount(0);
  expect((await page.goto(`${players}?character=bad%00id`)).status()).toBe(400);

  await rolePage(staff, id);
  await roleAction(staff, 'revoke', { account: actor.id, role: id }, 303);
});

test('analytics navigation, range filters and data endpoints fail closed', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const polish = lang === 'pl';
  const staff = manager.page;
  const labels = polish ? [
    'Przegląd', 'Stworzenia / bossowie', 'Aktywność godzinowa', 'Rozkład poziomów', 'Stan źródła', 'Gospodarka',
  ] : [
    'Overview', 'Creatures / bosses', 'Hourly activity', 'Level distribution', 'Feed health', 'Economy',
  ];
  const world = '019b0760-0000-7000-8000-000000000001';
  await staff.goto('/admin/analytics');
  for (const [index, suffix] of analyticsPages.entries()) {
    const path = `/admin/analytics${suffix}`;
    await staff.locator('main nav').getByRole('link', { name: labels[index], exact: true }).click();
    await expect(staff).toHaveURL(new RegExp(`${path}$`));
    await expect(staff.locator('html')).toHaveAttribute('lang', lang);
    const form = staff.locator('main form[method=get]');
    await expect(form.getByRole('button')).toHaveText(polish ? 'Pokaż' : 'Show');
    await expect(form.getByLabel(polish ? 'Świat' : 'World', { exact: true })).toBeVisible();
    if (suffix === '/economy') {
      await expect(form.getByLabel(polish ? 'UTC od (włącznie)' : 'UTC from (inclusive)')).toBeVisible();
      await expect(form.getByLabel(polish ? 'UTC do (wyłącznie)' : 'UTC to (exclusive)')).toBeVisible();
      await expect(staff.locator('main > p').first()).toContainText(polish
        ? 'Świat × dzień UTC. Opóźnienie publikacji: 24 godziny. Retencja: 400 dni. Polityka prywatności:'
        : 'World × UTC day. Release delay: 24 hours. Retention: 400 days. Privacy policy:');
      await expect(staff.locator('main > p').nth(1)).toContainText(polish
        ? 'Stany i przepływy są rozdzielone.'
        : 'Stocks and flows are separate.');
    }
    await form.locator('[name=world]').fill(world);
    const [filtered] = await Promise.all([staff.waitForNavigation(), form.getByRole('button').click()]);
    expect(filtered.status()).toBe(200);
    expect(filtered.headers()['cache-control']).toBe('no-store, private');
    const cold = staff.locator('[data-state=unavailable]');
    await expect(cold).toHaveCount(suffix === '/economy' ? 5 : 1);
    if (suffix === '/economy') {
      const text = polish
        ? 'Niedostępne: wymagana zaakceptowana baza synchronizacji i aktualne dane gry'
        : 'unavailable: accepted resync baseline and fresh Game evidence required';
      await expect(cold).toHaveText(Array(5).fill(text));
      await expect(staff.getByText(polish ? 'Stan danych / aktualność: —' : 'Data cut / freshness: —', { exact: true })).toHaveCount(5);
      await staff.getByRole('link', { name: polish ? 'Analityka' : 'Analytics', exact: true }).click();
      await expect(staff).toHaveURL(/\/admin\/analytics$/);
      await staff.locator('main nav').getByRole('link', { name: labels[index], exact: true }).click();
      await staff.goto(filtered.url());
    } else await expect(cold).toHaveText(polish
      ? 'Niedostępne — wymagana zaakceptowana baza synchronizacji i aktualne dane'
      : 'Unavailable — accepted resync baseline and fresh evidence required');
    const { search } = new URL(staff.url());
    const data = await staff.request.get(`${path}/data${search}`);
    expect(data.status()).toBe(200);
    expect(data.headers()['cache-control']).toBe('no-store, private');
    const [htmlEntry, dataEntry] = await Promise.all([filtered, data].map(async response =>
      (await query('SELECT action,actor,target FROM audit_log WHERE request_id=$1', [
        response.headers()['x-request-id'],
      ]))[0]));
    expect(htmlEntry).toEqual({ action: 'admin.analytics.read', actor: manager.id, target: path });
    expect(dataEntry).toEqual({ action: 'admin.analytics.read', actor: manager.id, target: `${path}/data` });
    const body = await data.json();
    if (suffix === '/economy') {
      expect(body.state).toBe('available');
      expect(body.groups.filter(group => group.state === 'unavailable')).toHaveLength(5);
    } else {
      expect(body).toEqual({ state: 'unavailable', cells: [] });
    }
    expect((await staff.request.get(`${path}/data?world=invalid`)).status()).toBe(400);
    expect((await page.request.get(path)).status()).toBe(403);
    expect((await page.request.get(`${path}/data`)).status()).toBe(403);
  }
  await staff.goto('/admin/analytics');
  await staff.locator('main nav').getByRole('link', { name: polish ? 'Administracja' : 'Admin', exact: true }).click();
  await expect(staff).toHaveURL(/\/admin$/);
});

test('control step-up validates request binding and consumes a fresh TOTP', async ({ page }, info) => {
  const polish = info.project.metadata.locale === 'pl';
  const staff = manager.page;
  const path = '/admin/control/step-up';
  const request = (await query('SELECT uuidv7() AS id'))[0].id;
  const scoped = `${path}?request_id=${request}`;
  await staff.goto(path);
  await expect(staff.getByRole('heading', { level: 1 })).toHaveText(polish
    ? 'Potwierdź nowym kodem TOTP' : 'Confirm with a new TOTP code');
  const codeLabel = polish ? 'Sześciocyfrowy kod z aplikacji uwierzytelniającej' : 'Six-digit authenticator code';
  await expect(staff.getByLabel(codeLabel)).toBeVisible();
  await expect(staff.locator('main button')).toHaveText(polish ? 'Potwierdź' : 'Confirm');
  expect((await page.request.get(path, { maxRedirects: 0 })).status()).toBe(303);
  const review = '/admin/control/approvals';
  expect((await staff.request.get(`${review}/invalid`)).status()).toBe(422);
  expect((await staff.request.get(`${review}/${request}`)).status()).toBe(403);
  await page.goto(`/${info.project.metadata.locale}/login`);
  const guestToken = await page.locator('main input[name=_csrf]').inputValue();
  expect((await postAdmin(page, scoped, { _csrf: guestToken, code: '123456' })).status()).toBe(303);
  expect((await postAdmin(page, `${review}/${request}`, { _csrf: guestToken })).status()).toBe(303);
  const token = await staff.locator('main input[name=_csrf]').inputValue();
  expect((await postAdmin(staff, path, { _csrf: token, code: '123456' })).status()).toBe(422);
  expect((await postAdmin(staff, scoped, { _csrf: token, code: 'invalid' })).status()).toBe(422);
  await csrfRejected(staff, scoped, { code: '123456' });
  await csrfRejected(staff, `${review}/${request}`);
  expect((await postAdmin(staff, `${review}/${request}`, { _csrf: token })).status()).toBe(403);

  await staff.goto(scoped);
  await expect.poll(() => manager.totp.counter(), { timeout: 35_000 }).toBeGreaterThan(manager.enrollmentCounter);
  const code = manager.totp.generate();
  await staff.getByLabel(codeLabel).fill(code);
  const [success] = await Promise.all([staff.waitForNavigation(), staff.locator('main button').click()]);
  expect(success.status()).toBe(200);
  await expect(staff.getByRole('heading', { level: 1 })).toHaveText(polish
    ? 'Potwierdzenie zapisane. Wróć do pierwotnej operacji i wyślij ją ponownie.'
    : 'Confirmation recorded. Return to the original operation and submit it again.');
  const session = await query(
    "SELECT count(*)::int AS count FROM web_sessions WHERE account_id=$1 AND data->'admin_control_request_step_up'->>'request_id'=$2",
    [manager.id, request],
  );
  expect(session[0].count).toBe(1);
  expect((await postAdmin(staff, scoped, { _csrf: token, code })).status()).toBe(403);
  await staff.getByRole('link', { name: polish ? 'Administracja' : 'Administration', exact: true }).click();
  await expect(staff).toHaveURL(/\/admin$/);
  await roleAudit(staff, polish ? 'pl' : 'en', {
    action: 'admin.control.totp.verified', actor: manager.id, target: manager.id,
    before: null, after: { verified: true },
  });
});

test('roles pagination follows the cursor and keeps later roles actionable', async ({}, info) => {
  const lang = info.project.metadata.locale;
  const staff = manager.page;
  const prefix = `page_${manager.id.replaceAll('-', '')}`;
  const records = await query(
    "INSERT INTO roles(key) SELECT $1::text || '_' || n FROM generate_series(1,51) n RETURNING id,key",
    [prefix],
  );
  try {
    const last = [...records].sort((a, b) => a.id.localeCompare(b.id)).at(-1);
    await staff.goto('/admin/roles');
    await expect(staff.locator(`select[name=role] option[value="${last.id}"]`)).toHaveCount(0);
    const next = staff.getByRole('link', { name: lang === 'pl' ? 'Następna strona' : 'Next page', exact: true });
    const lastRole = staff.locator(`select[name=role] option[value="${last.id}"]`);
    do {
      await expect(next).toBeVisible();
      const destination = new URL(await next.getAttribute('href'), staff.url()).href;
      await Promise.all([
        staff.waitForURL(destination, { waitUntil: 'domcontentloaded' }),
        next.click(),
      ]);
    } while (!(await lastRole.count()));
    await expect(staff).toHaveURL(/\/admin\/roles\?after=[a-f0-9-]+$/);
    const cursorPage = staff.url();
    await expect(staff.locator('main section > ul > li').filter({ hasText: new RegExp(`^${last.key}$`) })).toBeVisible();
    const actor = seedAdminAccount(lang);
    await roleAction(staff, 'grant', { account: actor.id, role: last.id }, 303);
    expect(await accountRoles(actor.id)).toEqual([last.key]);
    await roleAudit(staff, lang, {
      action: 'rbac.role.granted', actor: manager.id, target: actor.id,
      before: { role: last.id, assigned: false }, after: { role: last.id, assigned: true },
    });
    await staff.goto(cursorPage);
    await roleAction(staff, 'revoke', { account: actor.id, role: last.id }, 303);
    expect(await accountRoles(actor.id)).toEqual([]);
    await roleAudit(staff, lang, {
      action: 'rbac.role.revoked', actor: manager.id, target: actor.id,
      before: { role: last.id, assigned: true }, after: { role: last.id, assigned: false },
    });
  } finally {
    await query('DELETE FROM roles WHERE id=ANY($1::uuid[])', [records.map(({ id }) => id)]);
  }
});
