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

const password = 'AcceptanceSupportModeration!234';

function fixture(...args) {
  return JSON.parse(runBinary('php', ['scripts/acceptance/seed-browser-support-moderation.php', ...args]));
}

function seedIdentity(label, { confirmedMfa = false, permissions = [] } = {}) {
  const email = uniqueEmail(label);
  const recoveryCodes = [1, 2].map(
    (sequence) => `MOD-${label.toUpperCase().replace(/[^A-Z0-9]/gu, '').slice(0, 14)}-${String(sequence).padStart(2, '0')}`,
  );
  const result = fixture(
    'seed-identity',
    email,
    password,
    recoveryCodes.join(','),
    confirmedMfa ? 'confirmed' : 'unconfirmed',
    permissions.join(','),
  );

  return { ...result, password, recoveryCodes, nextRecoveryCode: 0, confirmedMfa };
}

async function signIn(page, identity) {
  await page.goto('/login?locale=en');
  await login(page, identity.email, identity.password);
  if (identity.confirmedMfa) {
    const recoveryCode = identity.recoveryCodes[identity.nextRecoveryCode];
    expect(recoveryCode, `No unused MFA recovery code remains for ${identity.email}`).toBeTruthy();
    await completeMfaChallenge(page, recoveryCode);
    identity.nextRecoveryCode += 1;
  } else {
    await page.waitForURL((url) => url.pathname !== '/login');
    await page.waitForLoadState('domcontentloaded');
  }
}

async function assertNoOverflow(page) {
  const dimensions = await page.evaluate(() => ({
    viewport: window.innerWidth,
    document: document.documentElement.scrollWidth,
  }));
  expect(dimensions.document, `Unexpected horizontal overflow on ${page.url()}`).toBeLessThanOrEqual(dimensions.viewport + 1);
}

async function chooseOption(page, label, value) {
  await page.getByLabel(label).selectOption(value);
}

test.setTimeout(240_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  fixture('reset');
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  try {
    await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
  } finally {
    fixture('reset');
  }
});

test('@portal-support-moderation guest, MFA, exact-permission and object-ownership boundaries fail closed', async ({ page }) => {
  await page.goto('/support/tickets?locale=pl');
  await expect(page).toHaveURL(/\/login$/u);
  await expect(page.getByRole('heading', { name: 'Zaloguj się do Oteryn Platform' })).toBeVisible();

  await page.goto('/admin/support/tickets');
  await expect(page).toHaveURL(/\/login/u);
  await page.context().clearCookies();

  const noMfa = seedIdentity('support-moderation-no-mfa', {
    confirmedMfa: false,
    permissions: ['support.tickets.manage', 'support.reports.manage', 'support.enforcement.manage'],
  });
  await signIn(page, noMfa);
  allowExpectedHttpFailure(page.__acceptanceDiagnostics, { status: 403, pathname: '/admin/support/tickets' });
  let response = await page.goto('/admin/support/tickets');
  expect(response?.status()).toBe(403);
  await logout(page);

  const noPermission = seedIdentity('support-moderation-no-permission', { confirmedMfa: true });
  await signIn(page, noPermission);
  allowExpectedHttpFailure(page.__acceptanceDiagnostics, { status: 403, pathname: '/admin/moderation/reports' });
  response = await page.goto('/admin/moderation/reports');
  expect(response?.status()).toBe(403);
  await logout(page);

  const owner = seedIdentity('support-moderation-owner');
  await signIn(page, owner);
  await page.goto('/support/tickets/create');
  await chooseOption(page, 'Category', 'account');
  await page.getByLabel('Subject').fill('Ownership acceptance ticket');
  await page.getByLabel('Initial message').fill('Only the owner may read this conversation.');
  await page.getByRole('button', { name: 'Open ticket' }).click();
  const ownerTicketPath = new URL(page.url()).pathname;
  await logout(page);

  const other = seedIdentity('support-moderation-other');
  await signIn(page, other);
  allowExpectedHttpFailure(page.__acceptanceDiagnostics, { status: 404, pathname: ownerTicketPath });
  response = await page.goto(ownerTicketPath);
  expect(response?.status()).toBe(404);

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});

