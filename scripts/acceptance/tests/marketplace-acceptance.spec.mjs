import { test, expect } from '@playwright/test';
import {
  assertAccessibilitySmoke,
  attachDiagnostics,
  completeMfaChallenge,
  installDiagnostics,
  login,
  runBinary,
  uniqueEmail,
} from './helpers.mjs';

const bidderPassword = 'Acceptance-Marketplace-9!Pass';
const adminPassword = 'Acceptance-Marketplace-Admin-9!Pass';
const adminRecoveryCode = 'MARKET-00001';

function seedMarketplace(email) {
  return JSON.parse(runBinary('php', [
    'scripts/acceptance/seed-marketplace.php',
    email,
    bidderPassword,
  ]));
}

function seedAdmin(email) {
  return JSON.parse(runBinary('php', [
    'scripts/acceptance/seed-browser-admin.php',
    email,
    adminPassword,
    adminRecoveryCode,
  ]));
}

async function assertNoHorizontalOverflow(page) {
  const dimensions = await page.evaluate(() => ({
    viewport: document.documentElement.clientWidth,
    content: document.documentElement.scrollWidth,
  }));
  expect(dimensions.content).toBeLessThanOrEqual(dimensions.viewport + 1);
}

test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@portal-marketplace-public catalogue detail filters localization responsive layout and accessibility', async ({ page }) => {
  const fixture = seedMarketplace(uniqueEmail('marketplace-public'));

  await page.goto('/bazaar');
  await expect(page.getByRole('heading', { name: 'Character Bazaar' })).toBeVisible();
  await expect(page.getByText(fixture.player_name, { exact: true })).toBeVisible();
  await page.getByLabel('Minimum level').fill('300');
  await page.getByRole('button', { name: 'Apply filters' }).click();
  await expect(page.getByText(fixture.player_name, { exact: true })).toBeVisible();
  await assertNoHorizontalOverflow(page);
  await assertAccessibilitySmoke(page);

  await page.getByRole('link', { name: 'View character' }).click();
  await expect(page.getByRole('heading', { name: fixture.player_name })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Verified listing snapshot' })).toBeVisible();
  await expect(page.getByText('Sword fighting', { exact: true })).toBeVisible();
  await expect(page.getByText('112', { exact: true })).toBeVisible();
  await assertNoHorizontalOverflow(page);
  await assertAccessibilitySmoke(page);

  await page.goto('/pl/bazaar');
  await expect(page.getByRole('heading', { name: 'Bazar postaci' })).toBeVisible();
  await expect(page.getByText(fixture.player_name, { exact: true })).toBeVisible();
});

test('@portal-marketplace-account authenticated watch bid wallet reservation dashboard and authorization journey', async ({ page }) => {
  const email = uniqueEmail('marketplace-bidder');
  const fixture = seedMarketplace(email);

  await login(page, email, bidderPassword);
  await page.goto(`/bazaar/${fixture.auction_id}`);
  await page.getByRole('button', { name: 'Watch auction' }).click();
  await expect(page.getByRole('status')).toContainText('Auction added to your watchlist.');

  await page.getByLabel('Amount in Oteryn Coins').fill('200');
  await page.getByRole('button', { name: 'Reserve coins and bid' }).click();
  await expect(page.getByRole('status')).toContainText('Your bid was placed');

  await page.goto('/account/bazaar');
  await expect(page.getByRole('heading', { name: 'My Bazaar' })).toBeVisible();
  await expect(page.getByText('4 800', { exact: true })).toBeVisible();
  await expect(page.getByText('200', { exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'My bids' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Watched auctions' })).toBeVisible();
  await expect(page.getByText(fixture.player_name, { exact: true }).first()).toBeVisible();
  await assertNoHorizontalOverflow(page);
  await assertAccessibilitySmoke(page);
});

test('@portal-marketplace-admin MFA permission wallet adjustment ledger and recovery queue surface', async ({ page }) => {
  const targetEmail = uniqueEmail('marketplace-wallet-target');
  seedMarketplace(targetEmail);
  const adminEmail = uniqueEmail('marketplace-admin');
  seedAdmin(adminEmail);

  await login(page, adminEmail, adminPassword);
  await completeMfaChallenge(page, adminRecoveryCode);
  await page.goto('/admin/marketplace');
  await expect(page.getByRole('heading', { name: 'Character Bazaar' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Auctions requiring operator reconciliation' })).toBeVisible();

  await page.getByLabel('Platform Identity email').fill(targetEmail);
  await page.getByRole('button', { name: 'Find wallet' }).click();
  await expect(page.getByText(targetEmail, { exact: true })).toBeVisible();
  await expect(page.getByText('5 000 Oteryn Coins', { exact: true })).toBeVisible();

  await page.getByLabel('Signed adjustment').fill('300');
  await page.getByLabel('Operational reason').fill('Controlled acceptance allocation for marketplace verification.');
  await page.getByRole('button', { name: 'Record adjustment' }).click();
  await expect(page.getByRole('status')).toContainText('Wallet adjustment recorded.');
  await expect(page.getByText('5 300 Oteryn Coins', { exact: true })).toBeVisible();
  await expect(page.getByText('administrator_adjustment', { exact: true })).toBeVisible();
  await assertNoHorizontalOverflow(page);
  await assertAccessibilitySmoke(page);
});
