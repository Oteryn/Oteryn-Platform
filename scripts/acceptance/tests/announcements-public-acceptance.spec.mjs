import { test, expect } from '@playwright/test';
import {
  assertAccessibilitySmoke,
  attachDiagnostics,
  evidenceScreenshot,
  installDiagnostics,
  runBinary,
} from './helpers.mjs';

function announcementFixture(...args) {
  return JSON.parse(runBinary('php', ['scripts/acceptance/seed-browser-announcements.php', ...args]));
}

function restoreSharedHomepageFixture() {
  runBinary('php', ['scripts/acceptance/seed-homepage-navigation-seo.php']);
}

async function assertNoPageOverflow(page) {
  const dimensions = await page.evaluate(() => ({
    viewport: window.innerWidth,
    document: document.documentElement.scrollWidth,
  }));

  expect(dimensions.document, `Unexpected page overflow on ${page.url()}`).toBeLessThanOrEqual(dimensions.viewport + 1);
}

test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  try {
    await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
  } finally {
    restoreSharedHomepageFixture();
  }
});

test('@portal-announcements public none-active, active-window, expired, escaped and localized states', async ({ page }) => {
  announcementFixture('reset');

  let response = await page.goto('/en');
  expect(response?.status()).toBe(200);
  const englishTicker = page.locator('[data-content-state]').filter({ has: page.getByRole('heading', { name: 'Announcements' }) });
  await expect(englishTicker.getByText('No active announcements.')).toBeVisible();
  await assertAccessibilitySmoke(page);
  await assertNoPageOverflow(page);

  response = await page.goto('/pl');
  expect(response?.status()).toBe(200);
  const polishEmptyTicker = page.locator('[data-content-state]').filter({ has: page.getByRole('heading', { name: 'Ogłoszenia' }) });
  await expect(polishEmptyTicker.getByText('Brak aktywnych ogłoszeń.')).toBeVisible();

  announcementFixture('seed-public');

  response = await page.goto('/en');
  expect(response?.status()).toBe(200);
  const ticker = page.locator('[data-content-state="available"]').filter({ has: page.getByRole('heading', { name: 'Announcements' }) });
  await expect(ticker.getByRole('heading', { name: 'Acceptance Active Notice' })).toBeVisible();
  await expect(ticker).toContainText('<img src=x onerror=alert("announcement")> Plain-text maintenance details.');
  await expect(ticker.locator('img')).toHaveCount(0);
  await expect(ticker.getByText('Acceptance Expired Notice')).toHaveCount(0);
  await expect(ticker.getByText('Acceptance Future Notice')).toHaveCount(0);
  await expect(ticker.getByText('Acceptance Draft Notice')).toHaveCount(0);
  await expect(ticker.getByRole('link', { name: 'Read maintenance details' })).toHaveAttribute('href', '/en/news');
  await assertAccessibilitySmoke(page);
  await assertNoPageOverflow(page);
  await evidenceScreenshot(page, `announcements-public-en-${test.info().project.name}`);

  response = await page.goto('/pl');
  expect(response?.status()).toBe(200);
  const polishTicker = page.locator('[data-content-state="available"]').filter({ has: page.getByRole('heading', { name: 'Ogłoszenia' }) });
  await expect(polishTicker.getByRole('heading', { name: 'Aktywny komunikat akceptacyjny' })).toBeVisible();
  await expect(polishTicker.getByText('Polskie informacje o przerwie technicznej.')).toBeVisible();
  await expect(polishTicker.getByRole('link', { name: 'Przeczytaj szczegóły' })).toBeVisible();
  await expect(polishTicker.getByText('Acceptance Active Notice')).toHaveCount(0);
  await assertAccessibilitySmoke(page);
  await assertNoPageOverflow(page);
  await evidenceScreenshot(page, `announcements-public-pl-${test.info().project.name}`);

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});
