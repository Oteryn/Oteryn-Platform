import {randomBytes} from 'node:crypto';
import {expect} from './fixtures.mjs';
import {csrfRejected,login,register,submit,waitForLoginBudget} from './helpers.mjs';
import {enrollMfa,query} from './staff-rbac-helpers.mjs';

const seniorPermissions = [
  'admin.access',
  'game.analytics.read',
  'game.audit.read',
  'game.db.read',
  'game.economy.moderate',
  'game.economy.read',
  'game.node.read',
  'game.ops.logs.read',
  'game.ops.read',
  'game.player.manage',
  'game.player.moderate',
  'game.player.read',
  'game.player.value.write',
  'game.sanction.ban',
  'game.sanction.read',
  'game.sanction.write',
  'game.world.events',
  'game.world.read',
];

export async function grantSeniorRole(admin, accountId) {
  const form = admin.locator('main form:has(button[value="grant"])');
  const role = await form.locator('select[name=role]').selectOption({label:'game_master_senior'});
  await form.locator('[name=account]').fill(accountId);
  const fields = {account:accountId, role:role[0], action:'grant'};
  await csrfRejected(admin, '/admin/roles', fields);
  const csrf = await form.locator('[name=_csrf]').inputValue();
  const invalid = await admin.request.post('/admin/roles', {
    form:{...fields, account:'00000000-0000-0000-0000-000000000000', _csrf:csrf},
    headers:{origin:new URL(admin.url()).origin},
    maxRedirects:0,
  });
  expect(invalid.status(), 'a role assignment requires a real target account').toBe(400);
  const unassigned = await query('SELECT role_id FROM account_roles WHERE account_id=$1', [accountId]);
  expect(unassigned).toEqual([]);
  await form.getByRole('button', {name:/^(Assign role|Przypisz rolę)$/}).click();
  await expect(admin).toHaveURL(/\/admin\/roles$/);
  await expect(form.locator('select[name=role] option').filter({hasText:/^game_master_senior$/})).toHaveCount(1);
  const assigned = await query(`
    SELECT r.key, p.permission
    FROM account_roles ar
    JOIN roles r ON r.id=ar.role_id
    LEFT JOIN role_permissions p ON p.role_id=r.id
    WHERE ar.account_id=$1
    ORDER BY p.permission
  `, [accountId]);
  expect(assigned).toEqual(seniorPermissions.map(permission => ({key:'game_master_senior', permission})));
}

export async function grantPlatformAuditRole(admin, accountId, lang) {
  const key = `e2e_gm_${randomBytes(8).toString('hex')}`;
  const permissions = ['audit.view'];
  const creation = 'main form:has(button[value="create"])';
  await submit(admin, creation, {key, permissions:permissions.join(' ')});
  await expect(admin.locator(`${creation} input[name=key]`)).toHaveValue('');
  const grant = admin.locator('main form:has(button[value="grant"])');
  const option = grant.locator('option').filter({hasText:new RegExp(`^${key}$`)});
  // The creation redirects to the first page of the role list.
  while (!(await option.count())) {
    const next = admin.locator('main a[href^="/admin/roles?after="]');
    await expect(next).toBeVisible();
    const destination = new URL(await next.getAttribute('href'), admin.url()).href;
    await Promise.all([
      admin.waitForURL(destination, {waitUntil:'domcontentloaded'}),
      next.click(),
    ]);
  }
  const role = await option.getAttribute('value');
  const actual = await query('SELECT permission FROM role_permissions WHERE role_id=$1 ORDER BY permission', [role]);
  expect(actual.map(row => row.permission)).toEqual(permissions);
  await csrfRejected(admin, '/admin/roles', {account:accountId, role, action:'grant'});
  await grant.locator('[name=account]').fill(accountId);
  await grant.locator('select[name=role]').selectOption(role);
  await grant.getByRole('button', {name:lang === 'pl' ? 'Przypisz rolę' : 'Assign role', exact:true}).click();
  await expect(admin).toHaveURL(/\/admin\/roles$/);
  const assigned = await query('SELECT r.key FROM account_roles ar JOIN roles r ON r.id=ar.role_id WHERE ar.account_id=$1 ORDER BY r.key', [accountId]);
  expect(assigned.map(row => row.key)).toEqual([key, 'game_master_senior']);
  return role;
}

export async function requestForm(page, endpoint, form, csrf) {
  return page.request.post(new URL(endpoint, page.url()).href, {
    form:{...form, _csrf:csrf},
    headers:{origin:new URL(page.url()).origin},
    maxRedirects:0,
  });
}

export async function completeEnrollment(page, lang, account) {
  await enrollMfa(page, lang, account);
  const firstCode = page.locator('main textarea[readonly]').first();
  await expect(firstCode).toBeVisible();
  const code = await firstCode.inputValue();
  expect(code.length).toBeGreaterThan(0);
  await page.locator(`main a[href="/${lang}/login"]`).click();
  await waitForLoginBudget();
  await submit(page, 'main form', {email:account.email, password:account.password});
  await expect(page).toHaveURL(/\/mfa\/challenge$/);
  await submit(page, 'main form[action$="/mfa/challenge"]', {code});
  await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
}

export async function bootstrapAdmin(browser, info) {
  const lang = info.project.metadata.locale;
  const context = await browser.newContext({
    baseURL:process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443',
    ignoreHTTPSErrors:true,
    viewport:info.project.use.viewport,
    locale:lang,
  });
  let id;
  const close = async () => {
    await context.close();
    if (id) await query('DELETE FROM account_roles WHERE account_id=$1', [id]);
  };
  try {
    const page = await context.newPage();
    const account = await register(page, lang);
    // First administrator fixture, matching the existing staff RBAC suite.
    const [membership] = await query(`
      INSERT INTO account_roles(account_id,role_id)
      SELECT a.id,r.id FROM accounts a CROSS JOIN roles r
      WHERE a.email=$1 AND r.key='platform_admin'
      RETURNING account_id
    `, [account.email]);
    id = membership.account_id;
    await login(page, lang, account);
    await completeEnrollment(page, lang, account);
    return {page, close};
  } catch (error) {
    await close();
    throw error;
  }
}
