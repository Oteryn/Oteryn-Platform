import { expect, test } from '@playwright/test';
import { password, pool, register, seedPasswordRange } from '../helpers.mjs';

const editions = {
  pl: {
    note: 'Wersja robocza — dane operatora są tymczasowe.',
    titles: ['Regulamin Oteryn', 'Polityka prywatności', 'Polityka plików cookie'],
    terms: ['ustawieniach, możesz tam złożyć wniosek', 'poważnego lub uporczywego naruszenia', 'funkcji przekazywania monet w portalu'],
    breach: ['pięć pierwszych znaków szesnastkowych skrótu SHA-1', 'na 30 dni'],
  },
  en: {
    note: 'Draft — operator details are placeholders.',
    titles: ['Terms of Service', 'Privacy Policy', 'Cookie Policy'],
    terms: ['account settings when the feature is enabled', 'serious or persistent violations', 'portal’s coin transfer feature'],
    breach: ['first five hexadecimal characters of the password’s SHA-1 hash', 'for 30 days'],
  },
};

for (const [index, slug] of ['terms', 'privacy', 'cookies'].entries()) {
  test(`legal page shows the default ${slug} text`, async ({ page }, testInfo) => {
    const { locale } = testInfo.project.metadata;
    const { note, titles, terms, breach } = editions[locale];
    const response = await page.goto(`/${locale}/legal/${slug}`);

    expect(response?.status()).toBe(200);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(titles[index]);
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
  await seedPasswordRange(password());
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
      'SELECT document_kind, document_version FROM account_consents JOIN accounts ON accounts.id=account_consents.account_id WHERE accounts.email=$1',
      [account.email],
    );
    expect(Object.fromEntries(rows.map(row => [row.document_kind, row.document_version]))).toEqual(versions);
  } finally {
    await db.end();
  }
});
