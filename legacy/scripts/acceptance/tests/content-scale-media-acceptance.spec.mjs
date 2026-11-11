import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  completeMfaChallenge,
  installDiagnostics,
  login,
  logout,
  runBinary,
} from './helpers.mjs';

const mediaScaleMarker = '@portal-content-scale-media bounded Editorial Media pagination and table containment use valid private bytes';
const adminEmail = 'content-scale-media-administrator@example.test';
const adminPassword = 'ContentScaleMedia!234';
const adminRecoveryCode = 'CONTENT-SCALE-MEDIA-01';

test.setTimeout(150_000);
test.describe.configure({ retries: 0, mode: 'serial' });

test.beforeEach(async ({ page }) => {
  page.__mediaScaleFixture = JSON.parse(
    runBinary('php', ['scripts/acceptance/seed-content-scale-media.php']),
  );
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

async function expectNoHorizontalOverflow(page) {
  const dimensions = await page.evaluate(() => ({
    documentWidth: document.documentElement.scrollWidth,
    viewportWidth: document.documentElement.clientWidth,
  }));
  expect(dimensions.documentWidth).toBeLessThanOrEqual(dimensions.viewportWidth + 1);
}

async function expectTableContainment(locator) {
  const metrics = await locator.evaluate((element) => {
    const region = element.closest('.table-wrap');
    const table = region?.querySelector('table');
    if (!(region instanceof HTMLElement) || !(table instanceof HTMLTableElement)) {
      throw new Error('Expected Editorial Media metadata inside a table-wrap table.');
    }

    const elementRect = element.getBoundingClientRect();
    const tableRect = table.getBoundingClientRect();
    return {
      overflowX: getComputedStyle(region).overflowX,
      clientWidth: region.clientWidth,
      scrollWidth: region.scrollWidth,
      elementLeft: elementRect.left,
      elementRight: elementRect.right,
      tableLeft: tableRect.left,
      tableRight: tableRect.right,
    };
  });

  expect(['auto', 'scroll']).toContain(metrics.overflowX);
  expect(metrics.scrollWidth).toBeGreaterThanOrEqual(metrics.clientWidth);
  expect(metrics.elementLeft).toBeGreaterThanOrEqual(metrics.tableLeft - 1);
  expect(metrics.elementRight).toBeLessThanOrEqual(metrics.tableRight + 1);
}

async function signInAsMediaManager(page) {
  runBinary('php', [
    'scripts/acceptance/seed-browser-announcements.php',
    'seed-identity',
    adminEmail,
    adminPassword,
    adminRecoveryCode,
    'confirmed',
    'media.manage',
  ]);
  await login(page, adminEmail, adminPassword);
  await completeMfaChallenge(page, adminRecoveryCode);
}

// Evidence marker: @portal-content-scale-media bounded Editorial Media pagination and table containment use valid private bytes
test(mediaScaleMarker, async ({ page }) => {
  const fixture = page.__mediaScaleFixture;
  await signInAsMediaManager(page);

  let response = await page.goto('/admin/media');
  expect(response?.status()).toBe(200);
  const longName = page.getByText(fixture.media_long_name, { exact: true });
  await expect(longName).toBeVisible();
  await expect(page.getByText(fixture.media_long_alt, { exact: true })).toBeVisible();
  await expectTableContainment(longName);

  const preview = page.getByAltText(fixture.media_long_alt);
  await expect(preview).toBeVisible();
  await expect.poll(() => preview.evaluate((image) => image instanceof HTMLImageElement ? image.naturalWidth : 0)).toBe(1);
  await expect(page.getByText('content-scale-media-001.png', { exact: true })).toHaveCount(0);
  await expectNoHorizontalOverflow(page);

  response = await page.goto('/admin/media?page=2');
  expect(response?.status()).toBe(200);
  await expect(page.getByText('content-scale-media-001.png', { exact: true })).toBeVisible();
  await expect(page.getByText(fixture.media_long_name, { exact: true })).toHaveCount(0);
  const pageTwoUrl = page.url();
  await page.goto(pageTwoUrl);
  await expect(page.getByText('content-scale-media-001.png', { exact: true })).toBeVisible();
  await expectNoHorizontalOverflow(page);

  await logout(page);
  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});
