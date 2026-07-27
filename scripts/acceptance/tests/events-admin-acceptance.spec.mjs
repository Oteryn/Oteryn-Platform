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

const password = 'AcceptanceEvents!234';

function eventFixture(...args) {
  return JSON.parse(runBinary('php', ['scripts/acceptance/seed-browser-events.php', ...args]));
}

function seedIdentity(label, { confirmedMfa, permissions }) {
  const email = uniqueEmail(label);
  const recoveryCode = `EVENT-${label.toUpperCase().replace(/[^A-Z0-9]/gu, '').slice(0, 12)}-01`;
  eventFixture(
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

async function fillEventForm(page, {
  startsAt,
  endsAt,
  englishTitle,
  englishSlug,
  englishSummary = 'Acceptance event summary.',
  englishBody = 'Acceptance event body.',
  polishTitle = '',
  polishSlug = '',
  polishSummary = '',
  polishBody = '',
}) {
  await page.getByLabel('Starts at (UTC)').fill(startsAt);
  await page.getByLabel('Ends at (UTC)').fill(endsAt);
  await page.getByLabel('Featured event').check();

  const english = page.getByRole('group', { name: 'English translation (required)' });
  await english.getByLabel('Title').fill(englishTitle);
  await english.getByLabel('Slug').fill(englishSlug);
  await english.getByLabel('Summary').fill(englishSummary);
  await english.getByLabel('Body (plain text)').fill(englishBody);

  const polish = page.getByRole('group', { name: /Polish translation/u });
  await polish.getByLabel('Title').fill(polishTitle);
  await polish.getByLabel('Slug').fill(polishSlug);
  await polish.getByLabel('Summary').fill(polishSummary);
  await polish.getByLabel('Body (plain text)').fill(polishBody);
}

test.setTimeout(180_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@portal-events guest, MFA and exact manage/publish permission boundaries fail closed', async ({ page }) => {
  eventFixture('reset');

  await page.goto('/admin/events');
  await expect(page).toHaveURL(/\/login$/u);

  const noMfa = seedIdentity('events-no-mfa', {
    confirmedMfa: false,
    permissions: ['events.manage', 'events.publish'],
  });
  await signIn(page, noMfa);
  let response = await page.goto('/admin/events');
  expect(response?.status()).toBe(403);
  await expect(page.getByRole('heading', { name: 'You do not have access to this page' })).toBeVisible();
  await logout(page);

  const noPermission = seedIdentity('events-no-permission', {
    confirmedMfa: true,
    permissions: [],
  });
  await signIn(page, noPermission);
  response = await page.goto('/admin/events');
  expect(response?.status()).toBe(403);
  await expect(page.getByRole('heading', { name: 'You do not have access to this page' })).toBeVisible();
  await logout(page);

  const manageOnly = seedIdentity('events-manage-only', {
    confirmedMfa: true,
    permissions: ['events.manage'],
  });
  await signIn(page, manageOnly);
  response = await page.goto('/admin/events');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Events' })).toBeVisible();
  await page.getByRole('link', { name: 'Create event' }).click();

  const slug = `events-manage-only-${Date.now()}`;
  await fillEventForm(page, {
    startsAt: utcInput(12),
    endsAt: utcInput(14),
    englishTitle: 'Manage-only acceptance event',
    englishSlug: slug,
  });
  await page.getByRole('button', { name: 'Save event draft' }).click();
  await expect(page.getByRole('status')).toContainText('Event draft saved.');
  await expect(page.getByRole('heading', { name: 'Publication state' })).toHaveCount(0);

  const eventId = page.url().match(/\/admin\/events\/(\d+)\/edit$/u)?.[1];
  expect(eventId).toBeTruthy();
  const csrf = await page.locator('input[name="_token"]').first().inputValue();
  const lockVersion = await page.locator('input[name="lock_version"]').first().inputValue();
  const deniedStatusChange = await page.request.put(`/admin/events/${eventId}/status`, {
    form: {
      _token: csrf,
      status: 'scheduled',
      lock_version: lockVersion,
    },
    maxRedirects: 0,
  });
  expect(deniedStatusChange.status()).toBe(403);
});

test('@portal-events administrator validation, create, publish, edit-to-draft, stale conflict and audit lifecycle', async ({ page }) => {
  eventFixture('reset');
  const administrator = seedIdentity('events-administrator', {
    confirmedMfa: true,
    permissions: ['events.manage', 'events.publish', 'audit.view'],
  });
  await signIn(page, administrator);

  await page.goto('/admin/events');
  await expect(page.getByRole('heading', { name: 'Events' })).toBeVisible();
  await assertAccessibilitySmoke(page);
  await page.getByRole('link', { name: 'Create event' }).click();
  await assertAccessibilitySmoke(page);

  const run = (process.env.ACCEPTANCE_RUN_ID ?? 'local').replace(/[^a-zA-Z0-9-]/gu, '-').toLowerCase();
  const suffix = Math.random().toString(16).slice(2, 10);
  const englishSlug = `events-${run}-${suffix}`.slice(0, 150);
  const polishSlug = `wydarzenie-${run}-${suffix}`.slice(0, 150);

  await fillEventForm(page, {
    startsAt: utcInput(8),
    endsAt: utcInput(7),
    englishTitle: 'Invalid acceptance event',
    englishSlug,
  });
  await page.getByRole('button', { name: 'Save event draft' }).click();
  await expect(page.getByRole('alert')).toBeVisible();

  await fillEventForm(page, {
    startsAt: utcInput(8),
    endsAt: utcInput(10),
    englishTitle: 'Acceptance Browser Event',
    englishSlug,
    englishSummary: 'English browser event summary.',
    englishBody: 'English browser event body.',
    polishTitle: 'Wydarzenie przeglądarkowe',
    polishSlug,
    polishSummary: 'Polskie podsumowanie wydarzenia.',
    polishBody: 'Polska treść wydarzenia.',
  });
  await page.getByRole('button', { name: 'Save event draft' }).click();
  await expect(page.getByRole('status')).toContainText('Event draft saved.');
  const editUrl = page.url();
  await expect(page.getByRole('heading', { name: 'Publication state' })).toBeVisible();

  let response = await page.goto(`/events/${englishSlug}`);
  expect(response?.status()).toBe(404);

  await page.goto(editUrl);
  await page.getByLabel('State').selectOption('scheduled');
  await page.getByRole('button', { name: 'Change publication state' }).click();
  await expect(page.getByRole('status')).toContainText('Event publication state changed.');

  response = await page.goto(`/events/${englishSlug}`);
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Acceptance Browser Event' })).toBeVisible();
  response = await page.goto(`/pl/events/${polishSlug}`);
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Wydarzenie przeglądarkowe' })).toBeVisible();

  await page.goto(editUrl);
  const english = page.getByRole('group', { name: 'English translation (required)' });
  await english.getByLabel('Title').fill('Acceptance Browser Event Updated');
  await page.getByRole('button', { name: 'Save event draft' }).click();
  await expect(page.getByRole('status')).toContainText('Publication approval is required again.');

  response = await page.goto(`/events/${englishSlug}`);
  expect(response?.status()).toBe(404);

  await page.goto(editUrl);
  await page.getByLabel('State').selectOption('scheduled');
  await page.getByRole('button', { name: 'Change publication state' }).click();
  await expect(page.getByRole('status')).toContainText('Event publication state changed.');

  await english.getByLabel('Title').fill('Stale browser edit must fail');
  eventFixture('bump-lock', englishSlug);
  const [conflictResponse] = await Promise.all([
    page.waitForResponse((candidate) => candidate.url().includes('/admin/events/') && candidate.status() === 409),
    page.getByRole('button', { name: 'Save event draft' }).click(),
  ]);
  expect(conflictResponse.status()).toBe(409);
  await expect(page.locator('body')).not.toContainText('SQLSTATE');
  await expect(page.locator('body')).not.toContainText(administrator.password);
  await expect(page.locator('body')).not.toContainText(administrator.recoveryCode);

  await page.goto('/admin/audit');
  await expect(page.getByRole('heading', { name: 'Administrator audit' })).toBeVisible();
  await expect(page.getByText('events.event_created').first()).toBeVisible();
  await expect(page.getByText('events.event_updated').first()).toBeVisible();
  await expect(page.getByText('events.status_changed').first()).toBeVisible();
  await expect(page.locator('body')).not.toContainText('English browser event body.');
  await evidenceScreenshot(page, `events-admin-audit-${test.info().project.name}`);
});
