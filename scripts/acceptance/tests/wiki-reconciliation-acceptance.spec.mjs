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
  runArtisan,
  runBinary,
  uniqueEmail,
} from './helpers.mjs';

const password = 'AcceptanceWikiReconciliation!234';

function wikiFixture(...args) {
  return JSON.parse(runBinary('php', ['scripts/acceptance/seed-browser-wiki-reconciliation.php', ...args]));
}

function seedPublicWiki() {
  runBinary('php', ['scripts/acceptance/seed-public-wiki.php']);
}

function seedIdentity(label, { confirmedMfa, permissions }) {
  const email = uniqueEmail(label);
  const recoveryCode = `WKR-${label.toUpperCase().replace(/[^A-Z0-9]/gu, '').slice(0, 14)}-01`;
  wikiFixture(
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

async function fillCategoryForm(page, suffix) {
  await page.getByLabel('Stable key').fill(`acceptance-${suffix}`);
  await page.getByLabel('Display order').fill('10');
  await page.getByLabel('Name').first().fill(`Acceptance Guides ${suffix}`);
  await page.getByLabel('Slug').first().fill(`acceptance-guides-${suffix}`);
  await page.getByLabel('Description').first().fill('Acceptance reconciliation guides.');
  await page.getByLabel('Name').nth(1).fill(`Poradniki akceptacyjne ${suffix}`);
  await page.getByLabel('Slug').nth(1).fill(`poradniki-akceptacyjne-${suffix}`);
  await page.getByLabel('Description').nth(1).fill('Poradniki uzgodnienia akceptacyjnego.');
}

async function fillArticleForm(page, {
  englishTitle,
  englishSlug,
  polishTitle,
  polishSlug,
  categoryName,
  englishMarkdown,
  changeNote,
}) {
  await page.getByLabel('Content type').fill('guide');
  await page.getByLabel('Display order').fill('5');
  await page.getByLabel('Feature this article').check();
  await page.getByLabel('Title').first().fill(englishTitle);
  await page.getByLabel('Slug').first().fill(englishSlug);
  await page.getByLabel('Summary').first().fill('Acceptance reconciliation summary.');
  await page.getByLabel('Markdown source').first().fill(englishMarkdown);
  await page.getByLabel('Title').nth(1).fill(polishTitle);
  await page.getByLabel('Slug').nth(1).fill(polishSlug);
  await page.getByLabel('Summary').nth(1).fill('Podsumowanie uzgodnienia akceptacyjnego.');
  await page.getByLabel('Markdown source').nth(1).fill('# Polski nagłówek\n\nBezpieczna polska treść akceptacyjna.');
  await page.getByLabel(categoryName).check();
  await page.getByLabel('Change note').fill(changeNote);
}

test.setTimeout(240_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  wikiFixture('restore-public');
  wikiFixture('reset');
  runArtisan('cache:clear');
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  try {
    await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
  } finally {
    wikiFixture('restore-public');
    wikiFixture('reset');
  }
});

test('@portal-wiki-public route-complete empty, read, search, invalid, not-found, unavailable, recovery and EN/PL states', async ({ page }) => {
  let response = await page.goto('/en/wiki');
  expect(response?.status()).toBe(200);
  await expect(page.getByText('The Wiki has no published content yet.')).toBeVisible();
  await assertAccessibilitySmoke(page);
  await assertNoPageOverflow(page);

  seedPublicWiki();

  for (const path of ['/wiki', '/en/wiki']) {
    response = await page.goto(path);
    expect(response?.status(), path).toBe(200);
    await expect(page.getByRole('heading', { level: 1, name: 'Wiki' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Getting Started' })).toBeVisible();
  }

  response = await page.goto('/pl/wiki');
  expect(response?.status()).toBe(200);
  await expect(page.locator('html')).toHaveAttribute('lang', 'pl');
  await expect(page.getByRole('link', { name: 'Pierwsze kroki' })).toBeVisible();
  await expect(page.getByText('Getting Started', { exact: true })).toHaveCount(0);

  for (const path of ['/wiki/category/getting-started', '/en/wiki/category/getting-started']) {
    response = await page.goto(path);
    expect(response?.status(), path).toBe(200);
    await expect(page.getByRole('heading', { level: 1, name: 'Getting Started' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'First login' })).toBeVisible();
  }

  response = await page.goto('/pl/wiki/category/pierwsze-kroki');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { level: 1, name: 'Pierwsze kroki' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Pierwsze logowanie' })).toBeVisible();

  for (const path of ['/wiki/first-login', '/en/wiki/first-login']) {
    response = await page.goto(path);
    expect(response?.status(), path).toBe(200);
    await expect(page.getByRole('heading', { level: 1, name: 'First login' })).toBeVisible();
    await expect(page.getByRole('complementary').getByRole('link', { name: 'Install the client' })).toBeVisible();
  }

  response = await page.goto('/pl/wiki/pierwsze-logowanie');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { level: 1, name: 'Pierwsze logowanie' })).toBeVisible();
  await expect(page.getByText('First login', { exact: true })).toHaveCount(0);

  for (const path of ['/wiki/search?q=first+login', '/en/wiki/search?q=first+login']) {
    response = await page.goto(path);
    expect(response?.status(), path).toBe(200);
    await expect(page.getByRole('link', { name: 'First login' })).toBeVisible();
  }

  response = await page.goto('/pl/wiki/search?q=pierwsze');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('link', { name: 'Pierwsze logowanie' })).toBeVisible();
  await expect(page.getByText('First login', { exact: true })).toHaveCount(0);

  response = await page.goto('/en/wiki/search?q=no-match-acceptance');
  expect(response?.status()).toBe(200);
  await expect(page.getByText('No published articles matched.')).toBeVisible();

  response = await page.goto('/en/wiki/search?q=x');
  expect(response?.status()).toBe(422);
  await expect(page.getByText('The Wiki search query must contain at least two characters.')).toBeVisible();

  response = await page.goto('/en/wiki/category/does-not-exist');
  expect(response?.status()).toBe(404);
  response = await page.goto('/en/wiki/does-not-exist');
  expect(response?.status()).toBe(404);

  wikiFixture('set-public-unavailable');
  try {
    response = await page.goto('/en/wiki');
    expect(response?.status()).toBe(503);
    allowExpectedHttpFailure(page.__acceptanceDiagnostics, { status: 503, pathname: '/en/wiki' });
    await expect(page.getByText('Wiki is temporarily unavailable.')).toBeVisible();
    expect(page.__acceptanceDiagnostics.serverErrors.some((entry) => entry.status === 503)).toBe(true);
  } finally {
    wikiFixture('restore-public');
  }

  response = await page.goto('/en/wiki');
  expect(response?.status()).toBe(200);
  await expect(
    page.getByRole('region', { name: 'Featured articles' }).getByRole('link', { name: 'First login' }),
  ).toBeVisible();
  await assertAccessibilitySmoke(page);
  await assertNoPageOverflow(page);
  await evidenceScreenshot(page, `wiki-public-reconciliation-${test.info().project.name}`);

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});

test('@portal-wiki-admin guest, MFA and exact Wiki permission boundaries fail closed', async ({ page }) => {
  await page.goto('/admin/wiki');
  await expect(page).toHaveURL(/\/login$/u);

  const noMfa = seedIdentity('wiki-reconciliation-no-mfa', {
    confirmedMfa: false,
    permissions: ['wiki.access', 'wiki.articles.manage', 'wiki.categories.manage', 'wiki.publish'],
  });
  await signIn(page, noMfa);
  let response = await page.goto('/admin/wiki');
  expect(response?.status()).toBe(403);
  await expect(page.getByRole('heading', { name: 'You do not have access to this page' })).toBeVisible();
  await logout(page);

  const noPermission = seedIdentity('wiki-reconciliation-no-permission', {
    confirmedMfa: true,
    permissions: [],
  });
  await signIn(page, noPermission);
  response = await page.goto('/admin/wiki');
  expect(response?.status()).toBe(403);
  await expect(page.getByRole('heading', { name: 'You do not have access to this page' })).toBeVisible();
  await logout(page);

  const accessOnly = seedIdentity('wiki-reconciliation-access-only', {
    confirmedMfa: true,
    permissions: ['wiki.access'],
  });
  await signIn(page, accessOnly);
  response = await page.goto('/admin/wiki');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { name: 'Wiki administration' })).toBeVisible();
  response = await page.goto('/admin/wiki/articles/create');
  expect(response?.status()).toBe(403);
  response = await page.goto('/admin/wiki/categories/create');
  expect(response?.status()).toBe(403);

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});

