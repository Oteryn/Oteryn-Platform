import { test, expect } from '@playwright/test';
import {
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

const password = 'AcceptanceSupportLegal!234';

function supportLegalFixture(...args) {
  return JSON.parse(runBinary('php', ['scripts/acceptance/seed-browser-support-legal.php', ...args]));
}

function seedIdentity(label, { confirmedMfa, permissions }) {
  const email = uniqueEmail(label);
  const recoveryCode = `SUP-${label.toUpperCase().replace(/[^A-Z0-9]/gu, '').slice(0, 14)}-01`;
  supportLegalFixture(
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

async function submitTranslationWithAccessibleControls(page) {
  const button = page.getByRole('button', { name: 'Save translation' });
  await button.scrollIntoViewIfNeeded();
  await button.focus();
  await expect(button).toBeFocused();
  await page.keyboard.press('Enter');
}

test.setTimeout(180_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  try {
    await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
  } finally {
    supportLegalFixture('reset');
  }
});

test('@portal-support-legal-public route-complete missing, unpublished, published, legal-version, approved-link and EN/PL isolation states', async ({ page }) => {
  const definitions = supportLegalFixture('describe').pages;
  supportLegalFixture('reset');

  for (const definition of definitions) {
    for (const path of [definition.legacy_path, definition.english_path]) {
      const response = await page.goto(path);
      expect(response?.status(), `Missing state for ${path}`).toBe(404);
      await expect(page.locator('main')).toContainText('has not been configured');
    }
  }

  const unpublished = supportLegalFixture('seed-unpublished').pages;
  for (const definition of unpublished) {
    for (const path of [definition.legacy_path, definition.english_path]) {
      const response = await page.goto(path);
      expect(response?.status(), `Unpublished state for ${path}`).toBe(404);
      await expect(page.locator('main')).toContainText('not currently published');
      await expect(page.locator('body')).not.toContainText(definition.draft_secret);
    }
  }

  const published = supportLegalFixture('seed-public').pages;
  for (const definition of published) {
    let response = await page.goto(definition.legacy_path);
    expect(response?.status(), `Legacy route ${definition.legacy_path}`).toBe(200);
    await expect(page.getByRole('heading', { level: 1, name: definition.english_title })).toBeVisible();

    response = await page.goto(definition.english_path);
    expect(response?.status(), `English route ${definition.english_path}`).toBe(200);
    await expect(page.getByRole('heading', { level: 1, name: definition.english_title })).toBeVisible();
    await expect(page.locator('.prose-text')).toContainText(definition.english_body);
    await expect(page.locator('article img')).toHaveCount(0);
    await expect(page.getByText(definition.polish_title)).toHaveCount(0);

    if (definition.legal) {
      await expect(page.locator('article > .muted')).toContainText(`Version ${definition.legal_version}`);
    }

    response = await page.goto(definition.polish_path);
    expect(response?.status(), `Polish route ${definition.polish_path}`).toBe(200);
    await expect(page.getByRole('heading', { level: 1, name: definition.polish_title })).toBeVisible();
    await expect(page.locator('.prose-text')).toContainText(definition.polish_body);
    await expect(page.getByText(definition.english_title)).toHaveCount(0);
  }

  const support = published.find((definition) => definition.key === 'support');
  expect(support).toBeTruthy();
  await page.goto(support.english_path);
  await expect(page.getByRole('heading', { name: 'Approved support channels' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Open', exact: true }).nth(0)).toHaveAttribute('href', 'https://discord.gg/oteryn-acceptance');
  await expect(page.locator('a[href="mailto:support-acceptance@example.test"]')).toBeVisible();
  await expect(page.locator('a[href="https://help.example.test/oteryn"]')).toBeVisible();
  await assertAccessibilitySmoke(page);
  await assertNoPageOverflow(page);
  await evidenceScreenshot(page, `support-legal-public-${test.info().project.name}`);

  const report = published.find((definition) => definition.key === 'report-a-bug');
  expect(report).toBeTruthy();
  await page.goto(report.english_path);
  await expect(page.locator('.notice')).toContainText('does not store a support ticket submission here');
  await expect(page.locator('article form')).toHaveCount(0);
  const postResponse = await page.request.post(report.english_path, {
    form: { email: 'player@example.test', description: 'must not persist' },
  });
  expect(postResponse.status()).toBe(405);

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});

test('@portal-support-legal-admin guest, MFA and exact permission boundaries fail closed', async ({ page }) => {
  supportLegalFixture('reset');

  await page.goto('/admin/support-content');
  await expect(page).toHaveURL(/\/login$/u);

  const noMfa = seedIdentity('support-legal-no-mfa', {
    confirmedMfa: false,
    permissions: ['support.content.manage'],
  });
  await signIn(page, noMfa);
  let response = await page.goto('/admin/support-content');
  expect(response?.status()).toBe(403);
  await expect(page.getByRole('heading', { name: 'You do not have access to this page' })).toBeVisible();
  await logout(page);

  const noPermission = seedIdentity('support-legal-no-permission', {
    confirmedMfa: true,
    permissions: [],
  });
  await signIn(page, noPermission);
  response = await page.goto('/admin/support-content');
  expect(response?.status()).toBe(403);
  await expect(page.getByRole('heading', { name: 'You do not have access to this page' })).toBeVisible();

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});

test('@portal-support-legal-admin administrator validation, legal publication, Polish translation and bounded audit lifecycle', async ({ page }) => {
  supportLegalFixture('reset');
  const administrator = seedIdentity('support-legal-administrator', {
    confirmedMfa: true,
    permissions: ['support.content.manage', 'audit.view'],
  });
  await signIn(page, administrator);

  await page.goto('/admin/support-content');
  await expect(page.getByRole('heading', { name: 'Support, rules and legal content' })).toBeVisible();
  await expect(page.getByRole('row')).toHaveCount(9);
  await assertAccessibilitySmoke(page);
  await assertNoPageOverflow(page);

  await page.goto('/admin/support-content/terms/edit');
  await expect(page.getByRole('heading', { name: 'Terms of Service' })).toBeVisible();
  const suffix = Math.random().toString(16).slice(2, 10);
  const englishTitle = `Acceptance Browser Terms ${suffix}`;
  const englishBody = 'English legal body that must not appear in bounded audit metadata.';
  await page.getByLabel('Title').fill(englishTitle);
  await page.getByLabel('Body (plain text)').fill(englishBody);
  await page.getByLabel('Publish at').fill('2000-01-01T00:00');
  await page.getByRole('button', { name: 'Save editorial page' }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page).toHaveURL(/\/admin\/support-content\/terms\/edit$/u);

  await page.getByLabel('Legal version').fill('1.0');
  await page.getByLabel('Effective date').fill('2026-07-01');
  await page.getByRole('button', { name: 'Save editorial page' }).click();
  await expect(page.getByRole('status')).toContainText('Editorial page created.');
  await expect(page.getByRole('heading', { name: 'Preserved published versions' })).toBeVisible();
  await expect(page.getByRole('cell', { name: '1.0', exact: true })).toBeVisible();

  let response = await page.goto('/en/legal/terms');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { level: 1, name: englishTitle })).toBeVisible();
  await expect(page.locator('.prose-text')).toContainText(englishBody);
  await expect(page.locator('article > .muted')).toContainText('Version 1.0');

  await page.goto('/admin/support-content');
  const termsRow = page.getByRole('row').filter({ hasText: 'Terms of Service' });
  await termsRow.getByRole('link', { name: 'Polish translation' }).click();
  await expect(page.getByRole('heading', { name: 'Polish translation' })).toBeVisible();
  await page.getByLabel('Polish title').fill('Akceptacyjny regulamin przeglądarkowy');
  await page.getByLabel('Polish content (plain text)').fill('Polska treść regulaminu przeglądarkowego.');
  await page.getByLabel('Publish Polish translation at (UTC)').fill('2000-01-01T00:00');
  await submitTranslationWithAccessibleControls(page);
  await expect(page.getByRole('status')).toContainText('Polish translation saved.');

  response = await page.goto('/pl/legal/terms');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { level: 1, name: 'Akceptacyjny regulamin przeglądarkowy' })).toBeVisible();
  await expect(page.getByText('Polska treść regulaminu przeglądarkowego.')).toBeVisible();
  await expect(page.getByText(englishTitle)).toHaveCount(0);

  await page.goto('/admin/audit');
  await expect(page.getByRole('heading', { name: 'Administrator audit' })).toBeVisible();
  await expect(page.getByText('support.content_created').first()).toBeVisible();
  await expect(page.locator('body')).not.toContainText(englishBody);
  await expect(page.locator('body')).not.toContainText(administrator.password);
  await expect(page.locator('body')).not.toContainText(administrator.recoveryCode);
  await assertAccessibilitySmoke(page);
  await assertNoPageOverflow(page);
  await evidenceScreenshot(page, `support-legal-admin-audit-${test.info().project.name}`);

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});
