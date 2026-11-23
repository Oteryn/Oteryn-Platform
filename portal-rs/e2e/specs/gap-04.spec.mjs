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
    const signup = await visitor.newPage();
    for (const kind of Object.keys(paths)) {
      await signup.goto(`/${lang}/register`);
      await expect(signup.locator(`input[name=${kind}]`)).toHaveValue(versions[kind]);
      await signup.locator(`main a[href="/${lang}/legal/${kind}"]`).click();
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
