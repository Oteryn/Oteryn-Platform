import { test, expect } from '@playwright/test';
import {
  allowExpectedHttpFailure,
  assertAccessibilitySmoke,
  attachDiagnostics,
  completeMfaChallenge,
  evidenceScreenshot,
  installDiagnostics,
  login,
  logout,
  runBinary,
  uniqueEmail,
} from './helpers.mjs';

const password = 'AcceptanceEditorialMedia!234';

function editorialMediaFixture(...args) {
  return JSON.parse(runBinary('php', ['scripts/acceptance/seed-browser-editorial-media.php', ...args]));
}

function seedIdentity(label, { confirmedMfa, permissions }) {
  const email = uniqueEmail(label);
  const recoveryCode = `MED-${label.toUpperCase().replace(/[^A-Z0-9]/gu, '').slice(0, 14)}-01`;
  editorialMediaFixture(
    'seed-identity',
    email,
    password,
    recoveryCode,
    confirmedMfa ? 'confirmed' : 'unconfirmed',
    permissions.join(','),
  );

  return { email, password, recoveryCode, confirmedMfa };
}

async function signIn(page, identity) {
  await login(page, identity.email, identity.password);
  if (identity.confirmedMfa) {
    await completeMfaChallenge(page, identity.recoveryCode);
  }
}

async function assertNoPageOverflow(page) {
  const dimensions = await page.evaluate(() => ({
    viewport: window.innerWidth,
    document: document.documentElement.scrollWidth,
  }));
  expect(dimensions.document, `Unexpected page overflow on ${page.url()}`).toBeLessThanOrEqual(dimensions.viewport + 1);
}

test.setTimeout(180_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  editorialMediaFixture('reset');
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  try {
    await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
  } finally {
    editorialMediaFixture('reset');
  }
});

test('@portal-editorial-media guest, MFA and exact permission boundaries fail closed', async ({ page }) => {
  await page.goto('/admin/media');
  await expect(page).toHaveURL(/\/login$/u);

  const noMfa = seedIdentity('editorial-media-no-mfa', {
    confirmedMfa: false,
    permissions: ['media.manage'],
  });
  await signIn(page, noMfa);
  let response = await page.goto('/admin/media');
  expect(response?.status()).toBe(403);
  await expect(page.getByRole('heading', { name: 'You do not have access to this page' })).toBeVisible();
  await logout(page);

  const noPermission = seedIdentity('editorial-media-no-permission', {
    confirmedMfa: true,
    permissions: [],
  });
  await signIn(page, noPermission);
  response = await page.goto('/admin/media');
  expect(response?.status()).toBe(403);
  await expect(page.getByRole('heading', { name: 'You do not have access to this page' })).toBeVisible();

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});

