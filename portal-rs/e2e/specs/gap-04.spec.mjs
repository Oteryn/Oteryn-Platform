import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { register } from '../helpers.mjs';
import { query, seededStaff } from '../staff-rbac-helpers.mjs';

const labels = {
  en: {
    pages: 'Pages',
    readiness: 'Live signup legal readiness',
    blocked: 'Live signup is blocked by the legal editions below.',
    draft: 'Replace the draft operator details and publish a final edition.',
  },
  pl: {
    pages: 'Strony',
    readiness: 'Gotowość dokumentów do rejestracji produkcyjnej',
    blocked: 'Poniższe dokumenty blokują rejestrację produkcyjną.',
    draft: 'Zastąp tymczasowe dane operatora i opublikuj wersję ostateczną.',
  },
};

test('i18n-parity for signup legal readiness and pause messages', () => {
  const read = path => readFileSync(new URL(path, import.meta.url), 'utf8');
  const keys = (locale, path, pattern) => [...read(path.replace('{lang}', locale))
    .matchAll(/^([a-z][a-z0-9-]*)[ \t]*=[ \t]*\S/gm)]
    .map(([, key]) => key).filter(key => pattern.test(key)).sort();
  const check = (catalogue, pattern, sources) => {
    const expected = new Set();
    for (const path of sources) {
      const literals = path.endsWith('.html')
        ? /self\.text\("([a-z][a-z0-9-]*)"\)/g
        : /"([a-z][a-z0-9-]*)"/g;
      for (const [, key] of read(path).matchAll(literals)) {
        if (pattern.test(key)) expected.add(key);
      }
    }
    expect(expected.size).toBeGreaterThan(0);
    for (const lang of ['en', 'pl']) {
      expect(keys(lang, catalogue, pattern)).toEqual([...expected].sort());
    }
  };
  check('../../templates/pages/{lang}.ftl', /^signup-/, [
    '../../crates/content/src/pages/staff.rs',
    '../../templates/admin/pages/index.html',
  ]);
  check('../../crates/ui/i18n/{lang}.ftl', /^register-paused-/, [
    '../../templates/account/register_paused.html',
  ]);
});

test('staff inspect live signup readiness and a test signup records the displayed editions', async (
  { page, browser },
  { project },
) => {
  const lang = project.metadata.locale;
  const words = labels[lang];
  await seededStaff(page, lang, 'content_editor');
  await page.goto('/admin');
  await page.locator('main nav').getByRole('link', { name: words.pages, exact: true }).click();
  await expect(page).toHaveURL(/\/admin\/pages$/);

  const readiness = page.getByRole('region', { name: words.readiness });
  await expect(readiness.getByText(words.blocked)).toBeVisible();
  await expect(readiness.getByText(words.draft, { exact: true })).toHaveCount(4);
  expect(await page.evaluate(() => document.documentElement.scrollWidth))
    .toBeLessThanOrEqual(project.use.viewport.width);

  const paths = { terms: 'terms-of-service', privacy: 'privacy-policy' };
  const versions = {};
  for (const [kind, slug] of Object.entries(paths)) {
    for (const locale of ['en', 'pl']) {
      const link = readiness.locator(`a[href="/admin/pages/${slug}/${locale}"]`);
      await expect(link).toBeVisible();
    }
    await readiness.locator(`a[href="/admin/pages/${slug}/${lang}"]`).click();
    versions[kind] = await page.locator('input[name=legal_version]').inputValue();
    await page.goBack();
  }

  const visitor = await browser.newContext({
    baseURL: project.use.baseURL ?? new URL(page.url()).origin,
    ignoreHTTPSErrors: true,
    locale: lang,
    viewport: project.use.viewport,
  });
  try {
    expect(await query(
      'UPDATE registration_abuse_policy SET enabled=true WHERE singleton=true RETURNING enabled',
    )).toEqual([{ enabled: true }]);
    const signup = await visitor.newPage();
    for (const kind of Object.keys(paths)) {
      expect((await signup.goto(`/${lang}/register`)).status()).toBe(200);
      await expect(signup.locator(`input[name=${kind}]`)).toHaveValue(versions[kind]);
      const [policy] = await Promise.all([
        signup.waitForResponse(response => new URL(response.url()).pathname === `/${lang}/legal/${kind}`),
        signup.locator(`main a[href="/${lang}/legal/${kind}"]`).click(),
      ]);
      expect(policy.status()).toBe(200);
      await expect(signup.locator('main > p').first()).toContainText(versions[kind]);
      await expect(signup.getByRole('note')).toBeVisible();
    }
    const account = await register(signup, lang);
    const records = await query(
      `SELECT c.document_kind, c.document_version, v.locale
       FROM account_consents c JOIN accounts a ON a.id=c.account_id
       JOIN page_legal_versions v
         ON (v.page_id,v.locale,v.version)=(c.legal_page_id,c.legal_locale,c.document_version)
       WHERE a.email=$1 ORDER BY c.document_kind`,
      [account.email],
    );
    expect(records).toEqual(['privacy', 'terms'].map(kind => ({
      document_kind: kind,
      document_version: versions[kind],
      locale: lang,
    })));
  } finally {
    await visitor.close();
  }
});
