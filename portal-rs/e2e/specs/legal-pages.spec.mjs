import { expect, test } from '@playwright/test';

const editions = {
  pl: {
    note: 'Wersja robocza — dane operatora są tymczasowe.',
    titles: ['Regulamin Oteryn', 'Polityka prywatności', 'Polityka plików cookie'],
  },
  en: {
    note: 'Draft — operator details are placeholders.',
    titles: ['Terms of Service', 'Privacy Policy', 'Cookie Policy'],
  },
};

for (const [index, slug] of ['terms', 'privacy', 'cookies'].entries()) {
  test(`legal page shows the default ${slug} text`, async ({ page }, testInfo) => {
    const { locale } = testInfo.project.metadata;
    const { note, titles } = editions[locale];
    const response = await page.goto(`/${locale}/legal/${slug}`);

    expect(response?.status()).toBe(200);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(titles[index]);
    const article = page.locator('main article');
    await expect(article.getByText(note, { exact: true })).toBeVisible();
    await expect(article.getByRole('heading', { level: 2 }).first()).toBeVisible();
    await expect(article).toContainText('0000000000');
  });
}
