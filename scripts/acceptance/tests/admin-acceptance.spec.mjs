import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  completeMfaChallenge,
  enrollMfa,
  installDiagnostics,
  login,
  logout,
  register,
  runArtisan,
  runPhpState,
  uniqueEmail,
  waitForDifferentTotp,
} from './helpers.mjs';

test.setTimeout(180_000);
test.describe.configure({ retries: 0 });

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

test('Flows 7-9 — admin MFA/RBAC, CMS lifecycle and audit evidence', async ({ browser, page }) => {
  const adminEmail = uniqueEmail('admin');
  const adminPassword = 'AcceptanceAdmin!234';
  const editorEmail = uniqueEmail('editor');
  const editorPassword = 'AcceptanceEditor!234';
  const newsSlug = `acceptance-news-${(process.env.ACCEPTANCE_RUN_ID ?? 'local').replace(/[^a-zA-Z0-9-]/gu, '-').toLowerCase()}`;
  const pageSlug = `acceptance-page-${(process.env.ACCEPTANCE_RUN_ID ?? 'local').replace(/[^a-zA-Z0-9-]/gu, '-').toLowerCase()}`;

  await register(page, adminEmail, adminPassword);
  await login(page, adminEmail, adminPassword);
  const adminMfa = await enrollMfa(page, adminPassword);
  let adminLastTotp = adminMfa.enrollmentCode;

  const bootstrap = runArtisan('admin:bootstrap', adminEmail);
  expect(bootstrap).toContain('First platform administrator assigned');
  runPhpState('unknown-permission-denied', adminEmail);

  await page.goto('/admin');
  await expect(page.getByRole('heading', { name: 'Administration' })).toBeVisible();

  await logout(page);
  await login(page, adminEmail, adminPassword);
  const adminCode = await waitForDifferentTotp(adminMfa.secret, adminLastTotp);
  await completeMfaChallenge(page, adminCode);
  adminLastTotp = adminCode;
  await page.goto('/admin');
  await expect(page.getByRole('heading', { name: 'Administration' })).toBeVisible();

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
  await page.locator('tr').filter({ hasText: newsSlug }).getByRole('link', { name: 'Edit' }).click();
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
  let editorMfa;
  let editorLastTotp;
  try {
    await register(editorPage, editorEmail, editorPassword);
    await login(editorPage, editorEmail, editorPassword);
    editorMfa = await enrollMfa(editorPage, editorPassword);
    editorLastTotp = editorMfa.enrollmentCode;
    const editorBinding = runPhpState('binding', editorEmail);
    expect(editorBinding.status).toBe('ready');
  } finally {
    await editorContext.close();
  }

  await page.goto('/admin/roles');
  await page.locator('tr').filter({ hasText: editorEmail }).getByRole('button', { name: 'Assign content_editor' }).click();
  await expect(page.getByRole('status')).toBeVisible();

  const editorAuthorizedContext = await browser.newContext();
  const editorAuthorizedPage = await editorAuthorizedContext.newPage();
  await login(editorAuthorizedPage, editorEmail, editorPassword);
  const editorCode = await waitForDifferentTotp(editorMfa.secret, editorLastTotp);
  await completeMfaChallenge(editorAuthorizedPage, editorCode);
  editorLastTotp = editorCode;

  await editorAuthorizedPage.goto('/admin/news');
  await expect(editorAuthorizedPage.getByRole('heading', { name: 'News' })).toBeVisible();

  const deniedRoles = await editorAuthorizedPage.goto('/admin/roles');
  expect(deniedRoles?.status()).toBe(403);
  const deniedAudit = await editorAuthorizedPage.goto('/admin/audit');
  expect(deniedAudit?.status()).toBe(403);

  await page.goto('/admin/roles');
  await page.locator('tr').filter({ hasText: editorEmail }).getByRole('button', { name: 'Remove content_editor' }).click();
  await expect(page.getByRole('status')).toBeVisible();

  const revokedExistingSession = await editorAuthorizedPage.goto('/admin/news');
  expect(revokedExistingSession?.status()).toBe(403);
  await editorAuthorizedContext.close();

  await page.goto('/admin/roles');
  await page.locator('tr').filter({ hasText: adminEmail }).getByRole('button', { name: 'Remove platform_admin' }).click();
  await expect(page.getByRole('alert')).toContainText('final platform_admin');

  await page.goto('/admin/audit');
  await expect(page.getByText('admin.bootstrap_first_platform_admin').first()).toBeVisible();
  await expect(page.getByText('admin.role_assigned').first()).toBeVisible();
  await expect(page.getByText('admin.role_removed').first()).toBeVisible();
  await expect(page.getByText('cms.news_created').first()).toBeVisible();
  await expect(page.getByText('cms.news_updated').first()).toBeVisible();
  await expect(page.getByText('cms.page_created').first()).toBeVisible();
  await expect(page.getByText('cms.page_updated').first()).toBeVisible();

  const newsAuditRow = page.locator('tr').filter({ hasText: 'cms.news_created' }).first();
  await expect(newsAuditRow).toContainText(adminEmail);
  await expect(newsAuditRow).toContainText('news_post');
  await expect(newsAuditRow).toContainText(newsSlug);

  const pageAuditRow = page.locator('tr').filter({ hasText: 'cms.page_created' }).first();
  await expect(pageAuditRow).toContainText(adminEmail);
  await expect(pageAuditRow).toContainText('managed_page');
  await expect(pageAuditRow).toContainText(pageSlug);

  await expect(page.locator('body')).not.toContainText(adminPassword);
  await expect(page.locator('body')).not.toContainText(adminMfa.secret);
  for (const recoveryCode of adminMfa.recoveryCodes) {
    await expect(page.locator('body')).not.toContainText(recoveryCode);
  }
});
