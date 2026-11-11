import { test, expect } from '@playwright/test';
import {
  assertAccessibilitySmoke,
  attachDiagnostics,
  installDiagnostics,
  runBinary,
} from './helpers.mjs';

const responsiveViewports = [
  { width: 1440, height: 1000 },
  { width: 820, height: 1180 },
  { width: 390, height: 844 },
];

function seedGameCatalog() {
  return JSON.parse(runBinary('php', ['scripts/acceptance/seed-game-catalog.php']));
}

async function assertNoHorizontalOverflow(page) {
  const dimensions = await page.evaluate(() => ({
    viewport: document.documentElement.clientWidth,
    content: document.documentElement.scrollWidth,
  }));
  expect(dimensions.content).toBeLessThanOrEqual(dimensions.viewport + 1);
}

async function assertResponsiveLayout(page) {
  for (const viewport of responsiveViewports) {
    await page.setViewportSize(viewport);
    await expect(page.locator('main')).toBeVisible();
    await assertNoHorizontalOverflow(page);
  }

  await page.setViewportSize(responsiveViewports[0]);
}

async function visitAndAssertHeading(page, path, heading) {
  const response = await page.goto(path);
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { level: 1, name: heading })).toBeVisible();
}

test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@portal-game-catalog public routes projections localization responsive layout and accessibility', async ({ page }) => {
  const fixture = seedGameCatalog();
  expect(fixture.visible_entity_count).toBe(2);
  expect(fixture.visible_relation_count).toBe(1);

  await visitAndAssertHeading(page, '/en/wiki/catalog', 'Game Catalog');
  await expect(page.getByText('1 visible items', { exact: true })).toBeVisible();
  await expect(page.getByText('1 visible creatures', { exact: true })).toBeVisible();
  await assertResponsiveLayout(page);
  await assertAccessibilitySmoke(page);

  await page.getByRole('link', { name: 'Browse items' }).click();
  await expect(page.getByRole('heading', { level: 1, name: 'Items' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Fixture Sword', exact: true })).toBeVisible();
  await expect(page.getByText('Future Fixture Shield', { exact: true })).toHaveCount(0);
  await page.getByLabel('Search').fill('Future Fixture Shield');
  await page.getByRole('button', { name: 'Filter' }).click();
  await expect(page.getByRole('status')).toContainText('No visible items match these filters.');
  await assertResponsiveLayout(page);

  await page.goto('/en/wiki/items');
  await page.getByLabel('Category').selectOption('weapons');
  await page.getByLabel('Weapon type').selectOption('sword');
  await page.getByRole('button', { name: 'Filter' }).click();
  await expect(page.getByRole('link', { name: 'Fixture Sword', exact: true })).toBeVisible();
  await page.getByRole('link', { name: 'Fixture Sword', exact: true }).click();
  await expect(page.getByRole('heading', { level: 1, name: 'Fixture Sword' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Fixture Rat', exact: true })).toBeVisible();
  await expect(page.getByText(/Chance:\s*1 \/ 10/u)).toBeVisible();
  await expect(page.getByText(/1 \/ 20/u)).toHaveCount(0);
  await expect(page.getByText('catalog/fixtures/minimal-snapshot.json', { exact: true })).toHaveCount(0);
  await assertResponsiveLayout(page);
  await assertAccessibilitySmoke(page);

  await visitAndAssertHeading(page, '/en/wiki/creatures', 'Creatures');
  await expect(page.getByRole('link', { name: 'Fixture Rat', exact: true })).toBeVisible();
  await expect(page.getByText('Partial Fixture Beast', { exact: true })).toHaveCount(0);
  await page.getByRole('link', { name: 'Fixture Rat', exact: true }).click();
  await expect(page.getByRole('heading', { level: 1, name: 'Fixture Rat' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Fixture Sword', exact: true })).toBeVisible();
  await expect(page.getByText(/Chance:\s*1 \/ 10/u)).toBeVisible();
  await expect(page.getByText(/1 \/ 20/u)).toHaveCount(0);
  await assertResponsiveLayout(page);
  await assertAccessibilitySmoke(page);

  await visitAndAssertHeading(page, '/pl/wiki/catalog', 'Katalog gry');
  await page.getByRole('link', { name: 'Przeglądaj przedmioty' }).click();
  await expect(page.getByRole('link', { name: 'Miecz testowy', exact: true })).toBeVisible();
  await expect(page.getByText('Fixture Sword', { exact: true })).toHaveCount(0);
  await page.getByRole('link', { name: 'Miecz testowy', exact: true }).click();
  await expect(page.getByRole('heading', { level: 1, name: 'Miecz testowy' })).toBeVisible();
  await assertResponsiveLayout(page);

  for (const [path, heading] of [
    ['/wiki/catalog', 'Game Catalog'],
    ['/wiki/items', 'Items'],
    ['/wiki/items/fixture-sword', 'Fixture Sword'],
    ['/wiki/creatures', 'Creatures'],
    ['/wiki/creatures/fixture-rat', 'Fixture Rat'],
  ]) {
    await visitAndAssertHeading(page, path, heading);
  }
});
