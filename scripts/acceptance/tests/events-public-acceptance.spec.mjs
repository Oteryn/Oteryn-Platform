import { test, expect } from '@playwright/test';
import {
  assertAccessibilitySmoke,
  attachDiagnostics,
  evidenceScreenshot,
  installDiagnostics,
  runBinary,
} from './helpers.mjs';

function eventFixture(...args) {
  return JSON.parse(runBinary('php', ['scripts/acceptance/seed-browser-events.php', ...args]));
}

test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@portal-events public calendar, detail, locale isolation, empty and not-found states', async ({ page }) => {
  eventFixture('reset');

  let response = await page.goto('/events');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Events' })).toBeVisible();
  await expect(page.getByText('No events are available.')).toBeVisible();
  await assertAccessibilitySmoke(page);

  response = await page.goto('/pl/events');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Wydarzenia' })).toBeVisible();
  await expect(page.getByText('Brak dostępnych wydarzeń.')).toBeVisible();

  eventFixture('seed-public');

  response = await page.goto('/events');
  expect(response?.status()).toBe(200);
  for (const heading of ['Active now', 'Upcoming', 'Archived', 'Cancelled']) {
    await expect(page.getByRole('heading', { name: heading, exact: true })).toBeVisible();
  }
  for (const title of [
    'Acceptance Active Event',
    'Acceptance Upcoming Event',
    'Acceptance Archived Event',
    'Acceptance Cancelled Event',
  ]) {
    await expect(page.getByRole('link', { name: title })).toBeVisible();
  }
  await expect(page.getByText('Featured event')).toBeVisible();
  await assertAccessibilitySmoke(page);
  await evidenceScreenshot(page, `events-calendar-${test.info().project.name}`);

  await page.getByRole('link', { name: 'Acceptance Active Event' }).click();
  await expect(page).toHaveURL(/\/events\/acceptance-active-en$/u);
  await expect(page.getByRole('heading', { name: 'Acceptance Active Event' })).toBeVisible();
  await expect(page.getByText('Acceptance Active Event body paragraph one.')).toBeVisible();
  await expect(page.getByText('Acceptance Active Event body paragraph two.')).toBeVisible();
  await expect(page.getByRole('link', { name: 'Back to events' })).toBeVisible();

  response = await page.goto('/pl/events/acceptance-active-pl');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Aktywne wydarzenie akceptacyjne' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Wróć do wydarzeń' })).toBeVisible();

  response = await page.goto('/pl/events/acceptance-active-en');
  expect(response?.status()).toBe(404);
  await expect(page.locator('body')).not.toContainText('SQLSTATE');

  response = await page.goto('/events/acceptance-event-does-not-exist');
  expect(response?.status()).toBe(404);
  await expect(page.locator('body')).not.toContainText('SQLSTATE');
});
