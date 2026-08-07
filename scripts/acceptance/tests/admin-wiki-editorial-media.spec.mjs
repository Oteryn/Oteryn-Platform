import crypto from 'node:crypto';
import { test, expect } from '@playwright/test';
import {
  assertAccessibilitySmoke,
  attachDiagnostics,
  completeMfaChallenge,
  installDiagnostics,
  login,
  runArtisan,
  runBinary,
  uniqueEmail,
} from './helpers.mjs';

const password = 'Acceptance-Wiki-Media-9!Pass';
const recoveryCode = 'WIKIMEDIA-00001';

function suffix() {
  return crypto.randomBytes(6).toString('hex');
}

function editorialMediaFixture(...args) {
  return JSON.parse(runBinary('php', ['scripts/acceptance/seed-browser-editorial-media.php', ...args]));
}

function seedWikiMediaEditor(email, label) {
  runBinary('php', [
    'scripts/acceptance/seed-browser-admin.php',
    email,
    password,
    recoveryCode,
  ]);
  runBinary('php', [
    'scripts/acceptance/seed-admin-wiki-permissions.php',
    email,
    '--wiki-only',
  ]);

  return JSON.parse(runBinary('php', [
    'scripts/acceptance/seed-wiki-editorial-media.php',
    email,
    label,
  ]));
}

async function expectNoHorizontalOverflow(page) {
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow).toBeLessThanOrEqual(1);
}

function trackWikiMediaThumbnailRequests(page) {
  const pending = new Set();
  const isThumbnail = (request) => {
    const pathname = new URL(request.url()).pathname;
    return /^\/admin\/wiki\/media\/\d+\/thumbnail$/u.test(pathname);
  };
  const finish = (request) => pending.delete(request);

  page.on('request', (request) => {
    if (isThumbnail(request)) {
      pending.add(request);
    }
  });
  page.on('requestfinished', finish);
  page.on('requestfailed', finish);

  return () => pending.size;
}

async function waitForWikiMediaThumbnailIdle(page, pendingCount) {
  await expect(page.locator('[data-wiki-media-status]'))
    .toHaveText(/\d+ approved images? available\./u);

  let idleSince = Date.now();
  await expect.poll(
    () => {
      if (pendingCount() > 0) {
        idleSince = Date.now();
        return false;
      }

      return Date.now() - idleSince >= 750;
    },
    { timeout: 30_000, intervals: [100] },
  ).toBe(true);
}

test.setTimeout(180_000);
test.describe.configure({ mode: 'serial', retries: 0 });

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

