import { expect, test } from '@playwright/test';
import { pool, register, seedPasswordRange } from '../helpers.mjs';

const editions = {
  pl: {
    draftNotice: 'Wersja robocza — dane operatora zostaną zastąpione.',
    note: 'Wersja robocza — dane operatora są tymczasowe.',
    titles: ['Regulamin Oteryn', 'Polityka prywatności', 'Polityka plików cookie'],
    terms: ['ustawieniach, możesz tam złożyć wniosek', 'poważnego lub uporczywego naruszenia', 'funkcji przekazywania monet w portalu'],
    breach: ['pięć pierwszych znaków szesnastkowych skrótu SHA-1', 'na 30 dni'],
  },
  en: {
    draftNotice: 'Draft — operator data will be replaced.',
    note: 'Draft — operator details are placeholders.',
    titles: ['Terms of Service', 'Privacy Policy', 'Cookie Policy'],
    terms: ['account settings when the feature is enabled', 'serious or persistent violations', 'portal’s coin transfer feature'],
    breach: ['first five hexadecimal characters of the password’s SHA-1 hash', 'for 30 days'],
  },
};

for (const [index, slug] of ['terms', 'privacy', 'cookies'].entries()) {
  test(`legal page shows the default ${slug} text`, async ({ page }, testInfo) => {
    const { locale } = testInfo.project.metadata;
    const { draftNotice, note, titles, terms, breach } = editions[locale];
    const response = await page.goto(`/${locale}/legal/${slug}`);

    expect(response?.status()).toBe(200);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(titles[index]);
    await expect(page.getByRole('note').getByText(draftNotice, { exact: true })).toBeVisible();
    const article = page.locator('main article');
    await expect(article.getByText(note, { exact: true })).toBeVisible();
    await expect(article.getByRole('heading', { level: 2 }).first()).toBeVisible();
    await expect(article).toContainText('0000000000');
    if (slug === 'terms') {
      for (const clause of terms) await expect(article).toContainText(clause);
    }
    if (slug === 'privacy') {
      for (const clause of ['Have I Been Pwned', ...breach]) await expect(article).toContainText(clause);
    }
  });
}

test('signup records the versions shown on the linked legal pages', async ({ page }, testInfo) => {
  const lang = testInfo.project.metadata.locale;
  await seedPasswordRange('registration');
  const versions = {};
  for (const kind of ['terms', 'privacy']) {
    const path = `/${lang}/legal/${kind}`;
    await page.goto(`/${lang}/register`);
    versions[kind] = await page.locator(`input[name="${kind}"]`).inputValue();
    await page.locator(`main a[href="${path}"]`).click();
    await expect(page).toHaveURL(new RegExp(path + '$'));
    await expect(page.locator('main > p').first()).toHaveText(`${versions[kind]} 2026-11-22`);
  }
  const account = await register(page, lang);
  const db = pool();
  try {
    const { rows } = await db.query(
      `SELECT c.document_kind, c.document_version, v.locale, p.slug
      FROM account_consents c JOIN accounts a ON a.id=c.account_id
      JOIN page_legal_versions v
        ON (v.page_id,v.locale,v.version)=(c.legal_page_id,c.legal_locale,c.document_version)
      JOIN pages p ON p.id=v.page_id WHERE a.email=$1 ORDER BY c.document_kind`,
      [account.email],
    );
    expect(rows).toEqual([
      { document_kind: 'privacy', document_version: versions.privacy, locale: lang, slug: 'privacy-policy' },
      { document_kind: 'terms', document_version: versions.terms, locale: lang, slug: 'terms-of-service' },
    ]);
  } finally {
    await db.end();
  }
});
