import path from 'node:path';
import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  enrollMfa,
  installDiagnostics,
  login,
  register,
  repoRoot,
  runArtisan,
  runBinary,
  uniqueEmail,
} from './helpers.mjs';

test.setTimeout(180_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@portal-homepage-template-selector administrator preview activation rollback localization responsive layout keyboard and cache controls', async ({ page }) => {
  const adminEmail = uniqueEmail('homepage-template-admin');
  const adminPassword = 'HomepageTemplate!234';

  await register(page, adminEmail, adminPassword);
  await login(page, adminEmail, adminPassword);
  await enrollMfa(page, adminPassword);
  expect(runArtisan('admin:bootstrap', adminEmail)).toContain('First platform administrator assigned');
  expect(runBinary('php', [path.join(repoRoot, 'scripts/acceptance/prepare-homepage-template-selector.php'), adminEmail])).toContain('ready');

  const requiredViewports = [
    { width: 1440, height: 1000 },
    { width: 820, height: 1180 },
    { width: 390, height: 844 },
  ];

  for (const viewport of requiredViewports) {
    await page.setViewportSize(viewport);
    await page.goto('/admin/portal/homepage');
    await expect(page.getByRole('heading', { name: 'Homepage template' })).toBeVisible();
    const noHorizontalOverflow = await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth);
    expect(noHorizontalOverflow).toBe(true);
  }

  const previewResponse = await page.goto('/admin/portal/homepage/preview/classic');
  expect(previewResponse?.headers()['x-robots-tag']).toBe('noindex, nofollow');
  expect(previewResponse?.headers()['cache-control']).toContain('no-store');
  expect(previewResponse?.headers()['cache-control']).toContain('private');
  await expect(page.locator('body')).toHaveClass(/classic-home-shell/);

  await page.goto('/admin/portal/homepage');
  const classicCard = page.locator('article').filter({ hasText: 'Classic portal' });
  const activate = classicCard.getByRole('button', { name: 'Activate' });
  await activate.focus();
  await expect(activate).toBeFocused();
  await page.keyboard.press('Space');
  await expect(page.getByRole('status')).toContainText('Homepage template activated.');

  await page.goto('/');
  await expect(page.locator('body')).toHaveClass(/classic-home-shell/);

  await page.goto('/login?locale=pl');
  await page.goto('/admin/portal/homepage');
  await expect(page.getByRole('heading', { name: 'Szablon strony głównej' })).toBeVisible();
  await expect(page.getByText('Klasyczny portal').first()).toBeVisible();

  const rollback = page.getByRole('button', { name: /Przywróć/ });
  await rollback.focus();
  await expect(rollback).toBeFocused();
  await page.keyboard.press('Space');
  await expect(page.getByRole('status')).toContainText('Poprzedni szablon strony głównej został przywrócony.');

  await page.goto('/');
  await expect(page.locator('body')).toHaveClass(/production-home-shell/);
});
