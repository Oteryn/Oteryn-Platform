import { test, expect } from '../fixtures.mjs';
import { submit } from '../helpers.mjs';
import { action, audit, isolatedAdmin, player, query, unique } from '../staff-rbac-helpers.mjs';

test('players appeal their own sanctions and staff uphold or overturn them', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const index = `/${lang}/support/appeals`;
  expect((await page.goto(index)).status()).toBe(403);
  const owner = await player(page, lang);
  const staff = await isolatedAdmin(browser, info);
  try {
    for (const [decision, sanctionStatus, label] of [
      ['accepted', 'revoked', lang === 'pl' ? 'Uwzględnione' : 'Overturned'],
      ['rejected', 'active', lang === 'pl' ? 'Odrzucone' : 'Upheld'],
    ]) {
      const reason = unique();
      const privateNote = unique();
      const message = `Please review <${unique()}>`;
      const outcome = `Reviewed ${unique()}`;
      expect((await staff.page.goto('/admin/moderation/enforcement/create')).status()).toBe(200);
      const now = Math.floor(Date.now() / 1000);
      await action(staff.page, 'form:has(input[value=sanction])', {
        target: owner.id,
        category: 'restriction',
        status: 'active',
        public_reason: reason,
        moderator_notes: privateNote,
        effective_at: String(now - 1),
        expires_at: String(now + 3600),
      });
      await expect(staff.page).toHaveURL(/enforcement\/[a-f0-9-]+$/);
      const staffPath = new URL(staff.page.url()).pathname;
      const id = staffPath.split('/').at(-1);
      const playerPath = `${index}/${id}`;

      expect((await staff.page.request.get(playerPath)).status()).toBe(404);
      expect((await page.request.get(staffPath)).status()).toBe(403);
      const list = await page.goto(index);
      expect(list.status()).toBe(200);
      expect(list.headers()['cache-control']).toContain('no-store');
      await expect(page.locator('html')).toHaveAttribute('lang', lang);
      await page.getByRole('link', { name: reason, exact: true }).click();
      await expect(page).toHaveURL(playerPath);
      await expect(page.locator('main')).not.toContainText(privateNote);
      await submit(page, 'main form', { message });
      await expect(page.locator('main')).toContainText(message);
      await expect(page.locator('main')).toContainText(lang === 'pl' ? 'Oczekuje na rozpatrzenie' : 'Awaiting review');

      expect((await staff.page.goto("/admin/moderation/enforcement")).status()).toBe(200);
      await expect(staff.page.getByRole("link", { name: reason, exact: true })).toHaveAttribute("href", staffPath);
      expect((await staff.page.goto(staffPath)).status()).toBe(200);
      await expect(staff.page.locator('main')).toContainText(message);
      const form = staff.page.locator('form:has(input[value=appeal])');
      const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
      await action(staff.page, 'form:has(input[value=appeal])', { appeal_status: decision, appeal_outcome: outcome });
      await expect(staff.page.locator('main')).toContainText(outcome);
      await expect(form).toHaveCount(0);

      const repeated = await staff.page.request.post(staffPath, {
        form: { ...fields, appeal_status: decision, appeal_outcome: outcome },
        headers: { origin: new URL(staff.page.url()).origin },
        maxRedirects: 0,
      });
      expect(repeated.status()).toBe(409);
      expect((await query('SELECT status FROM sanctions WHERE id=$1', [id]))[0].status).toBe(sanctionStatus);
      await page.reload();
      await expect(page.locator('main')).toContainText(label);
      await expect(page.locator('main')).toContainText(outcome);
      await expect(page.locator('main')).not.toContainText(privateNote);
      await expect(page.locator('main form')).toHaveCount(0);
      await audit(staff.page, 'support.enforcement_appeal_updated', id);
      expect(await query(
        'SELECT actor FROM audit_log WHERE action=$1 AND target=$2',
        ['support.enforcement_appeal_submitted', id],
      )).toEqual([{ actor: owner.id }]);
      expect(await query(
        'SELECT action FROM audit_log WHERE action=$1 AND target=$2',
        ['support.enforcement_appeal_updated', id],
      )).toHaveLength(1);
    }
  } finally {
    await staff.context.close();
  }
});
