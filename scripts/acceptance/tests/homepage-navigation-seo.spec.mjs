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

test('@portal-today public guest command centre preserves source truth empty partial LiveOps absence localization and no-store', async ({ page, request }) => {
  let response = await page.goto('/en/today');
  expect(response?.status()).toBe(200);
  expect(response?.headers()['cache-control']).toContain('no-store');
  await expect(page.locator('[data-today-state="partial"]')).toBeVisible();
  await expect(page.locator('[data-today-card="liveops"]')).toHaveAttribute('data-content-state', 'unavailable');
  await expect(page.locator('[data-today-card="liveops"]')).toHaveAttribute('data-today-runtime-evidence', 'absent');
  await expect(page.locator('[data-today-card="liveops"] [data-today-item]')).toHaveCount(0);
  await expect(page.getByText('Acceptance Today maintenance')).toBeVisible();
  await expect(page.getByText('Acceptance Today event')).toBeVisible();
  await expect(page.getByText('Acceptance Today news')).toBeVisible();
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute('href', /\/en\/today$/u);
  await expect(page.locator('link[rel="alternate"][hreflang="pl"]')).toHaveAttribute('href', /\/pl\/today$/u);
  await assertAccessibilitySmoke(page);

  const dimensions = await page.evaluate(() => ({
    scrollWidth: document.documentElement.scrollWidth,
    innerWidth: window.innerWidth,
  }));
  expect(dimensions.scrollWidth).toBeLessThanOrEqual(dimensions.innerWidth + 1);

  await page.setExtraHTTPHeaders({ 'X-Oteryn-Acceptance-Today-Scenario': 'empty' });
  response = await page.goto('/en/today');
  expect(response?.status()).toBe(200);
  for (const kind of ['announcements', 'events', 'news']) {
    await expect(page.locator(`[data-today-card="${kind}"]`)).toHaveAttribute('data-content-state', 'empty');
  }
  await expect(page.getByText('Acceptance Today maintenance')).toHaveCount(0);
  await expect(page.locator('[data-today-card="liveops"]')).toHaveAttribute('data-content-state', 'unavailable');

  await page.setExtraHTTPHeaders({ 'X-Oteryn-Acceptance-Today-Scenario': 'news-outage' });
  response = await page.goto('/en/today');
  expect(response?.status()).toBe(200);
  await expect(page.locator('[data-today-state="partial"]')).toBeVisible();
  await expect(page.getByText('Acceptance Today maintenance')).toBeVisible();
  await expect(page.getByText('Acceptance Today event')).toBeVisible();
  await expect(page.locator('[data-today-card="news"]')).toHaveAttribute('data-content-state', 'unavailable');
  await expect(page.getByText('Published news is temporarily unavailable.')).toBeVisible();

  await page.setExtraHTTPHeaders({});
  response = await page.goto('/pl/today');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { level: 1, name: 'Dzisiaj' })).toBeVisible();
  await expect(page.getByText('Testowa konserwacja Dzisiaj')).toBeVisible();
  await expect(page.getByText('Testowe wydarzenie Dzisiaj')).toBeVisible();
  await expect(page.getByText('Testowa aktualność Dzisiaj')).toBeVisible();
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute('href', /\/pl\/today$/u);
  await expect(page.locator('link[rel="alternate"][hreflang="en"]')).toHaveAttribute('href', /\/en\/today$/u);

  const sitemap = await request.get('/sitemap.xml');
  expect(sitemap.status()).toBe(200);
  const sitemapBody = await sitemap.text();
  expect(sitemapBody).toContain('/en/today');
  expect(sitemapBody).toContain('/pl/today');
});