test('@portal-wiki-admin complete validation, draft, signed-preview, conflict, review, publish, unpublish, revision-restore, archive and audit lifecycle', async ({ page, context }) => {
  const publisher = seedIdentity('wiki-reconciliation-publisher', {
    confirmedMfa: true,
    permissions: ['wiki.access', 'wiki.articles.manage', 'wiki.categories.manage', 'wiki.publish', 'audit.view'],
  });
  await signIn(page, publisher);

  const suffix = Math.random().toString(16).slice(2, 10);
  const categoryName = `Acceptance Guides ${suffix}`;
  const originalTitle = `Acceptance Wiki Article ${suffix}`;
  const updatedTitle = `Updated Acceptance Wiki Article ${suffix}`;
  const articleSlug = `acceptance-wiki-${suffix}`;
  const polishTitle = `Artykuł Wiki ${suffix}`;
  const polishSlug = `artykul-wiki-${suffix}`;

  await page.goto('/admin/wiki/categories/create');
  await fillCategoryForm(page, suffix);
  await page.getByRole('button', { name: 'Create category' }).click();
  await expect(page.getByRole('status')).toContainText('Wiki category created.');

  await page.goto('/admin/wiki/articles/create');
  await fillArticleForm(page, {
    englishTitle: originalTitle,
    englishSlug: articleSlug,
    polishTitle,
    polishSlug,
    categoryName,
    englishMarkdown: '<script>alert(1)</script>',
    changeNote: 'Rejected unsafe reconciliation draft.',
  });
  await page.getByRole('button', { name: 'Create draft' }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page).toHaveURL(/\/admin\/wiki\/articles\/create$/u);

  await page.getByLabel('Markdown source').first().fill('# Acceptance heading\n\nSafe acceptance article body.');
  await page.getByLabel('Change note').fill('Acceptance draft baseline.');
  await page.getByRole('button', { name: 'Create draft' }).click();
  await expect(page.getByRole('status')).toContainText('Wiki article draft created.');
  await expect(page.getByRole('heading', { name: originalTitle })).toBeVisible();
  const editUrl = page.url();
  const articleId = editUrl.match(/\/admin\/wiki\/articles\/(\d+)\/edit$/u)?.[1];
  expect(articleId).toBeTruthy();

  let response = await page.goto(`/en/wiki/${articleSlug}`);
  expect(response?.status()).toBe(404);

  await page.goto(editUrl);
  const previewPromise = context.waitForEvent('page');
  await page.getByRole('link', { name: 'Preview EN' }).click();
  const preview = await previewPromise;
  await preview.waitForLoadState();
  await expect(preview.getByRole('heading', { name: originalTitle })).toBeVisible();
  await expect(preview.locator('meta[name="robots"]')).toHaveAttribute('content', 'noindex,nofollow,noarchive');
  await assertNoPageOverflow(preview);
  await preview.close();

  await page.getByLabel('Title').first().fill('Stale edit must not persist');
  wikiFixture('bump-article-lock', articleId);
  const [conflictResponse] = await Promise.all([
    page.waitForResponse((candidate) => candidate.url().includes(`/admin/wiki/articles/${articleId}`) && candidate.status() === 409),
    page.getByRole('button', { name: 'Save draft' }).click(),
  ]);
  expect(conflictResponse.status()).toBe(409);
  await expect(page.locator('body')).not.toContainText('SQLSTATE');
  await expect(page.locator('body')).not.toContainText(publisher.password);
  await expect(page.locator('body')).not.toContainText(publisher.recoveryCode);

  await page.goto(editUrl);
  await page.getByLabel('Title').first().fill(updatedTitle);
  await page.getByLabel('Summary').nth(1).fill('Odświeżone podsumowanie uzgodnienia akceptacyjnego.');
  await page.getByLabel('Change note').fill('Acceptance update before lifecycle.');
  await page.getByRole('button', { name: 'Save draft' }).click();
  await expect(page.getByRole('status')).toContainText('Wiki article draft saved.');

  await page.getByRole('button', { name: 'Submit for review' }).click();
  await expect(page.getByRole('status')).toContainText('Wiki article submitted for review.');
  await expect(page.locator('.page-header')).toContainText('Status: In Review');

  await page.getByRole('button', { name: 'Publish', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Wiki article published.');
  await expect(page.locator('.page-header')).toContainText('Status: Published');

  response = await page.goto(`/en/wiki/${articleSlug}`);
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { level: 1, name: updatedTitle })).toBeVisible();
  response = await page.goto(`/pl/wiki/${polishSlug}`);
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('heading', { level: 1, name: polishTitle })).toBeVisible();

  await page.goto(editUrl);
  await page.getByRole('button', { name: 'Unpublish to draft' }).click();
  await expect(page.getByRole('status')).toContainText('Wiki article unpublished and returned to draft.');
  response = await page.goto(`/en/wiki/${articleSlug}`);
  expect(response?.status()).toBe(404);

  await page.goto(editUrl);
  await page.getByRole('link', { name: 'Revision history' }).click();
  await expect(page.getByRole('heading', { name: updatedTitle })).toBeVisible();
  const originalRevision = page.getByRole('row').filter({ hasText: originalTitle }).first();
  await expect(originalRevision).toBeVisible();
  await originalRevision.getByPlaceholder('Reason for restore').fill('Restore original acceptance title.');
  await originalRevision.getByRole('button', { name: 'Restore as new revision' }).click();
  await expect(page.getByRole('status')).toContainText('Historical Wiki revision restored as a new revision.');
  await expect(page.getByText(/Restored from/u).first()).toBeVisible();

  await page.getByRole('link', { name: 'Edit article' }).click();
  await expect(page.getByRole('heading', { name: originalTitle })).toBeVisible();
  await page.getByRole('button', { name: 'Archive' }).click();
  await expect(page.getByRole('status')).toContainText('Wiki article archived.');
  await expect(page.locator('.page-header')).toContainText('Status: Archived');
  response = await page.goto(`/en/wiki/${articleSlug}`);
  expect(response?.status()).toBe(404);

  await page.goto('/admin/audit');
  await expect(page.getByRole('heading', { name: 'Administrator audit' })).toBeVisible();
  for (const action of [
    'wiki.article_created',
    'wiki.article_updated',
    'wiki.article_submitted_for_review',
    'wiki.article_published',
    'wiki.article_unpublished',
    'wiki.revision_restored',
    'wiki.article_archived',
  ]) {
    await expect(page.getByText(action).first()).toBeVisible();
  }
  await expect(page.locator('body')).not.toContainText('Safe acceptance article body.');
  await expect(page.locator('body')).not.toContainText(publisher.password);
  await expect(page.locator('body')).not.toContainText(publisher.recoveryCode);
  await assertAccessibilitySmoke(page);
  await assertNoPageOverflow(page);
  await evidenceScreenshot(page, `wiki-admin-reconciliation-${test.info().project.name}`);

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});
