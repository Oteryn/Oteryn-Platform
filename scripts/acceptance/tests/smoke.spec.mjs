import { test, expect } from '@playwright/test';
import {
  assertAccessibilitySmoke,
  attachDiagnostics,
  evidenceScreenshot,
  installDiagnostics,
} from './helpers.mjs';

test.use({
  trace: 'retain-on-failure',
  screenshot: 'only-on-failure',
});

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@smoke public critical surfaces render through the real HTTP runtime', async ({ page }) => {
  const surfaces = [
    ['/', 'Oteryn Platform'],
    ['/news', 'News'],
    ['/login', 'Sign in to Oteryn Platform'],
    ['/register', 'Create an Oteryn Platform identity'],
  ];

  for (const [url, heading] of surfaces) {
    const response = await page.goto(url);
    expect(response?.status(), `${url} should be successful`).toBe(200);
    await expect(page.getByRole('heading', { name: heading }).first()).toBeVisible();
    await assertAccessibilitySmoke(page);
  }
});

test.skip('@smoke character search reaches the seeded public character detail', async ({ page }) => {
  // Unavailable: game-world character data has no native source until native fixtures return.
  await page.goto('/');
  await page.getByLabel('Character name').fill('Acceptance Hero');
  await page.getByRole('button', { name: 'Search' }).click();
  await expect(page).toHaveURL(/\/characters\/Acceptance%20Hero$/u);
  await expect(page.getByRole('heading', { name: 'Acceptance Hero' })).toBeVisible();
  await expect(page.getByText('42', { exact: true })).toBeVisible();
});

test('@smoke protected administrator access denies an unauthenticated browser', async ({ page }) => {
  await page.goto('/admin');
  await expect(page).toHaveURL(/\/login$/u);
  await expect(page.getByRole('heading', { name: 'Sign in to Oteryn Platform' })).toBeVisible();
});

test('@smoke browser state-changing request without a CSRF token fails closed', async ({ page }) => {
  await page.goto('/register');
  const response = await page.request.post('/register', {
    form: {
      email: 'csrf-probe@example.test',
      password: 'AcceptancePassword!234',
      password_confirmation: 'AcceptancePassword!234',
    },
    maxRedirects: 0,
  });

  expect(response.status()).toBe(419);
});

test('@smoke representative public layout has no small-screen horizontal overflow', async ({ page }) => {
  for (const [label, viewport] of [
    ['desktop', { width: 1440, height: 1000 }],
    ['tablet', { width: 820, height: 1180 }],
    ['mobile', { width: 390, height: 844 }],
  ]) {
    await page.setViewportSize(viewport);
    await page.goto('/');
    await assertAccessibilitySmoke(page);
    await evidenceScreenshot(page, `home-${label}`);
  }
});
