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

test.setTimeout(120_000);
test.describe.configure({ mode: 'serial', retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@wiki-media exact Wiki editor discovers inserts previews and publishes private EditorialMedia responsively', async ({ page, context }) => {
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
  await expect(page.getByRole('status')).toContainText('Wiki article submitted for review.');
  await page.getByRole('button', { name: 'Publish', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Wiki article published.');

  await context.clearCookies();
  await page.goto(`/en/wiki/${articleSlug}`);
  const publicImage = page.getByRole('img', { name: mediaLabel });
  await expect(publicImage).toBeVisible();
  expect(await publicImage.evaluate((image) => image.naturalWidth)).toBeGreaterThan(0);
  const imageResponse = await page.request.get(await publicImage.getAttribute('src'));
  expect(imageResponse.status()).toBe(200);
  expect(imageResponse.headers()['x-content-type-options']).toBe('nosniff');
  expect(imageResponse.headers()['cache-control']).toContain('must-revalidate');
  await expectNoHorizontalOverflow(page);
  await assertAccessibilitySmoke(page);

  await page.goto(`/pl/wiki/${polishSlug}`);
  await expect(page.getByRole('img', { name: `Most Oteryn ${id}` })).toBeVisible();
  await expectNoHorizontalOverflow(page);
});
