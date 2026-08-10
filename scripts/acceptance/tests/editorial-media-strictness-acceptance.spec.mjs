import fs from 'node:fs';
import path from 'node:path';
import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  completeMfaChallenge,
  installDiagnostics,
  login,
  repoRoot,
  runArtisan,
  runBinary,
  uniqueEmail,
} from './helpers.mjs';

const password = 'AcceptanceEditorialStrictness!234';
const recoveryCode = 'EDITORIAL-STRICTNESS-01';
const evidenceMarker = '@portal-editorial-media-strictness not-found csrf-419 server-failure recovery';
const adminIndexView = path.join(repoRoot, 'resources/views/admin/media/index.blade.php');
const unavailableAdminIndexView = `${adminIndexView}.strictness-unavailable`;

function editorialMediaFixture(...args) {
  return JSON.parse(runBinary('php', ['scripts/acceptance/seed-browser-editorial-media.php', ...args]));
}

function seedManager() {
  const email = uniqueEmail('editorial-strictness-manager');
  editorialMediaFixture(
    'seed-identity',
    email,
    password,
    recoveryCode,
    'confirmed',
    'media.manage',
  );
  return { email, password, recoveryCode };
}

function restoreAdminIndexView() {
  if (fs.existsSync(unavailableAdminIndexView) && !fs.existsSync(adminIndexView)) {
    fs.renameSync(unavailableAdminIndexView, adminIndexView);
  } else if (fs.existsSync(unavailableAdminIndexView) && fs.existsSync(adminIndexView)) {
    fs.rmSync(unavailableAdminIndexView, { force: true });
  }
  runArtisan('view:clear');
}

test.setTimeout(180_000);
test.describe.configure({ retries: 0, mode: 'serial' });

test.beforeEach(async ({ page }) => {
  restoreAdminIndexView();
  editorialMediaFixture('reset');
  runArtisan('cache:clear');
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  try {
    restoreAdminIndexView();
    await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
  } finally {
    editorialMediaFixture('reset');
  }
});

// Evidence marker: @portal-editorial-media-strictness not-found csrf-419 server-failure recovery
test(`${evidenceMarker}`, async ({ page }) => {
  const manager = seedManager();
  await login(page, manager.email, manager.password);
  await completeMfaChallenge(page, manager.recoveryCode);

  let response = await page.goto('/admin/media/999999999/content');
  expect(response?.status()).toBe(404);

  const csrfResponse = await page.request.post('/admin/media', { form: {} });
  expect(csrfResponse.status()).toBe(419);

  response = await page.goto('/admin/media');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Editorial image library' })).toBeVisible();

  try {
    fs.renameSync(adminIndexView, unavailableAdminIndexView);
    runArtisan('view:clear');

    response = await page.goto('/admin/media', { waitUntil: 'domcontentloaded' });
    expect(response?.status()).toBe(500);
    await expect(page.locator('.error-code')).toHaveText('500');
  } finally {
    restoreAdminIndexView();
  }

  page.__acceptanceDiagnostics.serverErrors = [];
  response = await page.goto('/admin/media');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Editorial image library' })).toBeVisible();
});
