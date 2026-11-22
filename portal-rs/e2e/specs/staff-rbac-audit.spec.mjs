import { test, expect } from '../fixtures.mjs';
import { audit, isolatedAdmin, membership, player, role } from '../staff-rbac-helpers.mjs';

test('audit shows the localized before and after values of a role grant', async ({ page, browser }, info) => {
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
    const [field, before, after, assigned, no, yes] = lang === 'pl'
      ? ['Pole', 'Przed', 'Po', 'Przypisanie', 'Nie', 'Tak']
      : ['Field', 'Before', 'After', 'Assigned', 'No', 'Yes'];
    await expect(table.getByRole('columnheader')).toHaveText([field, before, after]);
    const change = table.getByRole('row').filter({ has: staff.page.getByRole('rowheader', { name: assigned, exact: true }) });
    await expect(change.getByRole('cell')).toHaveText([no, yes]);
  } finally {
    await staff.context.close();
  }
});
