import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login } from '../helpers.mjs';
import { loginWithRecoveryCode, query, seedAccount, unique } from '../staff-rbac-helpers.mjs';

async function signIn(page, lang, role) {
  const { credentials, accountId, recoveryCodes } = await seedAccount(lang, role);
  if (role) {
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
  const moderator = await signIn(page, lang, 'moderator');
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
  const { accountId: reporter } = await seedAccount(lang);
  const [{ id: report }] = await query(
    `INSERT INTO player_reports(reporter_account_id,request_key,report_type,category,target_reference)
     VALUES($1,uuidv7(),'content','spam',$2) RETURNING id`,
    [reporter, unique()],
  );
  const reportPath = `/admin/moderation/reports/${report}`;
  await visit(page, reportPath, 200);
  const form = page.locator('form:has(input[value="moderate"])');
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const notes = unique();
  await csrfRejected(page, reportPath, { ...fields, status: 'reviewing', public_outcome: '', moderator_notes: notes });
  const readReport = () => query(
    'SELECT status, lock_version::int, assigned_to, moderator_notes FROM player_reports WHERE id=$1',
    [report],
  );
  const readAudit = () => query(
    "SELECT actor, before->>'status' AS previous, after->>'status' AS current FROM audit_log WHERE action='support.report_status_changed' AND target=$1",
    [report],
  );
  expect(await readReport()).toEqual([{ status: 'submitted', lock_version: 1, assigned_to: null, moderator_notes: null }]);
  expect(await readAudit()).toEqual([]);

  await form.locator('select[name="status"]').selectOption('reviewing');
  await form.locator('[name="public_outcome"]').fill('');
  await form.locator('[name="moderator_notes"]').fill(notes);
  const updated = page.waitForResponse(response => response.request().method() === 'POST' && new URL(response.url()).pathname === reportPath);
  await form.locator('button').click();
  expect((await updated).status()).toBe(303);
  await expect(page).toHaveURL(new RegExp(`${reportPath}$`));
  expect(await readReport()).toEqual([{ status: 'reviewing', lock_version: 2, assigned_to: moderator, moderator_notes: notes }]);
  expect(await readAudit()).toEqual([{ actor: moderator, previous: 'submitted', current: 'reviewing' }]);
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
  const alarm = main.locator('article').filter({ hasText: binding });
  await expect(alarm.locator('dt')).toHaveText(lang === 'pl'
    ? ['Alarm', 'UTC', 'Źródło danych', 'Rodzina', 'Rekord', 'Powiązanie']
    : ['Alarm', 'UTC', 'Authority', 'Family', 'Record', 'Binding']);
  await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='security_admin' AND built_in", [admin]);
  await visit(page, `/admin/console/audit?lang=${lang}&operation=ops.playerdata_binding_conflict&target=${binding}`, 200);
  await expect(main).toContainText(`ops.playerdata_binding_conflict · ${binding}`);
});
