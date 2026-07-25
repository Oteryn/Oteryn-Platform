import { test, expect } from '@playwright/test';
import {
  assertAccessibilitySmoke,
  attachDiagnostics,
  installDiagnostics,
} from './helpers.mjs';

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
  await expect(page.locator('.language-switcher').first().getByRole('link', { name: /Polski/i })).toBeVisible();
  await assertAccessibilitySmoke(page);

  const polishResponse = await page.goto('/pl');
  expect(polishResponse?.status()).toBe(200);
  await expect(page.locator('html')).toHaveAttribute('lang', 'pl');
  await expect(page.getByRole('heading', { name: 'Odpowiedz na wezwanie Oteryn' })).toBeVisible();
  await expect(page.getByRole('navigation', { name: 'Nawigacja publiczna' })).toContainText('Aktualności');
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute('href', /\/pl$/);
  await expect(page.locator('.language-switcher').first().getByRole('link', { name: /English/i })).toBeVisible();
  await assertAccessibilitySmoke(page);

  const missingResponse = await page.goto('/pl/brakujaca-strona');
  expect(missingResponse?.status()).toBe(404);
  await expect(page.locator('html')).toHaveAttribute('lang', 'pl');
  await expect(page.getByRole('heading', { name: 'Nie udało się znaleźć tej strony' })).toBeVisible();
  await assertAccessibilitySmoke(page);
});