test('@wiki-media exact Wiki editor discovers inserts previews and publishes private EditorialMedia responsively', async ({ page, context }) => {
  const pendingThumbnailRequests = trackWikiMediaThumbnailRequests(page);
  const email = uniqueEmail('wiki-media');
  const id = suffix();
  const mediaLabel = `Oteryn acceptance bridge ${id}`;
  const seeded = seedWikiMediaEditor(email, mediaLabel);
  const articleTitle = `Media guide ${id}`;
  const articleSlug = `media-guide-${id}`;
  const polishTitle = `Poradnik mediow ${id}`;
  const polishSlug = `poradnik-mediow-${id}`;

  runArtisan('cache:clear');
  await login(page, email, password);
  await completeMfaChallenge(page, recoveryCode);

  const forbiddenMedia = await page.request.get('/admin/media');
  expect(forbiddenMedia.status()).toBe(403);

  await page.goto('/admin/wiki/articles/create');
  await expect(page.getByRole('heading', { name: 'Create Wiki article' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Insert an existing image' })).toBeVisible();
  await page.getByLabel('Search approved images').fill(String(seeded.media_id));
  await page.getByRole('button', { name: 'Search', exact: true }).click();

  const mediaCard = page.locator('.wiki-media-card').filter({ hasText: `Media ${seeded.media_id}` });
  await expect(mediaCard.getByRole('img', { name: mediaLabel })).toBeVisible();
  await mediaCard.getByRole('button', { name: 'Insert in English' }).focus();
  await expect(mediaCard.getByRole('button', { name: 'Insert in English' })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByLabel('Markdown source').first())
    .toHaveValue(new RegExp(`wiki-media:${seeded.media_id}`));

  await mediaCard.getByRole('button', { name: 'Insert in Polish' }).click();
  const polishMarkdown = page.getByLabel('Markdown source').nth(1);
  await expect(polishMarkdown).toHaveValue(new RegExp(`wiki-media:${seeded.media_id}`));
  await polishMarkdown.fill(`![Most Oteryn ${id}](wiki-media:${seeded.media_id})`);

  await page.getByLabel('Content type').fill('guide');
  await page.getByLabel('Display order').fill('5');
  await page.getByLabel('Title').first().fill(articleTitle);
  await page.getByLabel('Slug').first().fill(articleSlug);
  await page.getByLabel('Summary').first().fill('Approved image integration guidance.');
  await page.getByLabel('Title').nth(1).fill(polishTitle);
  await page.getByLabel('Slug').nth(1).fill(polishSlug);
  await page.getByLabel('Summary').nth(1).fill('Zatwierdzony poradnik integracji obrazow.');
  await page.getByLabel('Change note').fill('Acceptance media integration.');
  await expectNoHorizontalOverflow(page);
  await assertAccessibilitySmoke(page);
  await page.getByRole('button', { name: 'Create draft' }).click();

  await expect(page.getByRole('status')).toContainText('Wiki article draft created.');
  const previewPromise = context.waitForEvent('page');
  await page.getByRole('link', { name: 'Preview EN' }).click();
  const preview = await previewPromise;
  await preview.waitForLoadState();
  const previewImage = preview.getByRole('img', { name: mediaLabel });
  await expect(previewImage).toBeVisible();
  expect(await previewImage.evaluate((image) => image.naturalWidth)).toBeGreaterThan(0);
  await expectNoHorizontalOverflow(preview);
  await assertAccessibilitySmoke(preview);
  await preview.close();

  await page.getByRole('button', { name: 'Submit for review' }).click();
  const publishButton = page.getByRole('button', { name: 'Publish', exact: true });
  await publishButton.scrollIntoViewIfNeeded();
  await waitForWikiMediaThumbnailIdle(page, pendingThumbnailRequests);
  await publishButton.click();
  await expect(page.getByRole('status')).toContainText('Wiki article published.');
  await expect(page.getByText(/Status:\s*Published/i)).toBeVisible();

  const browser = context.browser();
  expect(browser).not.toBeNull();
  const baseURL = new URL(page.url()).origin;
  const publicContext = await browser.newContext({ baseURL });
  const publicPage = await publicContext.newPage();
  try {
    await publicPage.goto(`/en/wiki/${articleSlug}`);
    const publicImage = publicPage.getByRole('img', { name: mediaLabel });
    await expect(publicImage).toBeVisible();
    expect(await publicImage.evaluate((image) => image.naturalWidth)).toBeGreaterThan(0);
    const imageResponse = await publicPage.request.get(await publicImage.getAttribute('src'));
    expect(imageResponse.status()).toBe(200);
    expect(imageResponse.headers()['x-content-type-options']).toBe('nosniff');
    expect(imageResponse.headers()['cache-control']).toContain('must-revalidate');
    await expectNoHorizontalOverflow(publicPage);
    await assertAccessibilitySmoke(publicPage);

    await publicPage.goto(`/pl/wiki/${polishSlug}`);
    await expect(publicPage.getByRole('img', { name: `Most Oteryn ${id}` })).toBeVisible();
    await expectNoHorizontalOverflow(publicPage);

    editorialMediaFixture('corrupt-files', String(seeded.media_id));

    await page.goto('/admin/wiki/articles/create');
    await page.getByLabel('Search approved images').fill(String(seeded.media_id));
    await page.getByRole('button', { name: 'Search', exact: true }).click();
    const corruptPickerCard = page.locator('.wiki-media-card').filter({ hasText: `Media ${seeded.media_id}` });
    const corruptPickerFallback = corruptPickerCard.getByRole('img', { name: mediaLabel });
    await expect(corruptPickerFallback).toBeVisible();
    await expect(corruptPickerFallback).toContainText(`Preview unavailable: ${mediaLabel}`);
    await expect(corruptPickerCard.locator('img')).toHaveCount(0);

    await publicPage.reload();
    const corruptPublicFallback = publicPage.getByRole('img', { name: `Most Oteryn ${id}` });
    await expect(corruptPublicFallback).toBeVisible();
    await expect(corruptPublicFallback).toHaveAttribute('data-media-fallback-state', 'unavailable');
    await expect(publicPage.locator('.wiki-editorial-image')).toHaveCount(0);
    await expectNoHorizontalOverflow(publicPage);

    editorialMediaFixture('remove-files', String(seeded.media_id));

    await page.goto('/admin/wiki/articles/create');
    await page.getByLabel('Search approved images').fill(String(seeded.media_id));
    await page.getByRole('button', { name: 'Search', exact: true }).click();
    const missingPickerCard = page.locator('.wiki-media-card').filter({ hasText: `Media ${seeded.media_id}` });
    const missingPickerFallback = missingPickerCard.getByRole('img', { name: mediaLabel });
    await expect(missingPickerFallback).toBeVisible();
    await expect(missingPickerFallback).toContainText(`Preview unavailable: ${mediaLabel}`);
    await expect(missingPickerCard.locator('img')).toHaveCount(0);

    await publicPage.goto(`/en/wiki/${articleSlug}`);
    const missingPublicFallback = publicPage.getByRole('img', { name: mediaLabel });
    await expect(missingPublicFallback).toBeVisible();
    await expect(missingPublicFallback).toHaveAttribute('data-media-fallback-state', 'unavailable');
    await expect(publicPage.locator('.wiki-editorial-image')).toHaveCount(0);
    await expectNoHorizontalOverflow(publicPage);
    await assertAccessibilitySmoke(publicPage);
  } finally {
    await publicContext.close();
  }
});

test('@wiki-media image-free Wiki draft preview remains accessible and contains no media fallback', async ({ page, context }) => {
  const pendingThumbnailRequests = trackWikiMediaThumbnailRequests(page);
  const email = uniqueEmail('wiki-media-none');
  const id = suffix();
  seedWikiMediaEditor(email, `Unused approved image ${id}`);
  const articleTitle = `Text only guide ${id}`;

  runArtisan('cache:clear');
  await login(page, email, password);
  await completeMfaChallenge(page, recoveryCode);
  await page.goto('/admin/wiki/articles/create');

  await page.getByLabel('Content type').fill('guide');
  await page.getByLabel('Display order').fill('6');
  await page.getByLabel('Title').first().fill(articleTitle);
  await page.getByLabel('Slug').first().fill(`text-only-guide-${id}`);
  await page.getByLabel('Summary').first().fill('Image-free Wiki acceptance preview.');
  await page.getByLabel('Markdown source').first().fill('# Text only\n\nThis article intentionally contains no media token.');
  await page.getByLabel('Title').nth(1).fill(`Poradnik tekstowy ${id}`);
  await page.getByLabel('Slug').nth(1).fill(`poradnik-tekstowy-${id}`);
  await page.getByLabel('Summary').nth(1).fill('Podglad Wiki bez obrazu.');
  await page.getByLabel('Markdown source').nth(1).fill('# Tylko tekst\n\nTen artykul celowo nie zawiera obrazu.');
  await page.getByLabel('Change note').fill('Image-free acceptance evidence.');
  await expect(page.getByLabel('Markdown source').first()).not.toHaveValue(/wiki-media:/u);
  await expect(page.getByLabel('Markdown source').nth(1)).not.toHaveValue(/wiki-media:/u);
  const createDraftButton = page.getByRole('button', { name: 'Create draft' });
  await createDraftButton.scrollIntoViewIfNeeded();
  await waitForWikiMediaThumbnailIdle(page, pendingThumbnailRequests);
  await createDraftButton.click();
  await expect(page.getByRole('status')).toContainText('Wiki article draft created.');

  const previewPromise = context.waitForEvent('page');
  await page.getByRole('link', { name: 'Preview EN' }).click();
  const preview = await previewPromise;
  await preview.waitForLoadState();
  await expect(preview.getByRole('heading', { name: articleTitle })).toBeVisible();
  await expect(preview.locator('.wiki-editorial-image')).toHaveCount(0);
  await expect(preview.locator('.wiki-image-placeholder')).toHaveCount(0);
  await expectNoHorizontalOverflow(preview);
  await assertAccessibilitySmoke(preview);
  await preview.close();
});
