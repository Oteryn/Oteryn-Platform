import { test, expect } from '@playwright/test';
import {
  assertAccessibilitySmoke,
  attachDiagnostics,
  installDiagnostics,
  runBinary,
  uniqueEmail,
} from './helpers.mjs';

const password = 'Acceptance-Wiki-Launch-9!Pass';
const recoveryCode = 'WIKILAUNCH-00001';

function seedLaunchContent() {
  const publisher = uniqueEmail('wiki-launch-publisher');
  runBinary('php', [
    'scripts/acceptance/seed-browser-admin.php',
    publisher,
    password,
    recoveryCode,
  ]);
  runBinary('php', [
    'scripts/acceptance/seed-admin-wiki-permissions.php',
    publisher,
    '--wiki-only',
  ]);
  runBinary('php', [
    'scripts/acceptance/seed-wiki-launch-content.php',
    publisher,
  ]);
}

async function expectNoHorizontalOverflow(page) {
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow).toBeLessThanOrEqual(1);
}

test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  seedLaunchContent();
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@wiki-launch reviewed bilingual launch content is discoverable responsive and accessible', async ({ page, request }) => {
  const wikiResponse = await page.goto('/en/wiki');
  expect(wikiResponse?.status()).toBe(200);
  await expect(page.getByRole('heading', { level: 1, name: 'Wiki' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Getting Started' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Server Information' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Game Systems' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Support', exact: true })).toBeVisible();
  await expectNoHorizontalOverflow(page);
  await assertAccessibilitySmoke(page);

  await page.goto('/en/wiki/download-and-installation');
  await expect(page.getByRole('heading', { level: 1, name: 'Download and installation' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Download from the approved source' })).toHaveAttribute(
    'href',
    '#download-from-the-approved-source',
  );
  await expect(page.getByRole('link', { name: 'Download Center' })).toHaveAttribute('href', '/download');
  await expect(page.locator('link[rel="alternate"][hreflang="pl"]')).toHaveAttribute(
    'href',
    /\/pl\/wiki\/pobieranie-i-instalacja$/u,
  );
  await expect(page.locator('img[src^="http"]')).toHaveCount(0);
  await expectNoHorizontalOverflow(page);
  await assertAccessibilitySmoke(page);

  await page.goto('/pl/wiki/pobieranie-i-instalacja');
  await expect(page.locator('html')).toHaveAttribute('lang', 'pl');
  await expect(page.getByRole('heading', { level: 1, name: 'Pobieranie i instalacja' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Pobierz z zatwierdzonego źródła' })).toBeVisible();
  await expectNoHorizontalOverflow(page);
  await assertAccessibilitySmoke(page);

  await page.goto('/en/wiki/server-rates');
  await expect(page.getByText(/contains no approved numeric experience/u)).toBeVisible();
  await expectNoHorizontalOverflow(page);

  await page.goto('/en/wiki/search?q=vocations');
  await expect(page.getByRole('link', { name: 'Vocations' })).toBeVisible();
  await assertAccessibilitySmoke(page);

  const sitemap = await request.get('/sitemap.xml');
  expect(sitemap.status()).toBe(200);
  const sitemapBody = await sitemap.text();
  expect(sitemapBody).toContain('/en/wiki/download-and-installation');
  expect(sitemapBody).toContain('/pl/wiki/pobieranie-i-instalacja');
  expect(sitemapBody).toContain('/en/wiki/report-a-bug');
  expect(sitemapBody).toContain('/pl/wiki/zglos-blad');
});
