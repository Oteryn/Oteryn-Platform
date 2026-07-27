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

const password = 'AcceptanceAnnouncements!234';

function announcementFixture(...args) {
  return JSON.parse(runBinary('php', ['scripts/acceptance/seed-browser-announcements.php', ...args]));
}

function restoreSharedHomepageFixture() {
  runBinary('php', ['scripts/acceptance/seed-homepage-navigation-seo.php']);
}

function seedIdentity(label, { confirmedMfa, permissions }) {
  const email = uniqueEmail(label);
  const recoveryCode = `ANN-${label.toUpperCase().replace(/[^A-Z0-9]/gu, '').slice(0, 14)}-01`;
  announcementFixture(
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

function utcInput(hoursFromNow) {
  return new Date(Date.now() + hoursFromNow * 60 * 60 * 1000).toISOString().slice(0, 16);
}

async function fillAnnouncementForm(page, {
  title,
  body,
  startsAt = utcInput(-1),
  endsAt = utcInput(2),
  state = 'published',
  severity = 'maintenance',
  actionLabel = 'Read announcement details',
  actionUrl = '/en/news',
}) {
  await page.getByLabel('Title').fill(title);
  await page.getByLabel('Body (plain text)').fill(body);
  await page.getByLabel('Severity').selectOption(severity);
  await page.getByLabel('Starts at (UTC)').fill(startsAt);
  await page.getByLabel('Ends at (UTC)').fill(endsAt);
  await page.getByLabel('Publication state').selectOption(state);
  await page.getByLabel('Action label').fill(actionLabel);
  await page.getByLabel('Action link').fill(actionUrl);
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
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  try {
    await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
  } finally {
    restoreSharedHomepageFixture();
  }
});

test('@portal-announcements guest, MFA and exact permission boundaries fail closed', async ({ page }) => {
  announcementFixture('reset');

  await page.goto('/admin/announcements');
  await expect(page).toHaveURL(/\/login$/u);

  const noMfa = seedIdentity('announcements-no-mfa', {
    confirmedMfa: false,
    permissions: ['portal.announcements.manage'],
  });
  await signIn(page, noMfa);
  let response = await page.goto('/admin/announcements');
  expect(response?.status()).toBe(403);
  await expect(page.getByRole('heading', { name: 'You do not have access to this page' })).toBeVisible();
  await logout(page);

  const noPermission = seedIdentity('announcements-no-permission', {
    confirmedMfa: true,
    permissions: [],
  });
  await signIn(page, noPermission);
  response = await page.goto('/admin/announcements');
  expect(response?.status()).toBe(403);
  await expect(page.getByRole('heading', { name: 'You do not have access to this page' })).toBeVisible();

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});

test('@portal-announcements administrator validation, publication, Polish translation, stale recovery, conflict and audit lifecycle', async ({ page }) => {
  announcementFixture('reset');
  const administrator = seedIdentity('announcements-administrator', {
    confirmedMfa: true,
    permissions: ['portal.announcements.manage', 'audit.view'],
  });
  await signIn(page, administrator);

  await page.goto('/admin/announcements');
  await expect(page.getByRole('heading', { name: 'Announcements' })).toBeVisible();
  await assertAccessibilitySmoke(page);
  await assertNoPageOverflow(page);
  await page.getByRole('link', { name: 'Create announcement' }).click();
  await expect(page.getByRole('heading', { name: 'Create announcement' })).toBeVisible();

  const suffix = Math.random().toString(16).slice(2, 10);
  const englishTitle = `Acceptance Browser Announcement ${suffix}`;
  const englishBody = 'English announcement body that must not appear in audit metadata.';
  await fillAnnouncementForm(page, {
    title: englishTitle,
    body: englishBody,
    actionUrl: 'javascript:alert(1)',
  });
  await page.getByRole('button', { name: 'Save announcement' }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page).toHaveURL(/\/admin\/announcements\/create$/u);

  await page.getByLabel('Action link').fill('/en/news');
  await page.getByRole('button', { name: 'Save announcement' }).click();
  await expect(page.getByRole('status')).toContainText('Announcement saved.');
  await expect(page).toHaveURL(/\/admin\/announcements\/\d+\/edit$/u);
  const editUrl = page.url();
  const announcementId = editUrl.match(/\/admin\/announcements\/(\d+)\/edit$/u)?.[1];
  expect(announcementId).toBeTruthy();

  let response = await page.goto('/en');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { name: englishTitle })).toBeVisible();
  await expect(page.getByText(englishBody)).toBeVisible();

  await page.goto(`/admin/announcements/${announcementId}/translations/pl`);
  await expect(page.getByRole('heading', { name: 'Polish translation' })).toBeVisible();
  await page.getByLabel('Polish title').fill('Polski komunikat przeglądarkowy');
  await page.getByLabel('Polish content (plain text)').fill('Polska treść komunikatu.');
  await page.getByLabel('Polish action label').fill('Przeczytaj komunikat');
  await page.getByLabel('Publish Polish translation at (UTC)').fill('2000-01-01T00:00');
  await page.getByRole('button', { name: 'Save translation' }).click();
  await expect(page.getByRole('status')).toContainText('Polish translation saved.');

  response = await page.goto('/pl');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Polski komunikat przeglądarkowy' })).toBeVisible();
  await expect(page.getByText('Polska treść komunikatu.')).toBeVisible();
  await expect(page.getByText(englishTitle)).toHaveCount(0);

  await page.goto(editUrl);
  await page.getByLabel('Title').fill(`${englishTitle} updated`);
  await page.getByRole('button', { name: 'Save announcement' }).click();
  await expect(page.getByRole('status')).toContainText('Announcement saved.');

  response = await page.goto('/pl');
  expect(response?.status()).toBe(200);
  await expect(page.getByText('Polski komunikat przeglądarkowy')).toHaveCount(0);
  await expect(page.getByText(`${englishTitle} updated`)).toHaveCount(0);
  await expect(page.getByText('Brak aktywnych ogłoszeń.')).toBeVisible();

  await page.goto(`/admin/announcements/${announcementId}/translations/pl`);
  await expect(page.getByText('The source changed after this translation was reviewed.')).toBeVisible();
  await page.getByRole('button', { name: 'Save translation' }).click();
  await expect(page.getByRole('status')).toContainText('Polish translation saved.');

  response = await page.goto('/pl');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Polski komunikat przeglądarkowy' })).toBeVisible();

  await page.goto(editUrl);
  await page.getByLabel('Title').fill('Stale browser announcement edit must fail');
  announcementFixture('bump-lock', announcementId);
  const [conflictResponse] = await Promise.all([
    page.waitForResponse((candidate) => candidate.url().includes('/admin/announcements/') && candidate.status() === 409),
    page.getByRole('button', { name: 'Save announcement' }).click(),
  ]);
  expect(conflictResponse.status()).toBe(409);
  await expect(page.locator('body')).not.toContainText('SQLSTATE');
  await expect(page.locator('body')).not.toContainText(administrator.password);
  await expect(page.locator('body')).not.toContainText(administrator.recoveryCode);

  await page.goto('/admin/audit');
  await expect(page.getByRole('heading', { name: 'Administrator audit' })).toBeVisible();
  await expect(page.getByText('portal.announcement_created').first()).toBeVisible();
  await expect(page.getByText('portal.announcement_updated').first()).toBeVisible();
  await expect(page.locator('body')).not.toContainText(englishBody);
  await assertAccessibilitySmoke(page);
  await assertNoPageOverflow(page);
  await evidenceScreenshot(page, `announcements-admin-audit-${test.info().project.name}`);

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});
