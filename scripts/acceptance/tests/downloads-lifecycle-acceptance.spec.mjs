import { test, expect } from '@playwright/test';
import path from 'node:path';
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

test.setTimeout(240_000);
test.describe.configure({ retries: 0 });

const desktopViewport = { width: 1440, height: 1000 };
const tabletViewport = { width: 820, height: 1180 };
const mobileViewport = { width: 390, height: 844 };

function downloadsState(...args) {
  const output = runBinary('php', [
    path.join(repoRoot, 'scripts/acceptance/seed-downloads-state.php'),
    ...args,
  ]);

  return JSON.parse(output);
}

async function assertNoPageOverflow(page) {
  const dimensions = await page.evaluate(() => ({
    viewport: window.innerWidth,
    document: document.documentElement.scrollWidth,
  }));

  expect(dimensions.document, `Unexpected page overflow on ${page.url()}`).toBeLessThanOrEqual(dimensions.viewport + 1);
}

async function fillReleaseDraft(page, { version, notes, artifactUrl, filename, sha256 }) {
  await page.getByLabel('Version').fill(version);
  await page.getByLabel('Channel').selectOption('stable');
  await page.getByLabel('Release notes (plain text)').fill(notes);
  await page.locator('#artifact-0-platform').selectOption('windows');
  await page.locator('#artifact-0-architecture').selectOption('x86_64');
  await page.locator('#artifact-0-url').fill(artifactUrl);
  await page.locator('#artifact-0-filename').fill(filename);
  await page.locator('#artifact-0-size').fill('1572864');
  await page.locator('#artifact-0-sha256').fill(sha256);
}

function artifactDownloadLink(page, filename) {
  return page
    .locator('article.card')
    .filter({ hasText: filename })
    .getByRole('link', { name: 'Download', exact: true });
}

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);

  if (testInfo.status !== testInfo.expectedStatus && !page.isClosed()) {
    const screenshot = await page.screenshot({
      fullPage: true,
      mask: [page.locator('input'), page.locator('textarea'), page.locator('code')],
    });
    await testInfo.attach('sanitized-failure-screenshot', {
      body: screenshot,
      contentType: 'image/png',
    });
  }
});

