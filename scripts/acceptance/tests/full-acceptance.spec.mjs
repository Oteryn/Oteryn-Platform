import { test, expect } from '@playwright/test';
import {
  allowExpectedHttpFailure,
  assertAccessibilitySmoke,
  attachDiagnostics,
  completeMfaChallenge,
  enrollMfa,
  evidenceScreenshot,
  installDiagnostics,
  login,
  logout,
  register,
  runArtisan,
  runBinary,
  runPhpState,
  uniqueCharacterName,
  uniqueEmail,
  waitForDifferentTotp,
  waitForResetLink,
} from './helpers.mjs';

const primaryEmail = uniqueEmail('player');
let primaryPassword = 'AcceptancePassword!234';
const changedPassword = 'AcceptanceChanged!567';
const rotatedPassword = 'AcceptanceRotated!890';
const editorEmail = uniqueEmail('editor');
const editorPassword = 'AcceptanceEditor!234';
const foreignEmail = uniqueEmail('foreign');
const foreignPassword = 'AcceptanceForeign!234';

let primaryMfa;
let editorMfa;
let primaryLastTotp;
let editorLastTotp;
let primaryCharacter;
const newsSlug = `acceptance-news-${(process.env.ACCEPTANCE_RUN_ID ?? 'local').replace(/[^a-zA-Z0-9-]/gu, '-').toLowerCase()}`;
const pageSlug = `acceptance-page-${(process.env.ACCEPTANCE_RUN_ID ?? 'local').replace(/[^a-zA-Z0-9-]/gu, '-').toLowerCase()}`;

test.setTimeout(120_000);

async function createCharacter(page, name) {
  await page.goto('/account/characters/create');
  await page.getByLabel('Character name').fill(name);
  await page.getByLabel('Vocation').selectOption('4');
  await page.getByLabel('Sex').selectOption('1');
  await page.getByRole('button', { name: 'Create character' }).click();
}

async function signInWithPrimaryMfa(page) {
  await login(page, primaryEmail, primaryPassword);
  const code = await waitForDifferentTotp(primaryMfa.secret, primaryLastTotp);
  await completeMfaChallenge(page, code);
  primaryLastTotp = code;
  await expect(page).toHaveURL(/\/$/u);
}

async function signInWithEditorMfa(page) {
  await login(page, editorEmail, editorPassword);
  const code = await waitForDifferentTotp(editorMfa.secret, editorLastTotp);
  await completeMfaChallenge(page, code);
  editorLastTotp = code;
  await expect(page).toHaveURL(/\/$/u);
}

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);

  if (testInfo.status !== testInfo.expectedStatus && !page.isClosed()) {
    const screenshot = await page.screenshot({
      fullPage: true,
      mask: [
        page.locator('input'),
        page.locator('textarea'),
        page.locator('code'),
      ],
    });
    await testInfo.attach('sanitized-failure-screenshot', {
      body: screenshot,
      contentType: 'image/png',
    });
  }
});

