import { test, expect } from '@playwright/test';
import {
  assertAccessibilitySmoke,
  attachDiagnostics,
  installDiagnostics,
  runBinary,
} from './helpers.mjs';

function seedPublicWiki() {
  runBinary('php', ['scripts/acceptance/seed-public-wiki.php']);
}

test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  seedPublicWiki();
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@wiki published bilingual reads search table of contents and keyboard flow remain portable and responsive', async ({ page }) => {
  const homeResponse = await page.goto('/en/wiki');
  expect(homeResponse?.status()).toBe(200);
  await expect(page.getByRole('heading', { level: 1, name: 'Wiki' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Getting Started' })).toBeVisible();
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute('href', /\/en\/wiki$/u);
  await assertAccessibilitySmoke(page);

  const searchInput = page.getByLabel('Search published Wiki articles');
  await searchInput.focus();
  await expect(searchInput).toBeFocused();
  await searchInput.fill('first login');
  await page.getByRole('button', { name: 'Search' }).click();
  await expect(page).toHaveURL(/\/en\/wiki\/search\?q=first(\+|%20)login$/u);
  await expect(page.getByRole('link', { name: 'First login' })).toBeVisible();
  await assertAccessibilitySmoke(page);

  await page.getByRole('link', { name: 'First login' }).click();
  await expect(page.getByRole('heading', { level: 1, name: 'First login' })).toBeVisible();
  await expect(page.getByRole('complementary').getByRole('link', { name: 'Install the client' })).toHaveAttribute('href', '#install-the-client');
  await expect(page.locator('.wiki-table-scroll')).toBeVisible();
  await expect(page.locator('link[rel="alternate"][hreflang="pl"]')).toHaveAttribute('href', /\/pl\/wiki\/pierwsze-logowanie$/u);
  await assertAccessibilitySmoke(page);

  const polishResponse = await page.goto('/pl/wiki/pierwsze-logowanie');
  expect(polishResponse?.status()).toBe(200);
  await expect(page.locator('html')).toHaveAttribute('lang', 'pl');
  await expect(page.getByRole('heading', { level: 1, name: 'Pierwsze logowanie' })).toBeVisible();
  await expect(page.getByText('First login', { exact: true })).toHaveCount(0);
  await assertAccessibilitySmoke(page);
});
