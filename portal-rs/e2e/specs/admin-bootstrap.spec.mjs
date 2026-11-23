import {execFileSync} from 'node:child_process';
import {test, expect} from '../fixtures.mjs';
import {csrfRejected, mailLink, submit} from '../helpers.mjs';
import {action, query, seededAccount, seededLogin, seededMfaAccount} from '../staff-rbac-helpers.mjs';

const invitationPath = '/admin/roles/invitations';

test('operator grants a role and the invited account accepts it from email', async ({page, browser}, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const inviter = await seededMfaAccount(lang);
  const invitee = await seededAccount(lang);
  const {recoveryCodes} = JSON.parse(execFileSync(process.env.PORTAL_E2E_BINARY, [
    'seed', 'account-mfa', JSON.stringify({accountId: invitee.id, password: invitee.password}),
  ], {encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe']}));
  invitee.recoveryCode = recoveryCodes[0];
  const secondRecipient = await seededAccount(lang);
  const grant = () => execFileSync(process.env.PORTAL_E2E_BINARY, [
    'admin-cli', 'staff', 'grant-role', '--email', inviter.email, '--role', 'platform_admin',
  ], {encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe']}).trim();

  expect(grant()).toBe('Role granted');
  expect(grant()).toBe('Role already assigned');
  expect(await query(
    "SELECT actor_source FROM audit_log WHERE action='rbac.role.granted' AND target=$1",
    [inviter.id],
  )).toEqual([{actor_source: 'operator-cli'}]);

  const context = await browser.newContext({
    baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443', ignoreHTTPSErrors: true,
    locale: lang, viewport: info.project.use.viewport,
  });
  try {
    const staff = await context.newPage();
    await seededLogin(staff, lang, inviter);
    await staff.goto('/admin/roles');
    await staff.locator(`a[href="${invitationPath}"]`).click();
    await expect(staff).toHaveURL(new RegExp(invitationPath + '$'));
    const form = `form[action="${invitationPath}"]`;
    const role = await staff.locator(`${form} select option`).filter({hasText: /^support$/}).getAttribute('value');
    await action(staff, form, {email: invitee.email, role, days: '7'});
    await expect(staff.getByText(invitee.email, {exact: true})).toBeVisible();
    expect(await staff.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);

    await action(staff, form, {email: secondRecipient.email, role, days: '1'});
    const second = staff.locator('article').filter({hasText: secondRecipient.email});
    const revoked = await second.locator('form').getAttribute('action');
    await csrfRejected(staff, revoked, {});
    await second.getByRole('button').click();
    await expect(second).toHaveCount(0);

    const link = await mailLink(page, invitee.email, /https:\/\/[^\s/]+\/(?:pl|en)\/staff-invitation\?token(?:=3D|=)[A-Za-z0-9_-]+/, 60_000);
    await page.goto(link);
    await expect(page.locator(`main a[href="/${lang}/login"]`)).toBeVisible();
    await seededLogin(page, lang, invitee);
    await page.goto(link);
    await expect(page.locator('main')).toContainText('support');
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    await submit(page, 'main form[method=post]', {});
    await expect(page.locator('main')).toContainText(lang === 'pl' ? 'Przyjęto zaproszenie' : 'Invitation accepted');

    expect(await query(
      'SELECT r.key FROM account_roles ar JOIN roles r ON r.id=ar.role_id WHERE ar.account_id=$1',
      [invitee.id],
    )).toEqual([{key: 'support'}]);
    expect(await query(
      "SELECT action FROM audit_log WHERE actor=$1 AND action LIKE 'rbac.invitation.%' ORDER BY created_at",
      [inviter.id],
    )).toEqual([{action: 'rbac.invitation.created'}, {action: 'rbac.invitation.created'}, {action: 'rbac.invitation.revoked'}]);
    expect(await query(
      "SELECT count(*)::int AS count FROM audit_log WHERE actor=$1 AND action='rbac.invitation.accepted'",
      [invitee.id],
    )).toEqual([{count: 1}]);

    await seededLogin(page, lang, {...invitee, recoveryCode: recoveryCodes[1]});
    expect((await page.goto('/admin')).status()).toBe(200);
    expect((await page.goto(link)).status()).toBe(404);
  } finally {
    await context.close();
  }
});

test('i18n-parity for staff invitation confirmation', async ({request}, info) => {
  const lang = info.project.metadata.locale;
  const response = await request.get(`/${lang}/staff-invitation?token=invalid`);
  expect(response.status()).toBe(404);
  const html = await response.text();
  expect(html).toMatch(new RegExp(`<html lang="${lang}"[ >]`));
  expect(html).toContain(lang === 'pl' ? 'Zaproszenie do zespołu' : 'Staff invitation');
});
