import { readFile } from 'node:fs/promises';
import { test, expect } from '../fixtures.mjs';
import { action, query, seededStaff } from '../staff-rbac-helpers.mjs';

test('staff triages a grouped portal incident and sees it reopen on recurrence', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const labels = lang === 'pl'
    ? { title: 'Błędy portalu', resolved: 'Rozwiązany', ignored: 'Zignorowany', open: 'Otwarty', filter: 'Filtruj', skip: 'Przejdź do treści' }
    : { title: 'Portal errors', resolved: 'Resolved', ignored: 'Ignored', open: 'Open', filter: 'Filter', skip: 'Skip to content' };
  const guest = await page.request.get('/admin/errors', { maxRedirects: 0 });
  expect(guest.status()).toBe(303);
  expect(guest.headers().location).toBe('/login');
  const actor = await seededStaff(page, lang, 'platform_admin');
  await page.goto('/admin');
  await page.getByRole('link', { name: labels.title, exact: true }).click();
  await expect(page.getByRole('heading', { name: labels.title })).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  const skip = page.getByRole('link', { name: labels.skip });
  await page.keyboard.press('Tab');
  await expect(skip).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('main')).toBeFocused();
  expect((await page.request.get('/admin/errors?before=invalid')).status()).toBe(400);
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
    expect(scrape.headers()['cache-control']).toBe('no-store');
    expect(await scrape.text()).toContain(`portal_http_requests_total{route="/{lang}/atlas",status_class="5xx"}`);
  }

  await page.reload();
  await page.locator('select[name=status]').selectOption('open');
  await page.getByRole('button', { name: labels.filter, exact: true }).click();
  const detail = `/admin/errors/${first.id}`;
  await page.locator(`main a[href="${detail}"]`).click();
  await expect(page.locator('main [data-error-status]')).toHaveText(labels.open);
  const review = async (operation) => {
    const [response] = await Promise.all([
      page.waitForResponse((res) => res.request().method() === 'POST' &&
        new URL(res.url()).pathname === `${detail}/${operation}` && res.status() === 303),
      action(page, `form[action$="/${operation}"]`),
    ]);
    const id = response.headers()['x-request-id'];
    expect(id).toMatch(/^[a-f0-9-]{36}$/);
    return id;
  };
  const resolved = await review('resolve');
  await expect(page.locator('main [data-error-status]')).toHaveText(labels.resolved);

  await page.goto('/admin/errors?status=resolved');
  await page.locator(`main a[href="${detail}"]`).click();
  expect((await occurrence()).id).toBe(first.id);
  await page.reload();
  await expect(page.locator('main [data-error-status]')).toHaveText(labels.open);
  const ignored = await review('ignore');
  await expect(page.locator('main [data-error-status]')).toHaveText(labels.ignored);

  const audits = await query(
    'SELECT action, request_id FROM audit_log WHERE actor=$1 AND target=$2 ORDER BY created_at', [actor, first.id],
  );
  expect(audits).toEqual([
    { action: 'admin.errors.resolve', request_id: resolved },
    { action: 'admin.errors.ignore', request_id: ignored },
  ]);

  if (process.env.OTERYN_ALERT_EMAILS) {
    await expect.poll(async () => query(
      "SELECT delivered_at IS NOT NULL AS delivered FROM email_outbox WHERE category='operations' AND strpos(text_body, $1) > 0",
      [detail],
    ), { timeout: 25_000 }).toEqual([{ delivered: true }]);
  }
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);

  const prefix = `/fixture/${info.project.name}/`;
  const inserted = await query(
    `INSERT INTO portal_errors(route, error_kind, last_request_id, alert_enqueued_at)
     SELECT $1 || n, 'http_500', uuidv7(), statement_timestamp() FROM generate_series(1, 31) n
     RETURNING id`, [prefix],
  );
  await page.goto(`/admin/errors?${new URLSearchParams({ route: prefix, status: 'open' })}`);
  const rows = page.locator('main article h2 a');
  await expect(rows).toHaveCount(30);
  const visible = await rows.evaluateAll((links) => links.map((link) => link.getAttribute('href')));
  await page.locator('.op-pagination a').click();
  await expect(rows).toHaveCount(1);
  await expect(page.locator('input[name=route]')).toHaveValue(prefix);
  await expect(page.locator('select[name=status]')).toHaveValue('open');
  await expect(page.locator('.op-pagination a')).toHaveCount(0);
  expect([...visible, await rows.getAttribute('href')].sort()).toEqual(
    inserted.map(({ id }) => `/admin/errors/${id}`).sort(),
  );
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
  expect(pl).toEqual(expect.arrayContaining([
    ...keys, 'errors-open', 'errors-resolved', 'errors-ignored',
    'errors-alert-group', 'errors-alert-spike', 'errors-alert-count', 'errors-alert-threshold',
  ]));
});