test('@portal-editorial-media manager validates, uploads, privately previews, protects references, deletes and audits', async ({ page, browser }) => {
  const manager = seedIdentity('editorial-media-manager', {
    confirmedMfa: true,
    permissions: ['media.manage', 'audit.view'],
  });
  await signIn(page, manager);

  await page.goto('/admin/media');
  await expect(page.getByRole('heading', { name: 'Editorial image library' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'No editorial images' })).toBeVisible();
  await assertAccessibilitySmoke(page);
  await assertNoPageOverflow(page);

  const imageInput = page.getByLabel('Image', { exact: true });
  await imageInput.setInputFiles({
    name: 'malformed.png',
    mimeType: 'image/png',
    buffer: Buffer.from('not a raster image'),
  });
  await page.getByLabel('Alternative text').fill('Rejected malformed image.');
  await page.getByRole('button', { name: 'Upload image' }).click();
  await expect(page.getByRole('alert')).toContainText('The request could not be completed.');
  await expect(page.getByRole('heading', { name: 'No editorial images' })).toBeVisible();

  const uploadedAlt = `Acceptance private bridge ${Math.random().toString(16).slice(2, 10)}`;
  const uploadFixture = editorialMediaFixture('create-upload-fixture');
  await imageInput.setInputFiles(uploadFixture.path);
  await page.getByLabel('Alternative text').fill(uploadedAlt);
  await page.getByRole('button', { name: 'Upload image' }).click();
  await expect(page.getByRole('status')).toContainText('Editorial image uploaded safely.');

  let uploadedRow = page.getByRole('row').filter({ hasText: uploadedAlt });
  await expect(uploadedRow).toBeVisible();
  await expect(uploadedRow.locator('code.admin-media-digest')).toHaveText(/^[a-f0-9]{64}$/u);
  const previewImage = uploadedRow.getByRole('img', { name: uploadedAlt });
  await expect(previewImage).toBeVisible();
  await expect.poll(
    () => previewImage.evaluate((image) => image.complete ? image.naturalWidth : 0),
    { message: 'Authenticated editorial image preview did not decode.' },
  ).toBeGreaterThan(0);

  const contentHref = await uploadedRow.getByRole('link').getAttribute('href');
  expect(contentHref).toBeTruthy();
  const authenticatedContent = await page.request.get(contentHref);
  expect(authenticatedContent.status()).toBe(200);
  expect(authenticatedContent.headers()['cache-control']).toContain('private');
  expect(authenticatedContent.headers()['cache-control']).toContain('no-store');
  expect(authenticatedContent.headers()['x-content-type-options']).toBe('nosniff');

  const anonymousContext = await browser.newContext();
  try {
    const absoluteContentUrl = new URL(contentHref, page.url()).toString();
    const anonymousContent = await anonymousContext.request.get(absoluteContentUrl, { maxRedirects: 0 });
    expect(anonymousContent.status()).toBe(302);
    expect(anonymousContent.headers().location).toMatch(/\/login$/u);
  } finally {
    await anonymousContext.close();
  }

  const referencedAlt = `Referenced acceptance bridge ${Math.random().toString(16).slice(2, 10)}`;
  editorialMediaFixture('seed-referenced', manager.email, referencedAlt);
  await page.reload();
  const referencedRow = page.getByRole('row').filter({ hasText: referencedAlt });
  await expect(referencedRow).toBeVisible();
  await expect(referencedRow.getByText('In use', { exact: true })).toBeVisible();
  await expect(referencedRow.getByRole('button', { name: 'Delete' })).toHaveCount(0);

  uploadedRow = page.getByRole('row').filter({ hasText: uploadedAlt });
  const deleteButton = uploadedRow.getByRole('button', { name: 'Delete' });
  await deleteButton.scrollIntoViewIfNeeded();
  await deleteButton.focus();
  await expect(deleteButton).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('status')).toContainText('Editorial image deleted.');
  await expect(page.getByRole('row').filter({ hasText: uploadedAlt })).toHaveCount(0);
  await expect(page.getByRole('row').filter({ hasText: referencedAlt })).toBeVisible();

  await page.goto('/admin/audit');
  await expect(page.getByRole('heading', { name: 'Administrator audit' })).toBeVisible();
  await expect(page.getByText('editorial_media.uploaded').first()).toBeVisible();
  await expect(page.getByText('editorial_media.deleted').first()).toBeVisible();
  await expect(page.locator('body')).not.toContainText(uploadedAlt);
  await expect(page.locator('body')).not.toContainText(manager.password);
  await expect(page.locator('body')).not.toContainText(manager.recoveryCode);
  await assertAccessibilitySmoke(page);
  await assertNoPageOverflow(page);
  await evidenceScreenshot(page, `editorial-media-admin-audit-${test.info().project.name}`);

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});

test('@portal-editorial-media missing and integrity-failed stored objects render accessible preview fallbacks', async ({ page }) => {
  const manager = seedIdentity('editorial-media-fallback-manager', {
    confirmedMfa: true,
    permissions: ['media.manage'],
  });
  const missingAlt = `Missing Editorial Media ${Math.random().toString(16).slice(2, 10)}`;
  const corruptAlt = `Corrupt Editorial Media ${Math.random().toString(16).slice(2, 10)}`;
  const missing = editorialMediaFixture('seed-referenced', manager.email, missingAlt);
  const corrupt = editorialMediaFixture('seed-referenced', manager.email, corruptAlt);
  editorialMediaFixture('remove-files', String(missing.media_id));
  editorialMediaFixture('corrupt-files', String(corrupt.media_id));

  await signIn(page, manager);
  await page.goto('/admin/media');

  for (const altText of [missingAlt, corruptAlt]) {
    const row = page.getByRole('row').filter({ hasText: altText });
    await expect(row).toBeVisible();
    const fallback = row.getByRole('img', { name: altText });
    await expect(fallback).toBeVisible();
    await expect(fallback).toContainText(`Preview unavailable: ${altText}`);
    await expect(row.locator('img')).toHaveCount(0);
  }

  await expect(page.locator('[data-media-fallback-state="unavailable"]')).toHaveCount(2);
  await assertAccessibilitySmoke(page);
  await assertNoPageOverflow(page);
  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors.length).toBeGreaterThanOrEqual(1);
  expect(page.__acceptanceDiagnostics.serverErrors.every((entry) => (
    entry.status === 500
    && entry.url.endsWith(`/admin/media/${corrupt.media_id}/thumbnail`)
  ))).toBe(true);
  allowExpectedHttpFailure(page.__acceptanceDiagnostics, {
    status: 500,
    pathname: `/admin/media/${corrupt.media_id}/thumbnail`,
    count: page.__acceptanceDiagnostics.serverErrors.length,
  });
});
