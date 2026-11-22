import { randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { login, submit } from '../helpers.mjs';
import { audit, confirmStaffPassword, enrollMfa, isolatedStaff, loginWithRecoveryCode, membership, query, role, seededAccount } from '../staff-rbac-helpers.mjs';

test('Platform audit shows role changes and keeps them from Game audit staff', async ({ page, browser }, info) => {
  test.setTimeout(480_000);
  const lang = info.project.metadata.locale;
  const account = await seededAccount(lang);
  await login(page, lang, account);
  const staff = await isolatedStaff(browser, info, 'platform_admin', await seededAccount(lang));

  try {
    const id = await role(staff.page, 'admin.access audit.view');
    await audit(staff.page, 'rbac.role.created', id, { lang });
    const auditUrl = new URL(staff.page.url());
    expect((await page.goto('/admin/console/audit')).status()).toBe(403);
    expect((await page.goto(auditUrl.pathname + auditUrl.search)).status()).toBe(403);
    const codes = await enrollMfa(page, lang, account);

    await confirmStaffPassword(staff.page, lang, staff.account.password);
    await membership(staff.page, account.id, id, 'grant');
    await audit(staff.page, 'rbac.role.granted', account.id, { lang });

    const table = staff.page.locator('main').getByRole('table');
    await expect(table).toBeVisible();
    expect(await staff.page.evaluate(() => document.documentElement.scrollWidth)).toBe(info.project.use.viewport.width);
    if (info.project.use.viewport.width === 390) {
      const wrapper = staff.page.locator('main .op-table-wrap');
      expect(await wrapper.evaluate(el => el.scrollWidth > el.clientWidth)).toBe(true);
    }
    const [field, before, after, assigned, no, yes, roleId, platformSource, gameSource] = lang === 'pl'
      ? ['Pole', 'Przed', 'Po', 'Przypisanie', 'Nie', 'Tak', 'Identyfikator roli', 'Platforma · audyt, ostatnie 100 dopasowań', 'Agent gry · czas obserwacji']
      : ['Field', 'Before', 'After', 'Assigned', 'No', 'Yes', 'Role ID', 'Platform · audit log, latest 100 matches', 'Game agent · observed at'];
    await expect(table.getByRole('columnheader')).toHaveText([field, before, after]);
    const change = table.getByRole('row').filter({ has: staff.page.getByRole('rowheader', { name: assigned, exact: true }) });
    await expect(change.getByRole('cell')).toHaveText([no, yes]);
    const summary = staff.page.locator('main li > p').filter({ hasText: `${roleId} ${id}` });
    await expect(summary).toContainText('rbac.role.granted');
    const grantUrl = new URL(staff.page.url());

    await loginWithRecoveryCode(page, lang, account, codes[0]);
    const slug = `draft-client-case-${randomUUID()}`;
    await test.step('the audit-only account can find the private page event by request ID', async () => {
      expect((await staff.page.goto(`/admin/pages/${slug}/en`)).status()).toBe(200);
      await submit(staff.page, 'main form', { title: 'Private draft', body: 'Draft content' });
      await expect(staff.page).toHaveURL(/\/admin\/pages$/);
      const [{ request_id }] = await query(
        "SELECT request_id FROM audit_log WHERE action='cms.page.saved' AND target=$1",
        [slug],
      );
      const params = new URLSearchParams({ lang, operation: 'cms.page.saved', target: slug });
      expect((await page.goto(`/admin/console/audit?${params}`)).status()).toBe(200);
      const main = page.locator('main');
      const pageSummary = main.locator('li > p').filter({ hasText: 'cms.page.saved' });
      await expect(pageSummary).toHaveCount(0);
      params.delete('target');
      params.set('request_id', request_id);
      expect((await page.goto(`/admin/console/audit?${params}`)).status()).toBe(200);
      await expect(pageSummary).toContainText(request_id);
      await expect(pageSummary).toContainText(lang === 'pl' ? 'Identyfikator strony ukryty' : 'Page target hidden');
      await expect(main).not.toContainText(slug);
    });

    await test.step('a denied page editor visit is recorded and shown as a route pattern', async () => {
      expect((await page.goto(`/admin/pages/${slug}/en`)).status()).toBe(403);
      const [{ request_id, target }] = await query(
        "SELECT request_id,target FROM audit_log WHERE action='admin.access.denied' AND actor=$1 ORDER BY created_at DESC,id DESC LIMIT 1",
        [account.id],
      );
      const pattern = '/admin/pages/{slug}/{locale}';
      expect(target).toBe(pattern);
      const params = new URLSearchParams({ lang, request_id, operation: 'admin.access.denied' });
      expect((await page.goto(`/admin/console/audit?${params}`)).status()).toBe(200);
      const main = page.locator('main');
      const denial = main.locator('li > p').filter({ hasText: 'admin.access.denied' });
      await expect(denial).toContainText(request_id);
      await expect(denial).toContainText(pattern);
      await expect(main).not.toContainText(slug);
    });

    await membership(staff.page, account.id, id, 'revoke');
    await audit(staff.page, 'rbac.role.revoked', account.id, { lang });
    await expect(summary).toContainText('rbac.role.revoked');
    await expect(change.getByRole('cell')).toHaveText([yes, no]);

    const gameRole = await role(staff.page, 'admin.access game.audit.read');
    await confirmStaffPassword(staff.page, lang, staff.account.password);
    await membership(staff.page, account.id, gameRole, 'grant');
    await loginWithRecoveryCode(page, lang, account, codes[1]);
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
