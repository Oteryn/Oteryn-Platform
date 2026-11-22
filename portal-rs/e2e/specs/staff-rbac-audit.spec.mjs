import { test, expect } from '../fixtures.mjs';
import { login } from '../helpers.mjs';
import { audit, enrollMfa, isolatedAdmin, loginWithRecoveryCode, membership, player, role } from '../staff-rbac-helpers.mjs';

test('Platform audit shows role changes and keeps them from Game audit staff', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const account = await player(page, lang);
  const staff = await isolatedAdmin(browser, info);

  try {
    const id = await role(staff.page, 'admin.access audit.view');
    await audit(staff.page, 'rbac.role.created', id, { lang });
    const auditUrl = new URL(staff.page.url());
    expect((await page.goto(auditUrl.pathname + auditUrl.search)).status()).toBe(403);

    await membership(staff.page, account.id, id, 'grant');
    await audit(staff.page, 'rbac.role.granted', account.id, { lang });

    const table = staff.page.locator('main').getByRole('table');
    const [field, before, after, assigned, no, yes, roleId, platformSource, gameSource] = lang === 'pl'
      ? ['Pole', 'Przed', 'Po', 'Przypisanie', 'Nie', 'Tak', 'Identyfikator roli', 'Platforma · audyt, ostatnie 100 dopasowań', 'Agent gry · czas obserwacji']
      : ['Field', 'Before', 'After', 'Assigned', 'No', 'Yes', 'Role ID', 'Platform · audit log, latest 100 matches', 'Game agent · observed at'];
    await expect(table.getByRole('columnheader')).toHaveText([field, before, after]);
    const change = table.getByRole('row').filter({ has: staff.page.getByRole('rowheader', { name: assigned, exact: true }) });
    await expect(change.getByRole('cell')).toHaveText([no, yes]);
    const summary = staff.page.locator('main li > p').filter({ hasText: `${roleId} ${id}` });
    await expect(summary).toContainText('rbac.role.granted');
    const grantUrl = new URL(staff.page.url());

    await membership(staff.page, account.id, id, 'revoke');
    await audit(staff.page, 'rbac.role.revoked', account.id, { lang });
    await expect(summary).toContainText('rbac.role.revoked');
    await expect(change.getByRole('cell')).toHaveText([yes, no]);

    const gameRole = await role(staff.page, 'admin.access game.audit.read');
    await login(page, lang, account);
    const codes = await enrollMfa(page, lang, account);
    await membership(staff.page, account.id, gameRole, 'grant');
    await loginWithRecoveryCode(page, lang, account, codes[0]);
    expect((await page.goto(grantUrl.pathname + grantUrl.search)).status()).toBe(200);
    const main = page.locator('main');
    await expect(main.getByRole('heading', { name: gameSource, exact: true })).toBeVisible();
    await expect(main.getByRole('heading', { name: platformSource, exact: true })).toHaveCount(0);
    await expect(main.getByRole('table')).toHaveCount(0);
    await expect(main).not.toContainText(id);
  } finally {
    await staff.context.close();
  }
});
