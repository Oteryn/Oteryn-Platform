import { test, expect } from '@playwright/test';
import {
  assertAccessibilitySmoke,
  attachDiagnostics,
  installDiagnostics,
  runBinary,
} from './helpers.mjs';

test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeAll(() => {
  runBinary('php', ['scripts/acceptance/seed-homepage-navigation-seo.php']);
});

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@homepage-seo homepage navigation metadata and crawl policy remain responsive and keyboard operable', async ({ page, request }) => {
  const response = await page.goto('/en');
  expect(response?.status()).toBe(200);

  await expect(page.getByText('Acceptance realm maintenance')).toBeVisible();
  await expect(page.getByText('Acceptance tournament')).toBeVisible();
  await expect(page.locator('meta[name="description"]')).toHaveAttribute('content', /Ancient powers stir/u);
  await expect(page.locator('meta[property="og:title"]')).toHaveAttribute('content', /Oteryn Platform/u);
  await expect(page.locator('meta[property="og:url"]')).toHaveAttribute('content', /\/en$/u);
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute('href', /\/en$/u);
  await expect(page.locator('link[rel="alternate"][hreflang="pl"]')).toHaveAttribute('href', /\/pl$/u);

  const desktopNavigation = page.getByRole('navigation', { name: 'Public navigation' });
  if (await desktopNavigation.isVisible()) {
    await expect(desktopNavigation.getByRole('link', { name: 'Guilds', exact: true })).toBeVisible();
    await expect(desktopNavigation.getByRole('link', { name: 'Download', exact: true })).toBeVisible();
  } else {
    await page.getByText('Menu', { exact: true }).click();
    const mobilePanel = page.locator('.mobile-nav-panel');
    await expect(mobilePanel.getByRole('link', { name: 'Guilds', exact: true })).toBeVisible();
    await expect(mobilePanel.getByRole('link', { name: 'Download', exact: true })).toBeVisible();
  }

  const quickDownload = page.locator('.production-discover').getByRole('link', { name: /Download/u });
  await quickDownload.focus();
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/\/en\/download$/u);

  await page.goto('/en');
  await assertAccessibilitySmoke(page);

  const sitemap = await request.get('/sitemap.xml');
  expect(sitemap.status()).toBe(200);
  expect(sitemap.headers()['content-type']).toContain('application/xml');
  const sitemapBody = await sitemap.text();
  expect(sitemapBody).toContain('/en/events/acceptance-tournament');
  expect(sitemapBody).toContain('/pl/events/turniej-testowy');
  expect(sitemapBody).not.toContain('/admin');
  expect(sitemapBody).not.toContain('/wiki/search');

  const robots = await request.get('/robots.txt');
  expect(robots.status()).toBe(200);
  const robotsBody = await robots.text();
  expect(robotsBody).toContain('Disallow: /admin');
  expect(robotsBody).toContain('Disallow: /account');
  expect(robotsBody).toContain('Sitemap:');
});
