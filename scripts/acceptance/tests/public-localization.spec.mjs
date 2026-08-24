import { test, expect } from '@playwright/test';
import {
  allowExpectedHttpFailure,
  assertAccessibilitySmoke,
  attachDiagnostics,
  installDiagnostics,
} from './helpers.mjs';

async function expectVisibleLink(page, name) {
  const links = page.getByRole('link', { name });
  for (let index = 0; index < await links.count(); index += 1) {
    if (await links.nth(index).isVisible()) {
      await expect(links.nth(index)).toBeVisible();
      return;
    }
  }

  const menu = page.getByText('Menu', { exact: true });
  if (await menu.isVisible()) await menu.click();

  for (let index = 0; index < await links.count(); index += 1) {
    if (await links.nth(index).isVisible()) {
      await expect(links.nth(index)).toBeVisible();
      return;
    }
  }

  throw new Error(`No visible link matched ${name}.`);
}

test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@localization canonical English and Polish public shells remain truthful and navigable', async ({ page }) => {
  const englishResponse = await page.goto('/en');
  expect(englishResponse?.status()).toBe(200);
  await expect(page.locator('html')).toHaveAttribute('lang', 'en');
  await expect(page.getByRole('heading', { name: 'Answer the call of Oteryn' })).toBeVisible();
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute('href', /\/en$/);
  await expect(page.locator('link[rel="alternate"][hreflang="pl"]')).toHaveAttribute('href', /\/pl$/);
  await expectVisibleLink(page, /Polski/i);
  await assertAccessibilitySmoke(page);

  const polishResponse = await page.goto('/pl');
  expect(polishResponse?.status()).toBe(200);
  await expect(page.locator('html')).toHaveAttribute('lang', 'pl');
  await expect(page.getByRole('heading', { name: 'Odpowiedz na wezwanie Oteryn' })).toBeVisible();
  await expectVisibleLink(page, 'Aktualności');
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute('href', /\/pl$/);
  await expectVisibleLink(page, /English/i);
  await assertAccessibilitySmoke(page);

  allowExpectedHttpFailure(page.__acceptanceDiagnostics, { status: 404, pathname: '/pl/brakujaca-strona' });
  const missingResponse = await page.goto('/pl/brakujaca-strona');
  expect(missingResponse?.status()).toBe(404);
  await expect(page.locator('html')).toHaveAttribute('lang', 'pl');
  await expect(page.getByRole('heading', { name: 'Nie udało się znaleźć tej strony' })).toBeVisible();
  await assertAccessibilitySmoke(page);
});
