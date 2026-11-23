import { expect, localized, test, visit } from './fixtures.mjs';
import { requestTo } from './transport.mjs';

const titles = {
  pl: {
    home: 'Aktualności', terms: 'Regulamin Oteryn', privacy: 'Polityka prywatności',
    cookies: 'Polityka plików cookie', missing: 'Nie znaleziono strony',
    rankings: 'Rankingi', rankingsMissing: 'Nie znaleziono strony.',
    bazaar: 'Aukcje postaci są niedostępne',
    creation: 'Tworzenie postaci jest obecnie niedostępne.',
    shop: 'Sklep Oterynu',
    shopStatus: 'Sklep jest obecnie zamknięty. Zakupy i doładowania są chwilowo niedostępne.',
  },
  en: {
    home: 'News', terms: 'Terms of Service', privacy: 'Privacy Policy',
    cookies: 'Cookie Policy', missing: 'Page not found',
    rankings: 'Rankings', rankingsMissing: 'Page not found.',
    bazaar: 'Character auctions unavailable',
    creation: 'Character creation is currently unavailable.',
    shop: 'Oteryn Shop',
    shopStatus: 'The shop is currently closed. Purchases and coin top-ups are temporarily unavailable.',
  },
};

test('6.1 Home, news and wiki display the localized public pages', async ({ page, lang }) => {
  for (const [path, title] of [['', titles[lang].home], ['news', titles[lang].home], ['wiki', 'Wiki']]) {
    const route = `/${lang}/${path}`;
    await test.step(`GET ${route}`, async () => {
      await visit(page, route);
      await localized(page, lang, title);
    });
  }
});

for (const [index, path] of ['terms', 'privacy', 'cookies'].entries()) {
  test(`6.${index + 2} Legal ${path} page loads in the requested language`, async ({ page, lang }) => {
    await visit(page, `/${lang}/legal/${path}`);
    await localized(page, lang, titles[lang][path]);
    await expect(page.locator('main article').getByRole('heading', { level: 2 }).first()).toBeVisible();
  });
}

test('6.5 Highscores alias resolves to the documented disabled Game ranking', async ({ page, lang, request }) => {
  const alias = await requestTo(request, 'GET', '/highscores');
  expect(alias.status(), 'GET /highscores').toBe(301);
  expect(alias.headers().location, 'GET /highscores Location').toBe('/pl/rankings/level');
  await visit(page, `/${lang}/rankings/level`, 404);
  await localized(page, lang, titles[lang].rankings);
  await expect(page.locator('main [role=status]')).toHaveText(titles[lang].rankingsMissing);
});

test('6.6 An unknown public route shows the localized 404 page', async ({ page, lang, account }) => {
  await visit(page, `/${lang}/${account.email.split('@')[0]}`, 404);
  await localized(page, lang, titles[lang].missing);
});

test('6.7 Game and Store expose their disabled staging states', async ({ page, lang }) => {
  await visit(page, `/${lang}/`);
  await expect(page.getByText(titles[lang].creation, { exact: true })).toBeVisible();
  const shopPreview = `/static/thais/shop-${lang}.html`;
  await expect(page.locator('header a.om-shop')).toHaveAttribute('href', shopPreview);
  await visit(page, shopPreview);
  await localized(page, lang, titles[lang].shop);
  await expect(page.locator('.shop-status')).toHaveText(titles[lang].shopStatus);
  await visit(page, `/${lang}/bazaar`, 503);
  await localized(page, lang, titles[lang].bazaar);
  for (const path of ['shop', 'shop/coins', 'shop/history']) {
    await visit(page, `/${lang}/${path}`, 404);
    await localized(page, lang, titles[lang].missing);
  }
});

test('6.8 Payments checkout is unavailable on staging', async ({ page }) => {
  const path = '/native-account/coin-checkout/01934f10-7c02-4001-805b-3b1122334401';
  const response = await visit(page, path, 503);
  expect(response.headers()['cache-control'], `GET ${path} cache policy`).toContain('no-store');
});
