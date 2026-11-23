import {readFileSync} from 'node:fs';
import {randomUUID} from 'node:crypto';
import {test, expect} from '../fixtures.mjs';
import {assertShell} from '../shell.mjs';
import {query, seededStaff} from '../staff-rbac-helpers.mjs';

test('staff opens System and reads live portal status', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  for (const path of ['/admin/system', '/admin/system/data']) {
    const response = await page.request.get(path, {maxRedirects: 0});
    expect(response.status()).toBe(303);
    expect(response.headers().location).toBe('/login');
  }
  const actor = await seededStaff(page, lang, 'platform_admin');
  expect((await page.goto('/admin')).status()).toBe(200);
  await page.getByRole('link', {name: 'System', exact: true}).click();
  await expect(page).toHaveURL(/\/admin\/system$/);
  await expect(page.getByRole('heading', {name: 'System', level: 1})).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await assertShell(page);
  await expect(page.getByText(lang === 'pl' ? 'Testowe' : 'Testing', {exact: true})).toBeVisible();
  expect((await page.request.get('/admin/system/unknown', {maxRedirects: 0})).status()).toBe(404);
  expect((await page.request.fetch('/admin/system/data', {method: 'OPTIONS'})).status()).toBe(405);

  const otherLang = lang === 'pl' ? 'en' : 'pl';
  await page.context().addCookies([{name: 'portal_language', value: otherLang, url: page.url()}]);
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('lang', otherLang);
  await expect(page.getByText(otherLang === 'pl' ? 'Testowe' : 'Testing', {exact: true})).toBeVisible();
  await page.context().clearCookies({name: 'portal_language'});

  const key = randomUUID();
  const [{id: job}] = await query(
    "INSERT INTO outbox(kind,payload,idempotency_key,created_at,run_after) VALUES('sysops.test','{}',$1,now()-interval '5 minutes',now()+interval '1 day') RETURNING id",
    [key],
  );
  try {
    const [{id: mail}] = await query(
      "INSERT INTO email_outbox(account_id,category,recipient,subject,text_body,html_body) VALUES($1,'security','status@example.invalid','Status','Status','Status') RETURNING id",
      [actor],
    );
    try {
      await query(
        "INSERT INTO outbox(kind,payload,idempotency_key,run_after) VALUES('notify.email',to_jsonb($1::text),$1,now()+interval '1 day')",
        [mail],
      );
      await page.reload();
      await expect(page.locator('dt:has(code:text-is("OTERYN_ADMIN_PANEL")) + dd')).toContainText(
        lang === 'pl' ? 'Włączona' : 'On',
      );
      await page.locator('summary#system-configuration').click();
      await expect(page.locator('dt:has(code:text-is("OTERYN_DATABASE_URL")) + dd')).toContainText(
        lang === 'pl' ? 'Ustawiono' : 'Set',
      );
      let data;
      await expect.poll(async () => {
        const response = await page.request.get('/admin/system/data', {headers: {'accept-language': lang}});
        expect(response.status()).toBe(200);
        expect(response.headers()['cache-control']).toContain('no-store');
        const body = await response.text();
        expect(/postgres(?:ql)?:\/\//.test(body)).toBe(false);
        data = JSON.parse(body);
        return data.outbox.value?.oldest_age_seconds >= 300 && data.mail_outbox.value >= 1;
      }, {intervals: [250, 500, 1000]}).toBe(true);
      expect(data.environment).toBe('testing');
      expect(data.build_version).toMatch(/^\d+\.\d+\.\d+/);
      expect(data.build_commit).toMatch(/^[a-f0-9]{40}$/);
      expect(data.uptime_seconds).toBeGreaterThanOrEqual(0);
      expect(data.postgres).toMatchObject({state: 'ok', value: expect.stringMatching(/^\d+/)});
      expect(data.migrations.state).toBe('ok');
      expect(data.migrations.value.applied).toBe(data.migrations.value.expected);
      expect(data.migrations.value.expected).toBeGreaterThanOrEqual(20261124110400);
      expect(data.worker.value).toBeGreaterThanOrEqual(0);
      expect(data.worker.state).toBe('ok');
      expect(data.outbox.value.backlog).toBeGreaterThanOrEqual(1);
      expect(data.outbox.value.oldest_age_seconds).toBeGreaterThanOrEqual(300);
      expect(data.mail_outbox.value).toBeGreaterThanOrEqual(1);
      for (const contact of [data.gateway, data.game_link]) {
        expect(['unknown', 'observed']).toContain(contact.state);
        if (contact.state === 'observed') {
          expect(contact.value).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/);
        }
      }
      expect(data.flags).toContainEqual({
        name: 'OTERYN_ADMIN_PANEL', enabled: true,
        description: lang === 'pl' ? 'Udostępnia panel zespołu.' : 'Enables the staff panel.',
      });
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      expect(data.configuration).toContainEqual({name: 'OTERYN_DATABASE_URL', state: 'set'});
    } finally {
      await query("DELETE FROM outbox WHERE kind='notify.email' AND idempotency_key=$1", [mail]);
      await query('DELETE FROM email_outbox WHERE id=$1', [mail]);
    }
  } finally {
    await query('DELETE FROM outbox WHERE id=$1', [job]);
  }

  await query(
    "UPDATE account_roles SET role_id=(SELECT id FROM roles WHERE key='support') WHERE account_id=$1 AND role_id=(SELECT id FROM roles WHERE key='platform_admin')",
    [actor],
  );
  expect((await page.goto('/admin')).status()).toBe(200);
  await expect(page.getByRole('link', {name: 'System', exact: true})).toHaveCount(0);
  for (const path of ['/admin/system', '/admin/system/data']) {
    expect((await page.request.get(path, {maxRedirects: 0})).status()).toBe(403);
  }
});

test('i18n-parity for System labels and flag descriptions', () => {
  const keys = lang => [...readFileSync(new URL(`../../templates/admin/${lang}.ftl`, import.meta.url), 'utf8')
    .matchAll(/^(system-[\w-]+)\s*=\s*\S/gm)].map(match => match[1]).sort();
  expect(keys('pl')).toEqual(keys('en'));
  expect(keys('en')).toContain('system-title');
  expect(keys('en')).toContain('system-flag-oteryn-admin-panel');
});
