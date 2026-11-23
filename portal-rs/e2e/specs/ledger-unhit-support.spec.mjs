import {test, expect} from '../fixtures.mjs';
import {login, submit} from '../helpers.mjs';
import {isolatedStaff, query, seededAccount, unique} from '../staff-rbac-helpers.mjs';

test('player closes a ticket and staff resolves another through the PUT endpoints', async ({page, browser}, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  await login(page, lang, await seededAccount(lang));
  const staff = await isolatedStaff(browser, info, 'platform_admin', await seededAccount(lang));
  try {
    for (const [actor, status, scope] of [[page, 'closed', ''], [staff.page, 'resolved', 'admin/']]) {
      const tickets = `/${lang}/support/tickets`;
      const subject = unique();
      expect((await page.goto(`${tickets}/create`)).status()).toBe(200);
      await submit(page, 'main form', {subject, body: 'Local ticket for status qualification.'});
      await expect(page).toHaveURL(/\/support\/tickets\/[a-f0-9-]+$/);
      const id = new URL(page.url()).pathname.split('/').at(-1);
      const path = `/${lang}/${scope}support/tickets/${id}`;
      expect((await actor.goto(path)).status()).toBe(200);
      const form = actor.locator('main form[action$="/status"]');
      await form.locator('[name=status]').selectOption(status);
      if (scope) await form.locator('[name=reason]').fill('Resolve the support ticket');
      const fields = await form.evaluate(f => Object.fromEntries(new FormData(f)));
      const changed = await actor.request.put(`${path}/status`, {
        form: fields, headers: {origin: new URL(actor.url()).origin}, maxRedirects: 0,
      });
      expect(changed.status()).toBe(303);
      expect(changed.headers().location).toBe(path);
      expect(await query('SELECT status FROM tickets WHERE id=$1', [id])).toEqual([{status}]);
      expect((await page.goto(`${tickets}/${id}`)).status()).toBe(200);
    }
  } finally {
    await staff.context.close();
  }
});