test.describe('full production-like acceptance', () => {
  test.describe.configure({ mode: 'serial', retries: 0 });

  test('Flow 1 — new player: registration -> login -> MFA -> provisioning/binding -> character -> public lookup', async ({ page }) => {
    await register(page, primaryEmail, primaryPassword);

    const binding = runPhpState('binding', primaryEmail);
    expect(binding.status).toBe('ready');
    expect(binding.canary_account_id).toBeGreaterThan(0);

    await login(page, primaryEmail, primaryPassword);
    await expect(page).toHaveURL(/\/$/u);

    primaryMfa = await enrollMfa(page, primaryPassword);
    primaryLastTotp = primaryMfa.enrollmentCode;

    primaryCharacter = uniqueCharacterName('Hero');
    await createCharacter(page, primaryCharacter);
    await expect(page.getByRole('status')).toContainText(`Character ${primaryCharacter} created.`);

    const owner = runPhpState('character-owner', primaryCharacter, primaryEmail);
    expect(owner.canary_account_id).toBe(binding.canary_account_id);

    await page.goto(`/characters/${encodeURIComponent(primaryCharacter)}`);
    await expect(page.getByRole('heading', { name: primaryCharacter })).toBeVisible();
    await expect(page.locator('dt').filter({ hasText: 'Level' })).toBeVisible();
    await expect(page.locator('dd').filter({ hasText: '8' }).first()).toBeVisible();
    await expect(page.locator('dt').filter({ hasText: 'Vocation ID' })).toBeVisible();
    await expect(page.locator('dd').filter({ hasText: '4' }).first()).toBeVisible();
  });

  test('Flow 2 — returning player: login -> MFA challenge -> protected surface -> logout', async ({ page }) => {
    await login(page, primaryEmail, primaryPassword);
    await expect(page.getByRole('heading', { name: 'Complete your sign in' })).toBeVisible();

    const code = await waitForDifferentTotp(primaryMfa.secret, primaryLastTotp);
    await completeMfaChallenge(page, code);
    primaryLastTotp = code;
    await expect(page).toHaveURL(/\/$/u);

    await page.goto('/account/characters/create');
    await expect(page.getByRole('heading', { name: 'Create a character' })).toBeVisible();

    await logout(page);
    await page.goto('/account/characters/create');
    await expect(page).toHaveURL(/\/login$/u);
  });

  test('Flow 3 — password recovery uses real test SMTP, revokes old sessions and rejects token replay', async ({ browser, page }) => {
    await signInWithPrimaryMfa(page);
    await page.goto('/mfa');
    await expect(page.getByText('MFA is enabled for your Oteryn Platform web sign in.')).toBeVisible();

    const resetContext = await browser.newContext();
    const resetPage = await resetContext.newPage();
    try {
      await resetPage.goto('/forgot-password');
      await resetPage.getByLabel('Email').fill(primaryEmail);
      await resetPage.getByRole('button', { name: 'Send reset link' }).click();
      await expect(resetPage.getByRole('status')).toBeVisible();

      const resetLink = await waitForResetLink(primaryEmail);
      await resetPage.goto(resetLink);
      await resetPage.getByLabel('New password', { exact: true }).fill(changedPassword);
      await resetPage.getByLabel('Confirm new password').fill(changedPassword);
      await resetPage.getByRole('button', { name: 'Reset password' }).click();
      await expect(resetPage.getByRole('status')).toContainText('Your password has been reset. Sign in again.');

      await page.goto('/mfa');
      await expect(page).toHaveURL(/\/login$/u);

      await resetPage.getByLabel('Email').fill(primaryEmail);
      await resetPage.getByLabel('Password').fill(primaryPassword);
      await resetPage.getByRole('button', { name: 'Sign in' }).click();
      await expect(resetPage.getByRole('alert')).toBeVisible();

      await resetPage.goto(resetLink);
      await resetPage.getByLabel('New password', { exact: true }).fill('AcceptanceReplay!890');
      await resetPage.getByLabel('Confirm new password').fill('AcceptanceReplay!890');
      await resetPage.getByRole('button', { name: 'Reset password' }).click();
      await expect(resetPage.getByRole('alert')).toContainText('This password reset link is invalid or expired.');

      primaryPassword = changedPassword;
      await login(resetPage, primaryEmail, primaryPassword);
      const code = await waitForDifferentTotp(primaryMfa.secret, primaryLastTotp);
      await completeMfaChallenge(resetPage, code);
      primaryLastTotp = code;
      await expect(resetPage).toHaveURL(/\/$/u);
    } finally {
      await resetContext.close();
    }
  });

  test('Password change — authenticated change revokes all existing sessions and requires the new password', async ({ browser, page }) => {
    await signInWithPrimaryMfa(page);

    const staleContext = await browser.newContext();
    const stalePage = await staleContext.newPage();
    try {
      await login(stalePage, primaryEmail, primaryPassword);
      const staleCode = await waitForDifferentTotp(primaryMfa.secret, primaryLastTotp);
      await completeMfaChallenge(stalePage, staleCode);
      primaryLastTotp = staleCode;
      await expect(stalePage).toHaveURL(/\/$/u);

      await page.goto('/password/change');
      await page.getByLabel('Current password').fill(primaryPassword);
      await page.getByLabel('New password', { exact: true }).fill(rotatedPassword);
      await page.getByLabel('Confirm new password').fill(rotatedPassword);
      await page.getByRole('button', { name: 'Change password' }).click();
      await expect(page.getByRole('status')).toContainText('Your password has been changed. Sign in again.');

      await page.goto('/mfa');
      await expect(page).toHaveURL(/\/login$/u);
      await stalePage.goto('/mfa');
      await expect(stalePage).toHaveURL(/\/login$/u);

      await login(page, primaryEmail, primaryPassword);
      await expect(page.getByRole('alert')).toBeVisible();

      primaryPassword = rotatedPassword;
      await login(page, primaryEmail, primaryPassword);
      const freshCode = await waitForDifferentTotp(primaryMfa.secret, primaryLastTotp);
      await completeMfaChallenge(page, freshCode);
      primaryLastTotp = freshCode;
      await expect(page).toHaveURL(/\/$/u);
    } finally {
      await staleContext.close();
    }
  });

  test('Flow 5 — character validation, reserved/duplicate/quota/ownership/idempotency boundaries', async ({ browser, page }) => {
    await signInWithPrimaryMfa(page);

    await createCharacter(page, 'Bad123');
    await expect(page.getByRole('alert')).toContainText('Use only ASCII letters and spaces.');

    await createCharacter(page, 'God');
    await expect(page.getByRole('alert')).toContainText('reserved');

    await createCharacter(page, primaryCharacter);
    await expect(page.getByRole('status')).toContainText(`Character ${primaryCharacter} already exists on your account.`);

    const foreignContext = await browser.newContext();
    const foreignPage = await foreignContext.newPage();
    try {
      await register(foreignPage, foreignEmail, foreignPassword);
      const foreignBinding = runPhpState('binding', foreignEmail);
      expect(foreignBinding.status).toBe('ready');
      await login(foreignPage, foreignEmail, foreignPassword);
      const foreignCharacter = uniqueCharacterName('Other');
      await createCharacter(foreignPage, foreignCharacter);
      await expect(foreignPage.getByRole('status')).toContainText(`Character ${foreignCharacter} created.`);

      await createCharacter(page, foreignCharacter);
      await expect(page.getByRole('alert')).toContainText('That character name is not available.');

      const ownedName = uniqueCharacterName('Owned');
      await page.goto('/account/characters/create');
      await page.getByLabel('Character name').fill(ownedName);
      await page.getByLabel('Vocation').selectOption('4');
      await page.getByLabel('Sex').selectOption('1');
      await page.locator('form').evaluate((form, accountId) => {
        const input = document.createElement('input');
        input.type = 'hidden';
        input.name = 'account_id';
        input.value = String(accountId);
        form.appendChild(input);
      }, foreignBinding.canary_account_id);
      await page.getByRole('button', { name: 'Create character' }).click();
      await expect(page.getByRole('status')).toContainText(`Character ${ownedName} created.`);
      runPhpState('character-owner', ownedName, primaryEmail);
    } finally {
      await foreignContext.close();
    }

    for (let index = 0; index < 8; index += 1) {
      const name = uniqueCharacterName('Quota');
      await createCharacter(page, name);
      await expect(page.getByRole('status')).toContainText(`Character ${name} created.`);
    }

    const overQuota = uniqueCharacterName('Limit');
    await createCharacter(page, overQuota);
    await expect(page.getByRole('alert')).toContainText('maximum number of active characters');
  });

  test('Flow 6 — public game data, pagination, empty and controlled dependency-failure states', async ({ page }) => {
    await page.goto('/');
    await page.getByLabel('Character name').fill('Acceptance Hero');
    await page.getByRole('button', { name: 'Search' }).click();
    await expect(page.getByRole('heading', { name: 'Acceptance Hero' })).toBeVisible();

    await page.goto('/guilds/Acceptance%20Guild');
    await expect(page.getByRole('heading', { name: 'Acceptance Guild' })).toBeVisible();
    await expect(page.getByText('Acceptance Guildmate')).toBeVisible();

    await page.goto('/online');
    await expect(page.getByText('Acceptance Hero')).toBeVisible();
    await expect(page.getByText(/Acceptance \(ID 1\)/u)).toBeVisible();

    await page.goto('/highscores');
    await expect(page.getByRole('heading', { name: 'Level highscores' })).toBeVisible();
    await expect(page.getByText('Acceptance Hero')).toBeVisible();

    await page.goto('/highscores?page=999');
    await expect(page.getByText('No active characters found.')).toBeVisible();

    await page.goto('/servers');
    await expect(page.getByText('ONLINE', { exact: true })).toBeVisible();
    await expect(page.getByText('1', { exact: true }).last()).toBeVisible();

    runBinary('redis-cli', ['DEL', 'cluster:channel:1:runtime']);
    await page.goto('/servers');
    await expect(page.getByText(/Runtime:/u)).toContainText('Unknown');

    runBinary('redis-cli', ['HSET', 'cluster:channel:1:runtime', 'channel_id', '1', 'status', 'INVALID', 'players_online', '1']);
    runBinary('redis-cli', ['PEXPIRE', 'cluster:channel:1:runtime', '3600000']);
    await page.goto('/servers');
    await expect(page.getByText(/live runtime dependency is temporarily unavailable/u)).toBeVisible();

    runBinary('redis-cli', ['DEL', 'cluster:channel:1:runtime']);
    runBinary('redis-cli', ['HSET', 'cluster:channel:1:runtime', 'channel_id', '1', 'status', 'ONLINE', 'players_online', '1']);
    runBinary('redis-cli', ['PEXPIRE', 'cluster:channel:1:runtime', '3600000']);

    const rootPassword = process.env.MARIADB_ROOT_PASSWORD;
    const canaryDb = process.env.CANARY_DB_DATABASE;
    const readonlyUser = process.env.CANARY_DB_USERNAME;
    expect(rootPassword).toBeTruthy();
    expect(canaryDb).toBeTruthy();
    expect(readonlyUser).toBeTruthy();

    runBinary('mariadb', [
      '--protocol=tcp', '-h127.0.0.1', '-uroot', `-p${rootPassword}`,
      '-e', `REVOKE SELECT ON \`${canaryDb}\`.cluster_sessions FROM '${readonlyUser}'@'%';`,
    ]);
    try {
      const response = await page.goto('/online');
      expect(response?.status()).toBe(503);
      allowExpectedHttpFailure(page.__acceptanceDiagnostics, { status: 503, pathname: '/online' });
      await expect(page.locator('body')).not.toContainText('SQLSTATE');
      await expect(page.locator('body')).not.toContainText(rootPassword);
    } finally {
      runBinary('mariadb', [
        '--protocol=tcp', '-h127.0.0.1', '-uroot', `-p${rootPassword}`,
        '-e', `GRANT SELECT ON \`${canaryDb}\`.cluster_sessions TO '${readonlyUser}'@'%';`,
      ]);
    }
  });

  test('Flows 7-9 — admin MFA/RBAC, CMS publish lifecycle and audit evidence', async ({ browser, page }) => {
    await signInWithPrimaryMfa(page);
    const bootstrap = runArtisan('admin:bootstrap', primaryEmail);
    expect(bootstrap).toContain('First platform administrator assigned');
    runPhpState('unknown-permission-denied', primaryEmail);

    await page.goto('/admin');
    await expect(page.getByRole('heading', { name: 'Oteryn Admin' })).toBeVisible();

    await logout(page);
    await login(page, primaryEmail, primaryPassword);
    const adminCode = await waitForDifferentTotp(primaryMfa.secret, primaryLastTotp);
    await completeMfaChallenge(page, adminCode);
    primaryLastTotp = adminCode;
    await page.goto('/admin');
    await expect(page.getByRole('heading', { name: 'Oteryn Admin' })).toBeVisible();

    await page.goto('/admin/news/create');
    await page.getByLabel('Slug').fill(newsSlug);
    await page.getByLabel('Title').fill('Acceptance News');
    await page.getByLabel('Body (plain text)').fill('Acceptance news body v1');
    await page.getByLabel('Publish at').fill('2000-01-01T00:00');
    await page.getByRole('button', { name: 'Save' }).click();

    await page.goto(`/news/${newsSlug}`);
    await expect(page.getByRole('heading', { name: 'Acceptance News' })).toBeVisible();
    await expect(page.getByText('Acceptance news body v1')).toBeVisible();

    await page.goto('/admin/news');
    const newsRow = page.locator('tr').filter({ hasText: newsSlug });
    await newsRow.getByRole('link', { name: 'Edit' }).click();
    await page.getByLabel('Title').fill('Acceptance News Updated');
    await page.getByLabel('Body (plain text)').fill('Acceptance news body v2');
    await page.getByRole('button', { name: 'Save' }).click();
    await page.goto(`/news/${newsSlug}`);
    await expect(page.getByRole('heading', { name: 'Acceptance News Updated' })).toBeVisible();
    await expect(page.getByText('Acceptance news body v2')).toBeVisible();

    await page.goto('/admin/news');
    await page.locator('tr').filter({ hasText: newsSlug }).getByRole('link', { name: 'Edit' }).click();
    await page.getByLabel('Publish at').fill('');
    await page.getByRole('button', { name: 'Save' }).click();
    const unpublishedNews = await page.goto(`/news/${newsSlug}`);
    expect(unpublishedNews?.status()).toBe(404);

    await page.goto('/admin/pages/create');
    await page.getByLabel('Slug').fill(pageSlug);
    await page.getByLabel('Title').fill('Acceptance Managed Page');
    await page.getByLabel('Body (plain text)').fill('Acceptance managed page body v1');
    await page.getByLabel('Publish at').fill('2000-01-01T00:00');
    await page.getByRole('button', { name: 'Save' }).click();

    await page.goto(`/pages/${pageSlug}`);
    await expect(page.getByRole('heading', { name: 'Acceptance Managed Page' })).toBeVisible();
    await expect(page.getByText('Acceptance managed page body v1')).toBeVisible();

    await page.goto('/admin/pages');
    await page.locator('tr').filter({ hasText: pageSlug }).getByRole('link', { name: 'Edit' }).click();
    await page.getByLabel('Title').fill('Acceptance Managed Page Updated');
    await page.getByLabel('Body (plain text)').fill('Acceptance managed page body v2');
    await page.getByRole('button', { name: 'Save' }).click();
    await page.goto(`/pages/${pageSlug}`);
    await expect(page.getByRole('heading', { name: 'Acceptance Managed Page Updated' })).toBeVisible();

    await page.goto('/admin/pages');
    await page.locator('tr').filter({ hasText: pageSlug }).getByRole('link', { name: 'Edit' }).click();
    await page.getByLabel('Publish at').fill('');
    await page.getByRole('button', { name: 'Save' }).click();
    const unpublishedPage = await page.goto(`/pages/${pageSlug}`);
    expect(unpublishedPage?.status()).toBe(404);

    const editorContext = await browser.newContext();
    const editorPage = await editorContext.newPage();
    try {
      await register(editorPage, editorEmail, editorPassword);
      await login(editorPage, editorEmail, editorPassword);
      editorMfa = await enrollMfa(editorPage, editorPassword);
      editorLastTotp = editorMfa.enrollmentCode;
      runPhpState('binding', editorEmail);
    } finally {
      await editorContext.close();
    }

    await page.goto('/admin/roles');
    const editorRow = page.locator('tr').filter({ hasText: editorEmail });
    await editorRow.getByRole('button', { name: 'Assign content_editor' }).click();
    await expect(page.getByRole('status')).toBeVisible();

    const editorAuthorizedContext = await browser.newContext();
    const editorAuthorizedPage = await editorAuthorizedContext.newPage();
    try {
      await signInWithEditorMfa(editorAuthorizedPage);
      await editorAuthorizedPage.goto('/admin/news');
      await expect(editorAuthorizedPage.getByRole('heading', { name: 'News' })).toBeVisible();
      const deniedRoles = await editorAuthorizedPage.goto('/admin/roles');
      expect(deniedRoles?.status()).toBe(403);
      const deniedAudit = await editorAuthorizedPage.goto('/admin/audit');
      expect(deniedAudit?.status()).toBe(403);
    } finally {
      await editorAuthorizedContext.close();
    }

    await page.goto('/admin/roles');
    await page.locator('tr').filter({ hasText: editorEmail }).getByRole('button', { name: 'Remove content_editor' }).click();
    await expect(page.getByRole('status')).toBeVisible();

    const removedRoleContext = await browser.newContext();
    const removedRolePage = await removedRoleContext.newPage();
    try {
      await signInWithEditorMfa(removedRolePage);
      const deniedNews = await removedRolePage.goto('/admin/news');
      expect(deniedNews?.status()).toBe(403);
    } finally {
      await removedRoleContext.close();
    }

    await page.goto('/admin/roles');
    await page.locator('tr').filter({ hasText: primaryEmail }).getByRole('button', { name: 'Remove platform_admin' }).click();
    await expect(page.getByRole('alert')).toContainText('final platform_admin');

    await page.goto('/admin/audit');
    await expect(page.getByText('admin.bootstrap_first_platform_admin')).toBeVisible();
    await expect(page.getByText('admin.role_assigned')).toBeVisible();
    await expect(page.getByText('admin.role_removed')).toBeVisible();
    await expect(page.getByText('cms.news_created')).toBeVisible();
    await expect(page.getByText('cms.news_updated')).toBeVisible();
    await expect(page.getByText('cms.page_created')).toBeVisible();
    await expect(page.getByText('cms.page_updated')).toBeVisible();

    const newsAuditRow = page.locator('tr').filter({ hasText: 'cms.news_created' }).first();
    await expect(newsAuditRow).toContainText(primaryEmail);
    await expect(newsAuditRow).toContainText('news_post');
    await expect(newsAuditRow).toContainText(newsSlug);

    const pageAuditRow = page.locator('tr').filter({ hasText: 'cms.page_created' }).first();
    await expect(pageAuditRow).toContainText(primaryEmail);
    await expect(pageAuditRow).toContainText('managed_page');
    await expect(pageAuditRow).toContainText(pageSlug);

    await expect(page.locator('body')).not.toContainText(primaryPassword);
    await expect(page.locator('body')).not.toContainText(primaryMfa.secret);
    for (const recoveryCode of primaryMfa.recoveryCodes) {
      await expect(page.locator('body')).not.toContainText(recoveryCode);
    }
  });

  test('Flow 4 — MFA invalid/replay/recovery single-use/disable and session invalidation', async ({ page }) => {
    await logout(page);
    await login(page, primaryEmail, primaryPassword);

    await completeMfaChallenge(page, 'not-a-code');
    await expect(page.getByRole('alert')).toBeVisible();

    await completeMfaChallenge(page, primaryLastTotp);
    await expect(page.getByRole('alert')).toBeVisible();

    const recoveryCode = primaryMfa.recoveryCodes[0];
    await completeMfaChallenge(page, recoveryCode);
    await expect(page).toHaveURL(/\/$/u);

    await logout(page);
    await login(page, primaryEmail, primaryPassword);
    await completeMfaChallenge(page, recoveryCode);
    await expect(page.getByRole('alert')).toBeVisible();

    await completeMfaChallenge(page, primaryMfa.recoveryCodes[1]);
    await expect(page).toHaveURL(/\/$/u);

    await page.goto('/mfa');
    await page.getByLabel('Current password').fill(primaryPassword);
    await page.getByLabel('Fresh authenticator or recovery code').fill(primaryMfa.recoveryCodes[2]);
    await page.getByRole('button', { name: 'Disable MFA and sign out everywhere' }).click();
    await expect(page.getByRole('status')).toContainText('Multi-factor authentication has been disabled.');

    await page.goto('/mfa');
    await expect(page).toHaveURL(/\/login$/u);
  });

  test('Visual/UX evidence — desktop/tablet/mobile representative surfaces and deterministic accessibility smoke', async ({ page }) => {
    const publicSurfaces = [
      ['home', '/', 'Oteryn Platform'],
      ['news-empty', '/news', 'News'],
      ['online', '/online', 'Online characters'],
      ['highscores', '/highscores', 'Level highscores'],
      ['servers', '/servers', 'Servers'],
      ['character-detail', '/characters/Acceptance%20Hero', 'Acceptance Hero'],
      ['login', '/login', 'Sign in to Oteryn Platform'],
      ['registration', '/register', 'Create an Oteryn Platform identity'],
    ];

    for (const [label, url, heading] of publicSurfaces) {
      await page.setViewportSize({ width: 1440, height: 1000 });
      await page.goto(url);
      await expect(page.getByRole('heading', { name: heading }).first()).toBeVisible();
      await assertAccessibilitySmoke(page);
      await evidenceScreenshot(page, `${label}-desktop`);
    }

    for (const [viewportLabel, viewport] of [
      ['tablet', { width: 820, height: 1180 }],
      ['mobile', { width: 390, height: 844 }],
    ]) {
      for (const [label, url, heading] of publicSurfaces.slice(0, 6)) {
        await page.setViewportSize(viewport);
        await page.goto(url);
        await expect(page.getByRole('heading', { name: heading }).first()).toBeVisible();
        await assertAccessibilitySmoke(page);
        await evidenceScreenshot(page, `${label}-${viewportLabel}`);
      }
    }

    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/register');
    await page.getByLabel('Email').fill('invalid@example.test');
    await page.getByLabel('Password', { exact: true }).fill('short');
    await page.getByLabel('Confirm password').fill('different');
    await page.getByRole('button', { name: 'Register' }).click();
    await expect(page.getByRole('alert')).toBeVisible();
    await evidenceScreenshot(page, 'validation-state-mobile');

    primaryMfa = await (async () => {
      await login(page, primaryEmail, primaryPassword);
      const reEnrollment = await enrollMfa(page, primaryPassword);
      primaryLastTotp = reEnrollment.enrollmentCode;
      return reEnrollment;
    })();

    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.goto('/mfa');
    await evidenceScreenshot(page, 'mfa-settings-desktop');
    await page.goto('/account/characters/create');
    await assertAccessibilitySmoke(page);
    await evidenceScreenshot(page, 'character-create-desktop');

    await logout(page);
    await login(page, primaryEmail, primaryPassword);
    await evidenceScreenshot(page, 'mfa-challenge-desktop');
    const code = await waitForDifferentTotp(primaryMfa.secret, primaryLastTotp);
    await completeMfaChallenge(page, code);
    primaryLastTotp = code;

    await page.goto('/admin');
    await assertAccessibilitySmoke(page);
    await evidenceScreenshot(page, 'admin-dashboard-desktop');
    await page.goto('/admin/news');
    await assertAccessibilitySmoke(page);
    await evidenceScreenshot(page, 'admin-cms-news-desktop');
    await page.goto('/admin/audit');
    await assertAccessibilitySmoke(page);
    await evidenceScreenshot(page, 'admin-audit-desktop');

    await page.setViewportSize({ width: 390, height: 844 });
    for (const [label, url] of [
      ['character-create-mobile', '/account/characters/create'],
      ['admin-dashboard-mobile', '/admin'],
      ['admin-cms-news-mobile', '/admin/news'],
      ['admin-audit-mobile', '/admin/audit'],
    ]) {
      await page.goto(url);
      await assertAccessibilitySmoke(page);
      await evidenceScreenshot(page, label);
    }
  });
});