test('@portal-support-moderation complete user, moderator, notification, privacy, localization and responsive lifecycle', async ({ page }) => {
  const suffix = Math.random().toString(16).slice(2, 10);
  const ticketSubject = `Acceptance ticket ${suffix}`;
  const publicTicketReply = `Public ticket reply ${suffix}`;
  const privateTicketNote = `PRIVATE-TICKET-NOTE-${suffix}`;
  const reportTarget = `Acceptance Player ${suffix}`;
  const reportEvidence = `PRIVATE-REPORT-EVIDENCE-${suffix}`;
  const reportOutcome = `Public report outcome ${suffix}`;
  const privateReportNote = `PRIVATE-REPORT-NOTE-${suffix}`;
  const enforcementReason = `Public enforcement reason ${suffix}`;
  const privateEnforcementNote = `PRIVATE-ENFORCEMENT-NOTE-${suffix}`;
  const appealMessage = `Appeal evidence ${suffix}`;
  const appealOutcome = `Appeal accepted outcome ${suffix}`;

  const user = seedIdentity(`support-user-${suffix}`);
  const moderator = seedIdentity(`support-moderator-${suffix}`, {
    confirmedMfa: true,
    permissions: [
      'support.tickets.manage',
      'support.reports.manage',
      'support.enforcement.manage',
      'audit.view',
    ],
  });

  await signIn(page, user);
  await page.goto('/support/tickets?locale=en');
  await expect(page.getByRole('heading', { name: 'Support tickets' })).toBeVisible();
  await expect(page.getByText('You have no support tickets.')).toBeVisible();
  await assertAccessibilitySmoke(page);
  await assertNoOverflow(page);

  await page.getByRole('link', { name: 'Open ticket' }).click();
  await expect(page.getByText('Attachments are not accepted.')).toBeVisible();
  await chooseOption(page, 'Category', 'technical');
  await page.getByLabel('Subject').fill(ticketSubject);
  await page.getByLabel('Initial message').fill(`Initial ticket message ${suffix}`);
  await page.getByRole('button', { name: 'Open ticket' }).click();
  await expect(page.getByRole('heading', { name: ticketSubject })).toBeVisible();
  const ticketPath = new URL(page.url()).pathname;

  await page.getByRole('link', { name: 'Reports' }).click();
  await expect(page.getByText('You have not submitted any reports.')).toBeVisible();
  await page.getByRole('link', { name: 'Submit report' }).click();
  await chooseOption(page, 'Report type', 'player');
  await chooseOption(page, 'Category', 'cheating');
  await page.getByLabel('Target').fill(reportTarget);
  await page.getByLabel('Evidence summary').fill(reportEvidence);
  await page.getByRole('button', { name: 'Submit report' }).click();
  await expect(page.getByText(reportTarget)).toBeVisible();
  const reportPath = new URL(page.url()).pathname;

  await page.getByRole('link', { name: 'Rules and penalties' }).click();
  await expect(page.getByText('There are no warnings or penalties on this account.')).toBeVisible();
  await assertAccessibilitySmoke(page);
  await assertNoOverflow(page);
  await evidenceScreenshot(page, `support-user-empty-${test.info().project.name}`);
  await logout(page);

  await signIn(page, moderator);
  await page.goto('/admin/support/tickets');
  const ticketRow = page.getByRole('row').filter({ hasText: ticketSubject });
  await expect(ticketRow).toBeVisible();
  await ticketRow.getByRole('link', { name: 'View' }).click();
  await page.getByLabel('Reply').fill(publicTicketReply);
  await page.getByRole('button', { name: 'Reply' }).click();
  await expect(page.getByRole('status')).toContainText('reply was saved');
  await page.getByLabel('Reply').fill(privateTicketNote);
  await page.getByLabel('Internal moderator note').check();
  await page.getByRole('button', { name: 'Reply' }).click();
  await expect(page.getByText('Private moderator note')).toBeVisible();

  await page.goto('/admin/moderation/reports');
  const reportRow = page.getByRole('row').filter({ hasText: reportTarget });
  await expect(reportRow).toBeVisible();
  await reportRow.getByRole('link', { name: 'View' }).click();
  await chooseOption(page, 'Status', 'actioned');
  await page.getByLabel('Public-safe outcome').fill(reportOutcome);
  await page.getByLabel('Private moderator notes').fill(privateReportNote);
  await page.getByRole('button', { name: 'Save' }).click();
  await expect(page.getByRole('status')).toContainText('report outcome was saved');

  await page.goto('/admin/moderation/enforcement/create');
  await page.getByLabel('Identity ID').fill(String(user.identity_id));
  await chooseOption(page, 'Category', 'warning');
  await chooseOption(page, 'Status', 'active');
  await page.getByLabel('Reason').fill(enforcementReason);
  await page.getByLabel('Private moderator notes').fill(privateEnforcementNote);
  await page.getByLabel('Effective at').fill('2026-07-01T00:00');
  await page.getByLabel('Expires at').fill('2026-08-01T00:00');
  await page.getByRole('button', { name: 'Save' }).click();
  await expect(page.getByRole('status')).toContainText('enforcement record was created');
  await assertAccessibilitySmoke(page);
  await assertNoOverflow(page);
  await evidenceScreenshot(page, `support-admin-enforcement-${test.info().project.name}`);
  await logout(page);

  await signIn(page, user);
  await page.goto(ticketPath);
  await expect(page.getByText(publicTicketReply)).toBeVisible();
  await expect(page.getByText(privateTicketNote)).toHaveCount(0);

  await page.goto(reportPath);
  await expect(page.getByText(reportOutcome)).toBeVisible();
  await expect(page.getByText(privateReportNote)).toHaveCount(0);

  await page.goto('/support/enforcement?locale=pl');
  await expect(page.getByRole('heading', { name: 'Ostrzeżenia i kary' })).toBeVisible();
  await page.getByRole('link', { name: 'Zobacz' }).click();
  await expect(page.getByText(enforcementReason)).toBeVisible();
  await expect(page.getByText(privateEnforcementNote)).toHaveCount(0);
  const enforcementPath = new URL(page.url()).pathname;
  await page.getByRole('button', { name: 'Potwierdź zapoznanie' }).click();
  await page.getByLabel('Treść odwołania').fill(appealMessage);
  await page.getByRole('button', { name: 'Odwołaj się' }).click();
  await expect(page.getByRole('status')).toContainText('Odwołanie zostało wysłane');
  await assertAccessibilitySmoke(page);
  await assertNoOverflow(page);
  await evidenceScreenshot(page, `support-user-enforcement-${test.info().project.name}`);
  await logout(page);

  await signIn(page, moderator);
  await page.goto('/admin/moderation/enforcement?locale=en');
  const enforcementRow = page.getByRole('row').filter({ hasText: `#${user.identity_id}` });
  await enforcementRow.getByRole('link', { name: 'View' }).click();
  await chooseOption(page, 'Appeal status', 'accepted');
  await page.getByLabel('Appeal outcome').fill(appealOutcome);
  await page.getByRole('button', { name: 'Save' }).click();
  await expect(page.getByRole('status')).toContainText('appeal state was updated');
  await logout(page);

  await signIn(page, user);
  await page.goto(`${enforcementPath}?locale=en`);
  await expect(page.getByText(appealOutcome)).toBeVisible();
  await expect(page.getByText(privateEnforcementNote)).toHaveCount(0);
  await expect(page.getByText('Revoked', { exact: true })).toBeVisible();

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});
