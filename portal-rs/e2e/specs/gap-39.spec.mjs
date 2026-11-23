import { readFileSync } from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { submit } from '../helpers.mjs';
import { role, seededStaff, query } from '../staff-rbac-helpers.mjs';

const copy = {
  pl: {
    audit: 'Audyt', target: 'Cel', view: 'Wyświetl', invalid: 'Sprawdź wartości w formularzu.',
    denied: 'Brak dostępu',
    auditCount: '1 wpis audytu', news: 'Aktualności',
    filter: 'Filtruj aktualności', month: 'Miesiąc (1–12)', tag: 'Tag',
    invalidMonth: 'Wpisz miesiąc od 1 do 12.', newsCount: '2 wpisy na tej stronie',
    search: 'Szukaj w historii zmian',
    date: '10.02.2022',
  },
  en: {
    audit: 'Audit', target: 'Target', view: 'View', invalid: 'Check the values in the form.',
    denied: 'Access denied',
    auditCount: '1 audit entry', news: 'News',
    filter: 'Filter news', month: 'Month (1–12)', tag: 'Tag',
    invalidMonth: 'Enter a month from 1 to 12.', newsCount: '2 entries on this page',
    search: 'Search changelog',
    date: '2022-02-10',
  },
};

test('staff audit and public news forms use the selected locale through the result', async ({ page, browser }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const t = copy[lang];
  const audit = '/admin/console/audit';
  await seededStaff(page, lang, 'platform_admin');
  const id = await role(page);
  const [entry] = await query(
    "SELECT created_at FROM audit_log WHERE action='rbac.role.created' AND target=$1", [id],
  );

  await page.goto('/admin');
  await page.getByRole('link', { name: t.audit, exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await page.getByLabel(t.target, { exact: true }).fill(id);
  await page.getByRole('button', { name: t.view, exact: true }).click();
  await expect(page.getByText(t.auditCount, { exact: true })).toBeVisible();
  await expect(page.getByLabel(t.target, { exact: true })).toHaveValue(id);
  await expect(page.locator('main')).toContainText('rbac.role.created');
  const stamp = entry.created_at.toISOString().slice(0, 19);
  const [day, time] = stamp.split('T');
  await expect(page.locator('main time')).toHaveText(
    (lang === 'pl' ? day.split('-').reverse().join('.') : day) + ' ' + time + ' UTC',
  );
  expect(await page.evaluate(() => document.documentElement.scrollWidth))
    .toBeLessThanOrEqual(info.project.use.viewport.width);

  const opposite = lang === 'pl' ? 'en' : 'pl';
  const bad = await page.goto(`${audit}?lang=${lang}&admin=invalid`);
  expect(bad.status()).toBe(400);
  await expect(page.getByRole('alert')).toHaveText(t.invalid);
  await expect(page.getByRole('heading', { level: 1, name: t.invalid })).toBeVisible();
  const context = await browser.newContext({
    baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443',
    ignoreHTTPSErrors: true,
    locale: opposite,
    storageState: await page.context().storageState(),
  });
  try {
    const check = await context.newPage();
    const response = await check.goto(`${audit}?lang=${lang}&admin=invalid`);
    expect(response.headers()['content-language']).toBe(lang);
    await expect(check.getByRole('alert')).toHaveText(t.invalid);
    await expect(check.locator('header a.om-brand')).toHaveAttribute('href', `/${lang}/`);
  } finally {
    await context.close();
  }

  const marker = `locale-${crypto.randomUUID()}`;
  const news = `/${lang}/news`;
  const edits = [];
  try {
    for (const suffix of ['a', 'b']) {
      await page.goto('/admin/news/create');
      await submit(page, 'main form', {
        slug: `${marker}-${suffix}`, title: `News ${suffix} ${marker}`,
        title_pl: `Aktualności ${suffix} ${marker}`, body: 'Details', body_pl: 'Szczegóły',
        published_at: '2022-02-10T12:00:00Z', translation_published_at: '2022-02-10T12:00:00Z',
        tags: marker,
      });
      await expect(page).toHaveURL(/\/admin\/news\/[\da-f-]+\/edit/);
      edits.push(new URL(page.url()).pathname);
    }
    await page.goto(`/${lang}/`);
    await page.locator(`main a[href="${news}"]`).first().click();
    await expect(page.getByRole('heading', { name: t.news, exact: true })).toBeVisible();
    await page.getByLabel(t.tag, { exact: true }).fill(marker);
    await page.getByRole('button', { name: t.filter }).click();
    await expect(page.getByText(t.newsCount, { exact: true })).toBeVisible();
    await expect(page.locator('main article time').first()).toHaveText(t.date);
    await expect(page.getByRole('link', { name: `${t.news} a ${marker}`, exact: true })).toBeVisible();
    expect((await page.goto(`${news}?month=13`)).status()).toBe(422);
    await expect(page.getByRole('alert')).toHaveText(t.invalidMonth);
    await expect(page.getByLabel(t.month, { exact: true })).toHaveValue('13');
    await page.locator(`main nav a[href="/${lang}/changelog"]`).click();
    await expect(page.getByRole('searchbox', { name: t.search })).toBeVisible();
  } finally {
    for (const edit of edits.reverse()) {
      await page.goto(edit);
      await submit(page, 'main form[action$="/delete"]');
      await expect(page).toHaveURL(/\/admin\/news$/);
    }
  }
});

test('staff panel audit navigation follows the current role', async ({ page }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const t = copy[lang];
  const actor = await seededStaff(page, lang, 'game_viewer');
  const auditLink = page.getByRole('link', { name: t.audit, exact: true });
  expect((await page.goto('/admin')).status()).toBe(200);
  await expect(auditLink).toHaveAttribute('href', `/admin/console/audit?lang=${lang}`);
  await auditLink.click();
  await expect(page.getByRole('heading', { level: 1, name: t.audit })).toBeVisible();

  await query("UPDATE account_roles SET role_id=(SELECT id FROM roles WHERE key='support') WHERE account_id=$1", [actor]);
  expect((await page.goto('/admin')).status()).toBe(200);
  await expect(auditLink).toHaveCount(0);
  const refused = await page.goto(`/admin/console/audit?lang=${lang}`);
  expect(refused.status()).toBe(403);
  expect(refused.headers()['content-language']).toBe(lang);
  await expect(page.getByRole('alert')).toHaveText(t.denied);
});

test('locale negotiation, fallback and refusal work independently of key parity', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const other = lang === 'pl' ? 'en' : 'pl';
  const get = (url, headers = {}) => page.request.get(url, { headers, maxRedirects: 0 });
  const negotiated = await get('/', { 'accept-language': `${other};q=0.2, ${lang}-${lang.toUpperCase()};q=0.8` });
  expect(negotiated.headers().location).toBe(`/${lang}/`);
  const fallback = await get('/', { 'accept-language': 'fr, en;q=0', cookie: 'portal_language=fr' });
  expect(fallback.headers().location).toBe('/pl/');
  const archive = await get('/news', { 'accept-language': `${other};q=0.2, ${lang};q=0.8` });
  expect(archive.status()).toBe(200);
  expect(archive.headers()['content-language']).toBe(lang);
  const legacy = await get('/stats/world', { 'accept-language': 'en' });
  expect(legacy.status()).toBe(200);
  expect(legacy.headers()['content-language']).toBe('pl');
  expect(await legacy.text()).toContain('href="/pl/"');
  const invalidArchive = await get('/news?lang=fr', { 'accept-language': lang });
  expect(invalidArchive.status()).toBe(400);
  expect(invalidArchive.headers().location).toBeUndefined();
  const switched = await get(`/language/${other}`);
  expect(switched.headers().location).toBe(`/${other}/`);
  expect((await get('/', { 'accept-language': lang })).headers().location).toBe(`/${other}/`);
  const refused = await get('/language/fr');
  expect(refused.status()).toBe(400);
  expect(refused.headers().location).toBeUndefined();
  expect(refused.headers()['set-cookie']).toBeUndefined();
});

test('i18n-parity for the shared shell, staff console and public archive', () => {
  for (const [pattern, required] of [
    ['crates/ui/i18n/{lang}.ftl', ['error-400-title', 'error-403-title', 'error-422-title']],
    ['crates/admin/src/console/access.{lang}.ftl', ['audit', 'target', 'submit', 'audit-count', 'language', 'skip']],
    ['crates/admin/src/console/{lang}.ftl', ['unavailable', 'fresh', 'seconds-ago', 'invalid-form', 'access-denied']],
    ['templates/admin/{lang}.ftl', ['audit-link']],
    ['templates/content/{lang}.ftl', [
      'news', 'month', 'archive-count', 'invalid-year', 'invalid-month', 'invalid-tag',
      'invalid-search', 'invalid-page', 'invalid-archive-query', 'archive-unavailable',
    ]],
  ]) {
    const keys = lang => [...readFileSync(new URL(`../../${pattern.replace('{lang}', lang)}`, import.meta.url), 'utf8')
      .matchAll(/^([a-zA-Z][\w-]*)\s*=/gm)].map(([, key]) => key).sort();
    const en = keys('en');
    expect(new Set(en).size, pattern).toBe(en.length);
    expect(keys('pl'), pattern).toEqual(en);
    expect(en, pattern).toEqual(expect.arrayContaining(required));
  }
});
