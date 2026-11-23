import { readFile } from 'node:fs/promises';
import { test, expect } from '../fixtures.mjs';
import { action, query, seededStaff } from '../staff-rbac-helpers.mjs';

test('staff triages a grouped portal incident and sees it reopen on recurrence', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const labels = lang === 'pl'
    ? { title: 'Błędy portalu', resolved: 'Rozwiązany', ignored: 'Zignorowany', open: 'Otwarty', filter: 'Filtruj' }
    : { title: 'Portal errors', resolved: 'Resolved', ignored: 'Ignored', open: 'Open', filter: 'Filter' };
  const actor = await seededStaff(page, lang, 'platform_admin');
  await page.goto('/admin');
  await page.getByRole('link', { name: labels.title, exact: true }).click();
  await expect(page.getByRole('heading', { name: labels.title })).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  expect((await page.request.get('/metrics')).status()).toBe(404);

  // The cold Atlas projection returns a real service-unavailable response.
  const route = `/${lang}/atlas`;
  const occurrence = async () => {
    const response = await page.request.get(route);
    expect(response.status()).toBe(503);
    const requestId = response.headers()['x-request-id'];
    expect(requestId).toMatch(/^[a-f0-9-]{36}$/);
    return (await query(
      'SELECT id, route, occurrence_count FROM portal_errors WHERE last_request_id = $1', [requestId],
    ))[0];
  };
  const first = await occurrence();
  const second = await occurrence();
  expect(first.route).toBe('/{lang}/atlas');
  expect(second.id).toBe(first.id);
  expect(Number(second.occurrence_count)).toBe(Number(first.occurrence_count) + 1);
  if (process.env.OTERYN_METRICS_TOKEN) {
    const scrape = await page.request.get('/metrics', {
      headers: { authorization: `Bearer ${process.env.OTERYN_METRICS_TOKEN}` },
    });
    expect(scrape.status()).toBe(200);
    expect(await scrape.text()).toContain(`portal_http_requests_total{route="/{lang}/atlas",status_class="5xx"}`);
  }

  await page.reload();
  await page.locator('select[name=status]').selectOption('open');
  await page.getByRole('button', { name: labels.filter, exact: true }).click();
  const detail = `/admin/errors/${first.id}`;
  await page.locator(`main a[href="${detail}"]`).click();
  await expect(page.locator('main [data-error-status]')).toHaveText(labels.open);
  await action(page, 'form[action$="/resolve"]');
  await expect(page.locator('main [data-error-status]')).toHaveText(labels.resolved);

  await page.goto('/admin/errors?status=resolved');
  await page.locator(`main a[href="${detail}"]`).click();
  expect((await occurrence()).id).toBe(first.id);
  await page.reload();
  await expect(page.locator('main [data-error-status]')).toHaveText(labels.open);
  await action(page, 'form[action$="/ignore"]');
  await expect(page.locator('main [data-error-status]')).toHaveText(labels.ignored);

  const audits = await query(
    'SELECT action FROM audit_log WHERE actor=$1 AND target=$2 ORDER BY created_at', [actor, first.id],
  );
  expect(audits).toEqual([{ action: 'admin.errors.resolve' }, { action: 'admin.errors.ignore' }]);

  if (process.env.OTERYN_ALERT_EMAILS) {
    await expect.poll(async () => query(
      "SELECT delivered_at IS NOT NULL AS delivered FROM email_outbox WHERE category='operations' AND strpos(text_body, $1) > 0",
      [detail],
    ), { timeout: 25_000 }).toEqual([{ delivered: true }]);
  }
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
});

test('i18n-parity covers every incident review label', async () => {
  const labels = async (lang) => {
    const translations = await readFile(new URL(`../../templates/admin/${lang}.ftl`, import.meta.url), 'utf8');
    return [...translations.matchAll(/^(errors-[a-z-]+)\s*=\s*(\S.*)$/gm)].map((match) => match[1]).sort();
  };
  const pl = await labels('pl');
  const en = await labels('en');
  expect(pl).toEqual(en);
  expect(new Set(pl).size).toBe(pl.length);
  const templates = await Promise.all(['list', 'detail'].map((name) =>
    readFile(new URL(`../../templates/admin/errors/${name}.html`, import.meta.url), 'utf8')));
  const keys = [...new Set(templates.flatMap((html) =>
    [...html.matchAll(/self\.text\("(errors-[a-z-]+)"\)/g)].map((match) => match[1])))];
  expect(pl).toEqual(expect.arrayContaining([...keys, 'errors-open', 'errors-resolved', 'errors-ignored']));
});
