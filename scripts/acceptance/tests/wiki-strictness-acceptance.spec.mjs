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

const password = 'AcceptanceWikiStrictness!234';
const evidenceMarker = '@portal-wiki-admin-strictness not-found csrf-419 server-failure recovery';
const adminIndexView = path.join(repoRoot, 'resources/views/admin/wiki/index.blade.php');
const unavailableAdminIndexView = `${adminIndexView}.strictness-unavailable`;

function wikiFixture(...args) {
  return JSON.parse(runBinary('php', ['scripts/acceptance/seed-browser-wiki-reconciliation.php', ...args]));
}

function seedPublisher() {
  const email = uniqueEmail('wiki-strictness-publisher');
  const recoveryCode = 'WKR-STRICTNESS-01';
  wikiFixture(
    'seed-identity',
    email,
    password,
    recoveryCode,
    'confirmed',
    'wiki.access,wiki.articles.manage,wiki.categories.manage,wiki.publish',
  );
  return { email, password, recoveryCode };
}

async function signIn(page, identity) {
  await login(page, identity.email, identity.password);
  await completeMfaChallenge(page, identity.recoveryCode);
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
  wikiFixture('reset');
  runArtisan('cache:clear');
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  try {
    restoreAdminIndexView();
    await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
  } finally {
    wikiFixture('reset');
  }
});

// Evidence marker: @portal-wiki-admin-strictness not-found csrf-419 server-failure recovery
test(`${evidenceMarker}`, async ({ page }) => {
  const publisher = seedPublisher();
  await signIn(page, publisher);

  let response = await page.goto('/admin/wiki/articles/999999999/edit');
  expect(response?.status()).toBe(404);

  const csrfResponse = await page.request.post('/admin/wiki/articles', { form: {} });
  expect(csrfResponse.status()).toBe(419);

  response = await page.goto('/admin/wiki');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Wiki administration' })).toBeVisible();

  try {
    fs.renameSync(adminIndexView, unavailableAdminIndexView);
    runArtisan('view:clear');

    response = await page.goto('/admin/wiki', { waitUntil: 'domcontentloaded' });
    expect(response?.status()).toBe(500);
    await expect(page.locator('.error-code')).toHaveText('500');
  } finally {
    restoreAdminIndexView();
  }

  page.__acceptanceDiagnostics.serverErrors = [];
  response = await page.goto('/admin/wiki');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Wiki administration' })).toBeVisible();
});
