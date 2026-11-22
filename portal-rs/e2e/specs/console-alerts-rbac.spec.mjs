import { spawnSync } from 'node:child_process';
import { test, expect } from '../fixtures.mjs';
import { login, password, pool } from '../helpers.mjs';
import { loginWithRecoveryCode, query, unique } from '../staff-rbac-helpers.mjs';

async function seed(entity, args) {
  const binary = process.env.PORTAL_E2E_BINARY;
  if (!binary) throw new Error('Set PORTAL_E2E_BINARY to the harness portal binary');
  const connection = pool();
  const database = connection.options.connectionString;
  await connection.end();
  const result = spawnSync(binary, ['seed', entity, JSON.stringify(args)], {
    encoding: 'utf8',
    env: { ...process.env, OTERYN_DATABASE_URL: database },
  });
  expect(result.status, `seed ${entity} succeeded`).toBe(0);
  return JSON.parse(result.stdout);
}

async function signIn(page, lang, role) {
  const credentials = { email: `${unique()}@example.invalid`, password: password() };
  const { accountId } = await seed('account', { ...credentials, locale: lang });
  if (role) {
    const { recoveryCodes } = await seed('account-mfa', { accountId, password: credentials.password });
    const grants = await query(
      'INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2 AND built_in RETURNING role_id',
      [accountId, role],
    );
    expect(grants).toHaveLength(1);
    await loginWithRecoveryCode(page, lang, credentials, recoveryCodes[0]);
  } else {
    await login(page, lang, credentials);
  }
  expect((await page.goto(`/${lang}/`)).status()).toBe(200);
  await expect(page.locator('[data-account-link]')).toBeAttached();
  return accountId;
}

async function accountMenu(page) {
  const toggle = page.locator('#portal-menu-toggle');
  if (await toggle.isVisible()) await toggle.click();
  await page.locator('[data-account-link]').click();
  const menu = page.locator('#podglad-konta');
  await expect(menu).toBeVisible();
  return menu;
}

async function visit(page, path, expected) {
  const response = await page.goto(path);
  expect(response.status(), path).toBe(expected);
  return response;
}

test('the player account menu links passkeys and excludes the staff dashboard', async ({ page }, { project }) => {
  const lang = project.metadata.locale;
  await signIn(page, lang);
  const menu = await accountMenu(page);
  await expect(menu.locator('a[href="/admin"]')).toHaveCount(0);
  const passkeys = menu.locator(`a[href="/${lang}/account/passkeys"]`);
  await expect(passkeys).toBeVisible();
  const navigation = page.waitForNavigation();
  await passkeys.click();
  expect((await navigation).status()).toBe(200);
  await visit(page, '/admin/moderation/reports', 403);
});

test('a moderator reaches the dashboard and reports with MFA, and passkeys stay hidden', async ({ page }, { project }) => {
  const lang = project.metadata.locale;
  await signIn(page, lang, 'moderator');
  const menu = await accountMenu(page);
  await expect(menu.locator(`a[href="/${lang}/account/passkeys"]`)).toHaveCount(0);
  const dashboard = menu.getByRole('link', {
    name: lang === 'pl' ? 'Panel administracyjny' : 'Staff dashboard',
    exact: true,
  });
  await expect(dashboard).toHaveAttribute('href', '/admin');
  const navigation = page.waitForNavigation();
  await dashboard.click();
  expect((await navigation).status()).toBe(200);
  await visit(page, '/admin/moderation/reports', 200);
  await visit(page, '/admin/moderation/reports/malformed', 400);
  await visit(page, '/admin/roles', 403);
  await visit(page, `/${lang}/account/passkeys`, 403);
  await visit(page, `/admin/console/players?lang=${lang}`, 403);
  await visit(page, `/admin/console/audit?lang=${lang}`, 403);
  await visit(page, '/admin/analytics', 403);
});

test('a senior GM can open the Game views while binding alarms require a separate grant', async ({ page }, { project }) => {
  const lang = project.metadata.locale;
  const binding = unique();
  const [{ id }] = await query(
    `INSERT INTO audit_log(actor,action,target,before,after,request_id)
     VALUES(NULL,'ops.playerdata_binding_conflict',$1,'null',$2,uuidv7()) RETURNING id`,
    [binding, { authority: 'test-journal', family: 'BANK', record_id: '019a0000-0000-7000-8000-000000000008' }],
  );
  const gm = await signIn(page, lang, 'game_master_senior');
  await visit(page, `/admin/console/players?lang=${lang}&q=Synthetic`, 200);
  await visit(page, `/admin/console/players?lang=${lang}&q=%25`, 400);
  await visit(page, `/admin/console/audit?lang=${lang}`, 200);
  await visit(page, `/admin/console/audit?lang=${lang}&request_id=bad`, 400);
  const analytics = await visit(page, '/admin/analytics', 200);
  expect(analytics.headers()['cache-control']).toBe('no-store, private');
  await expect(page.locator('[data-state="unavailable"]')).toBeVisible();
  await visit(page, '/admin/analytics?world=bad', 400);
  await visit(page, `/console/dashboard?lang=${lang}`, 200);
  await expect(page.locator('main')).not.toContainText('IMMUTABLE_BINDING');
  await expect(page.locator('main')).not.toContainText(binding);
  await expect(page.locator('main')).not.toContainText(id);

  // A general auditor can read other Platform audit entries but cannot read the conflict.
  await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='security_admin' AND built_in", [gm]);
  const visible = unique();
  await query(
    "INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES($1,'test.audit.visible',$2,'null','null',uuidv7())",
    [gm, visible],
  );
  await visit(page, `/admin/console/audit?lang=${lang}&target=${visible}`, 200);
  await expect(page.locator('main')).toContainText(visible);
  await visit(page, `/admin/console/audit?lang=${lang}&operation=ops.playerdata_binding_conflict&target=${binding}`, 200);
  await expect(page.locator('main')).not.toContainText('ops.playerdata_binding_conflict ·');
  await expect(page.locator('main')).not.toContainText(binding);
});

test('a Game administrator sees well-formed Platform alarms while Game is offline', async ({ page }, { project }) => {
  const lang = project.metadata.locale;
  const binding = unique();
  const malformed = unique();
  const [{ id }] = await query(
    `INSERT INTO audit_log(actor,action,target,before,after,request_id)
     VALUES(NULL,'ops.playerdata_binding_conflict',$1,'null',$2,uuidv7()) RETURNING id`,
    [binding, { authority: 'test-journal', family: 'BANK', record_id: '019a0000-0000-7000-8000-000000000008' }],
  );
  await query(
    `INSERT INTO audit_log(actor,action,target,before,after,request_id)
     VALUES(NULL,'ops.playerdata_binding_conflict',$1,'null','{}',uuidv7())`,
    [malformed],
  );
  const admin = await signIn(page, lang, 'game_admin');
  await visit(page, `/console/dashboard?lang=${lang}&target=invalid`, 400);
  await visit(page, `/console/dashboard?lang=${lang}`, 200);
  const main = page.locator('main');
  await expect(main).toContainText('IMMUTABLE_BINDING');
  await expect(main).toContainText(binding);
  await expect(main).toContainText(id);
  await expect(main).not.toContainText(malformed);
  await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='security_admin' AND built_in", [admin]);
  await visit(page, `/admin/console/audit?lang=${lang}&operation=ops.playerdata_binding_conflict&target=${binding}`, 200);
  await expect(main).toContainText(`ops.playerdata_binding_conflict · ${binding}`);
});