test('@portal-downloads complete public, administrator, protected-integration simulation, localization and recovery lifecycle', async ({ browser, page }) => {
  const suffix = (process.env.ACCEPTANCE_RUN_ID ?? 'local')
    .replace(/[^a-zA-Z0-9-]/gu, '-')
    .toLowerCase()
    .slice(-24);
  const version = `7.0.0-${suffix}`.slice(0, 60);
  const filename = `oteryn-${suffix}.zip`;
  const approvedUrl = `https://downloads.example.test/releases/${version}/${filename}`;
  const rejectedUrl = `https://evil.example.test/releases/${version}/${filename}`;
  const sha256 = 'a'.repeat(64);
  const englishNotes = 'Acceptance English release notes.';
  const polishNotes = 'Informacje o wydaniu z testu akceptacyjnego.';

  await page.goto('/en/download');
  await expect(page.getByRole('heading', { name: 'Download Center' })).toBeVisible();
  await expect(page.getByText(/No current download is available/u)).toBeVisible();
  await assertNoPageOverflow(page);

  await page.goto('/admin/downloads');
  await expect(page).toHaveURL(/\/login$/u);

  const adminEmail = uniqueEmail('downloads-admin');
  const adminPassword = 'AcceptanceDownloadsAdmin!234';
  await register(page, adminEmail, adminPassword);
  await login(page, adminEmail, adminPassword);
  await enrollMfa(page, adminPassword);
  const bootstrap = runArtisan('admin:bootstrap', adminEmail);
  expect(bootstrap).toContain('First platform administrator assigned');
  const adminGrant = downloadsState('grant-downloads', adminEmail);
  expect(adminGrant.permission).toBe('downloads.manage');

  await page.goto('/admin/downloads');
  await expect(page.getByRole('heading', { name: 'Client releases' })).toBeVisible();

  const noMfaContext = await browser.newContext();
  const noMfaPage = await noMfaContext.newPage();
  try {
    const noMfaEmail = uniqueEmail('downloads-no-mfa');
    const noMfaPassword = 'AcceptanceDownloadsNoMfa!234';
    await register(noMfaPage, noMfaEmail, noMfaPassword);
    const grant = downloadsState('grant-downloads', noMfaEmail);
    expect(grant.permission).toBe('downloads.manage');
    await login(noMfaPage, noMfaEmail, noMfaPassword);
    const response = await noMfaPage.goto('/admin/downloads');
    expect(response?.status()).toBe(403);
  } finally {
    await noMfaContext.close();
  }

  const noPermissionContext = await browser.newContext();
  const noPermissionPage = await noPermissionContext.newPage();
  try {
    const noPermissionEmail = uniqueEmail('downloads-no-permission');
    const noPermissionPassword = 'AcceptanceDownloadsNoPermission!234';
    await register(noPermissionPage, noPermissionEmail, noPermissionPassword);
    await login(noPermissionPage, noPermissionEmail, noPermissionPassword);
    await enrollMfa(noPermissionPage, noPermissionPassword);
    const response = await noPermissionPage.goto('/admin/downloads');
    expect(response?.status()).toBe(403);
  } finally {
    await noPermissionContext.close();
  }

  await page.goto('/admin/downloads/create');
  await expect(page.getByRole('heading', { name: 'Create release draft' })).toBeVisible();
  await expect(page.getByText('No executable upload is available.')).toBeVisible();
  await expect(page.locator('input[type="file"]')).toHaveCount(0);

  await fillReleaseDraft(page, {
    version,
    notes: englishNotes,
    artifactUrl: rejectedUrl,
    filename,
    sha256,
  });
  await page.getByRole('button', { name: 'Save draft' }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page).toHaveURL(/\/admin\/downloads\/create$/u);

  await page.locator('#artifact-0-url').fill(approvedUrl);
  await page.getByRole('button', { name: 'Save draft' }).click();
  await expect(page).toHaveURL(/\/admin\/downloads\/\d+\/edit$/u);
  await expect(page.getByRole('heading', { name: 'Manage client release' })).toBeVisible();

  await page.goto('/en/download');
  await expect(page.getByText(version)).toHaveCount(0);

  await page.goto('/admin/downloads');
  await page.locator('tr').filter({ hasText: version }).getByRole('link', { name: 'Manage English' }).click();
  await page.getByRole('button', { name: 'Publish and make current' }).click();
  await expect(page.getByText('This release is published and current.')).toBeVisible();
  await expect(page.locator('input[type="file"]')).toHaveCount(0);
  await expect(page.locator('input[name="artifact_url"]')).toHaveCount(0);
  await expect(page.getByText('Browser-published only.')).toBeVisible();

  await page.getByRole('button', { name: 'Enable for automatic updates' }).click();
  await expect(page.getByText('Updater release identity enabled without changing browser publication state.')).toBeVisible();
  await expect(page.getByText('Channel sequence')).toBeVisible();
  await page.getByRole('link', { name: 'Stable updater diagnostics' }).click();
  await expect(page.getByRole('heading', { name: 'Stable updater diagnostics' })).toBeVisible();
  await expect(page.getByText('No private updater signing key is accepted or stored by this Platform.')).toBeVisible();
  await expect(page.getByText('No Platform-active updater generation.')).toBeVisible();
  await expect(page.getByText('This web console intentionally has no route to import or activate signed-generation metadata.')).toBeVisible();
  await expect(page.getByLabel('Public signed-generation metadata JSON')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Activate Platform updater state' })).toHaveCount(0);

  await page.getByLabel('Update mode').selectOption('recommended');
  await page.getByLabel('Minimum supported release sequence').fill('1');
  await page.getByRole('button', { name: 'Approve updater policy' }).click();
  await expect(page.getByText(/Updater policy revision 1 approved/u)).toBeVisible();
  await expect(page.getByText('Revision 1 · sequence 1 · Recommended')).toBeVisible();

  const protectedIntegration = downloadsState('reconcile-updater-generation', 'stable', adminEmail);
  expect(protectedIntegration.platform_active).toBe(true);
  expect(protectedIntegration.harness_scope).toContain('acceptance-only');
  expect(protectedIntegration.harness_scope).toContain('no cryptographic TUF signing');
  await page.reload();
  await expect(page.getByText(protectedIntegration.generation_id)).toBeVisible();
  await expect(page.getByText('Platform-active').first()).toBeVisible();
  await expect(page.getByRole('button', { name: 'Activate Platform updater state' })).toHaveCount(0);

  await page.goto('/en/download');
  await expect(page.getByRole('heading', { name: `Oteryn Client ${version}` })).toBeVisible();
  await expect(page.getByText(englishNotes)).toBeVisible();
  await expect(page.getByText(filename)).toBeVisible();
  await expect(page.getByText('1.5 MB')).toBeVisible();
  await expect(page.getByText(sha256)).toBeVisible();
  await expect(page.getByText('Platform-active signed generation selects this exact release.')).toBeVisible();
  await expect(page.getByText(/first-party updater independently verifies TUF signatures/u)).toBeVisible();
  await expect(artifactDownloadLink(page, filename)).toHaveAttribute('href', approvedUrl);

  await page.goto('/en/download/windows');
  await expect(page.getByText(filename)).toBeVisible();
  await page.goto('/en/download/macos');
  await expect(page.getByText('No current download is available for macOS.')).toBeVisible();

  await page.goto('/pl/download');
  await expect(page.getByText(filename)).toBeVisible();
  await expect(page.getByText('Opis wydania nie jest dostępny w tym języku.')).toBeVisible();
  await expect(page.getByText(/Aktywna w Platformie podpisana generacja wskazuje dokładnie to wydanie/u)).toBeVisible();
  await expect(page.getByText(englishNotes)).toHaveCount(0);

  await page.goto('/admin/downloads');
  await page.locator('tr').filter({ hasText: version }).getByRole('link', { name: 'Polish release notes' }).click();
  await expect(page.getByRole('heading', { name: 'Polish translation' })).toBeVisible();
  await page.getByLabel('Polish content (plain text)').fill(polishNotes);
  await page.getByLabel('Publish Polish translation at (UTC)').fill('2000-01-01T00:00');
  await page.getByRole('button', { name: 'Save translation' }).click();

  await page.goto('/pl/download');
  await expect(page.getByText(polishNotes)).toBeVisible();
  await expect(page.getByText(englishNotes)).toHaveCount(0);
  await expect(page.getByText('Opis wydania nie jest dostępny w tym języku.')).toHaveCount(0);

  downloadsState('set-artifact-url', version, rejectedUrl);
  await page.goto('/en/download');
  await expect(page.getByText('Downloads are temporarily unavailable.')).toBeVisible();
  await expect(page.locator(`a[href="${rejectedUrl}"]`)).toHaveCount(0);

  downloadsState('set-artifact-url', version, approvedUrl);
  await page.reload();
  await expect(page.getByRole('heading', { name: `Oteryn Client ${version}` })).toBeVisible();
  await expect(artifactDownloadLink(page, filename)).toHaveAttribute('href', approvedUrl);

  for (const viewport of [desktopViewport, tabletViewport, mobileViewport]) {
    await page.setViewportSize(viewport);
    await page.goto('/en/download');
    await assertNoPageOverflow(page);
    await page.goto('/pl/download');
    await assertNoPageOverflow(page);
    await page.goto('/admin/downloads');
    await assertNoPageOverflow(page);
    await page.goto('/admin/downloads/updater/stable');
    await assertNoPageOverflow(page);
  }

  await page.setViewportSize(desktopViewport);
  await page.goto('/admin/audit');
  await expect(page.getByText('downloads.release_created').first()).toBeVisible();
  await expect(page.getByText('downloads.release_published').first()).toBeVisible();
  await expect(page.getByText('downloads.updater_release_enabled').first()).toBeVisible();
  await expect(page.getByText('downloads.updater_policy_approved').first()).toBeVisible();
  await expect(page.getByText('downloads.updater_generation_reconciled').first()).toBeVisible();
  await expect(page.getByText('downloads.updater_generation_activated').first()).toBeVisible();

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});