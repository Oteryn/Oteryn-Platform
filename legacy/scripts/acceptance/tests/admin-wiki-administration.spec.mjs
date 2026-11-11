import crypto from 'node:crypto';
import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  completeMfaChallenge,
  installDiagnostics,
  login,
  runArtisan,
  runBinary,
  uniqueEmail,
} from './helpers.mjs';

const password = 'Acceptance-Wiki-Admin-9!Pass';
const recoveryCode = 'WIKIADMIN-00001';

function seedWikiAdmin(email) {
  runBinary('php', [
    'scripts/acceptance/seed-browser-admin.php',
    email,
    password,
    recoveryCode,
  ]);
  runBinary('php', ['scripts/acceptance/seed-admin-wiki-permissions.php', email]);
}

function suffix() {
  return crypto.randomBytes(6).toString('hex');
}

async function expectNoHorizontalOverflow(page) {
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow).toBeLessThanOrEqual(1);
}

test.setTimeout(120_000);
test.describe.configure({ mode: 'serial', retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@wiki-admin trusted editor creates, previews and publishes bilingual Wiki content', async ({ page, context }) => {
  const email = uniqueEmail('wiki-admin');
  const id = suffix();
  const categoryName = `Getting Started ${id}`;
  const categorySlug = `getting-started-${id}`;
  const articleTitle = `Installation Guide ${id}`;
  const articleSlug = `installation-guide-${id}`;
  const polishTitle = `Poradnik instalacji ${id}`;
  const polishSlug = `poradnik-instalacji-${id}`;

  // Portability and responsive projects share one acceptance Redis runtime.
  // Clear prior-project throttle state before this independently seeded login.
  runArtisan('cache:clear');
  seedWikiAdmin(email);
  await login(page, email, password);
  await completeMfaChallenge(page, recoveryCode);

  await page.goto('/admin/wiki');
  await expect(page.getByRole('heading', { name: 'Wiki administration' })).toBeVisible();
  await expectNoHorizontalOverflow(page);

  await page.getByRole('link', { name: 'Create category' }).click();
  await page.getByLabel('Stable key').fill(`getting-started-${id}`);
  await page.getByLabel('Display order').fill('10');
  await page.getByLabel('Name').first().fill(categoryName);
  await page.getByLabel('Slug').first().fill(categorySlug);
  await page.getByLabel('Description').first().fill('Source-backed onboarding articles.');
  await page.getByLabel('Name').nth(1).fill(`Start ${id}`);
  await page.getByLabel('Slug').nth(1).fill(`start-${id}`);
  await page.getByLabel('Description').nth(1).fill('Artykuły wprowadzające.');
  await page.getByRole('button', { name: 'Create category' }).click();
  await expect(page.getByRole('status')).toContainText('Wiki category created.');

  await page.goto('/admin/wiki/articles/create');
  await expect(page.getByRole('heading', { name: 'Create Wiki article' })).toBeVisible();
  await page.getByLabel('Content type').fill('guide');
  await page.getByLabel('Display order').fill('5');
  await page.getByLabel('Feature this article').check();
  await page.getByLabel('Title').first().fill(articleTitle);
  await page.getByLabel('Slug').first().fill(articleSlug);
  await page.getByLabel('Summary').first().fill('Approved installation guidance for Oteryn.');
  await page.getByLabel('Markdown source').first().fill('# Install Oteryn\n\nUse only the approved Oteryn client package.');
  await page.getByLabel('Title').nth(1).fill(polishTitle);
  await page.getByLabel('Slug').nth(1).fill(polishSlug);
  await page.getByLabel('Summary').nth(1).fill('Zatwierdzony poradnik instalacji Oteryn.');
  await page.getByLabel('Markdown source').nth(1).fill('# Instalacja Oteryn\n\nUżywaj wyłącznie zatwierdzonego pakietu klienta Oteryn.');
  await page.getByLabel(categoryName).check();
  await page.getByLabel('Change note').fill('Acceptance editor baseline.');
  await page.getByRole('button', { name: 'Create draft' }).click();

  await expect(page.getByRole('status')).toContainText('Wiki article draft created.');
  await expect(page.getByRole('heading', { name: articleTitle })).toBeVisible();
  await expectNoHorizontalOverflow(page);

  const previewPromise = context.waitForEvent('page');
  await page.getByRole('link', { name: 'Preview EN' }).click();
  const preview = await previewPromise;
  await preview.waitForLoadState();
  await expect(preview.getByRole('heading', { name: articleTitle })).toBeVisible();
  await expect(preview.getByRole('heading', { name: 'Install Oteryn' })).toBeVisible();
  await expect(preview.locator('meta[name="robots"]')).toHaveAttribute('content', 'noindex,nofollow,noarchive');
  await expectNoHorizontalOverflow(preview);
  await preview.close();

  // Keep the historical publication-flash failure as a current zero-retry regression gate.
  // The separately proven Editorial Media fixture isolation prevents stale damaged rows from
  // contaminating this clean journey, while diagnostics fail it on any unexplained HTTP 5xx.
  await page.waitForLoadState('networkidle');
  await page.getByRole('button', { name: 'Submit for review' }).click();
  await expect(page.getByText(/Status:\s*In Review/i)).toBeVisible();
  await expect(page.getByRole('button', { name: 'Return to draft' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Publish', exact: true })).toBeVisible();

  // Quiesce the refreshed picker again before the next lifecycle mutation.
  await page.waitForLoadState('networkidle');
  await page.getByRole('button', { name: 'Publish', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Wiki article published.');
  await expect(page.getByText(/Status:\s*Published/i)).toBeVisible();
  await expect(page.getByRole('button', { name: 'Unpublish to draft' })).toBeVisible();
  expect(
    page.__acceptanceDiagnostics.serverErrors,
    'clean Wiki administration lifecycle emitted unexplained HTTP 5xx responses',
  ).toEqual([]);

  await page.goto(`/en/wiki/${articleSlug}`);
  await expect(page.getByRole('heading', { name: articleTitle })).toBeVisible();
  await expect(page.getByText('Use only the approved Oteryn client package.')).toBeVisible();
  await expectNoHorizontalOverflow(page);

  await page.goto(`/pl/wiki/${polishSlug}`);
  await expect(page.getByRole('heading', { name: polishTitle })).toBeVisible();
  await expect(page.getByText('Używaj wyłącznie zatwierdzonego pakietu klienta Oteryn.')).toBeVisible();
  await expectNoHorizontalOverflow(page);
});
