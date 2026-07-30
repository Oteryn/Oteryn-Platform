import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  completeMfaChallenge,
  installDiagnostics,
  login,
  logout,
  runBinary,
} from './helpers.mjs';

const contentScaleMarker = '@portal-content-scale long bilingual news and managed pages wrap and remain contained';
const adminScaleMarker = '@portal-content-scale-admin long CMS localization and bounded administrator pagination remain contained';
const adminEmail = 'content-scale-administrator@example.test';
const adminPassword = 'ContentScaleAdmin!234';
const adminRecoveryCode = 'CONTENT-SCALE-ADMIN-01';

test.setTimeout(180_000);
test.describe.configure({ retries: 0, mode: 'serial' });

test.beforeEach(async ({ page }) => {
  page.__contentScaleFixture = JSON.parse(
    runBinary('php', ['scripts/acceptance/seed-content-scale.php']),
  );
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

async function expectNoHorizontalOverflow(page) {
  const dimensions = await page.evaluate(() => ({
    documentWidth: document.documentElement.scrollWidth,
    viewportWidth: document.documentElement.clientWidth,
  }));

  expect(dimensions.documentWidth).toBeLessThanOrEqual(dimensions.viewportWidth + 1);
}

async function expectReadableWrappingAndContainment(locator, containerSelector) {
  await expect(locator).toBeVisible();

  const metrics = await locator.evaluate((element, selector) => {
    const container = element.closest(selector);
    if (!(container instanceof HTMLElement)) {
      throw new Error(`Expected ${selector} containment for long-content evidence.`);
    }

    const range = document.createRange();
    range.selectNodeContents(element);
    const lineTops = [];
    for (const rect of range.getClientRects()) {
      if (rect.width <= 0 || rect.height <= 0) continue;
      if (!lineTops.some((top) => Math.abs(top - rect.top) < 1)) lineTops.push(rect.top);
    }

    const elementRect = element.getBoundingClientRect();
    const containerRect = container.getBoundingClientRect();

    return {
      lineCount: lineTops.length,
      elementRect: {
        left: elementRect.left,
        right: elementRect.right,
        top: elementRect.top,
        bottom: elementRect.bottom,
      },
      containerRect: {
        left: containerRect.left,
        right: containerRect.right,
        top: containerRect.top,
        bottom: containerRect.bottom,
      },
    };
  }, containerSelector);

  expect(metrics.lineCount).toBeGreaterThanOrEqual(2);
  expect(metrics.elementRect.left).toBeGreaterThanOrEqual(metrics.containerRect.left - 1);
  expect(metrics.elementRect.right).toBeLessThanOrEqual(metrics.containerRect.right + 1);
  expect(metrics.elementRect.top).toBeGreaterThanOrEqual(metrics.containerRect.top - 1);
  expect(metrics.elementRect.bottom).toBeLessThanOrEqual(metrics.containerRect.bottom + 1);
}

async function expectScrollableTableContainment(locator, regionSelector) {
  const metrics = await locator.evaluate((element, selector) => {
    const region = element.closest(selector);
    const table = region?.querySelector('table');
    if (!(region instanceof HTMLElement) || !(table instanceof HTMLTableElement)) {
      throw new Error(`Expected table containment inside ${selector}.`);
    }

    const regionRect = region.getBoundingClientRect();
    const tableRect = table.getBoundingClientRect();
    const elementRect = element.getBoundingClientRect();

    return {
      overflowX: getComputedStyle(region).overflowX,
      clientWidth: region.clientWidth,
      scrollWidth: region.scrollWidth,
      regionLeft: regionRect.left,
      regionRight: regionRect.right,
      tableLeft: tableRect.left,
      tableRight: tableRect.right,
      elementLeft: elementRect.left,
      elementRight: elementRect.right,
    };
  }, regionSelector);

  expect(['auto', 'scroll']).toContain(metrics.overflowX);
  expect(metrics.scrollWidth).toBeGreaterThanOrEqual(metrics.clientWidth);
  expect(metrics.tableLeft).toBeGreaterThanOrEqual(metrics.regionLeft - 1);
  expect(metrics.tableRight).toBeLessThanOrEqual(metrics.regionLeft + metrics.scrollWidth + 1);
  expect(metrics.elementLeft).toBeGreaterThanOrEqual(metrics.tableLeft - 1);
  expect(metrics.elementRight).toBeLessThanOrEqual(metrics.tableRight + 1);
}

async function expectTextareaWrappingAndContainment(locator) {
  await expect(locator).toBeVisible();

  const metrics = await locator.evaluate((element) => {
    const container = element.closest('.form-field');
    if (!(element instanceof HTMLTextAreaElement) || !(container instanceof HTMLElement)) {
      throw new Error('Expected a textarea contained by a form-field.');
    }

    const elementRect = element.getBoundingClientRect();
    const containerRect = container.getBoundingClientRect();

    return {
      scrollWidth: element.scrollWidth,
      clientWidth: element.clientWidth,
      scrollHeight: element.scrollHeight,
      clientHeight: element.clientHeight,
      elementLeft: elementRect.left,
      elementRight: elementRect.right,
      containerLeft: containerRect.left,
      containerRight: containerRect.right,
    };
  });

  expect(metrics.scrollWidth).toBeLessThanOrEqual(metrics.clientWidth + 1);
  expect(metrics.scrollHeight).toBeGreaterThan(metrics.clientHeight);
  expect(metrics.elementLeft).toBeGreaterThanOrEqual(metrics.containerLeft - 1);
  expect(metrics.elementRight).toBeLessThanOrEqual(metrics.containerRight + 1);
}

async function expectLongDetail(page, { path, locale, title, body }) {
  const response = await page.goto(path);
  expect(response?.status()).toBe(200);
  await expect(page.locator('html')).toHaveAttribute('lang', locale);

  const article = page.locator('main article').first();
  const heading = article.getByRole('heading', { level: 1, name: title, exact: true });
  const content = article.locator('.card .prose-text');

  await expect(content).toHaveText(body);
  await expectReadableWrappingAndContainment(heading, 'article');
  await expectReadableWrappingAndContainment(content, '.card');
  await expectNoHorizontalOverflow(page);
}

async function expectLongHomepageContent(page, { locale, title, body }) {
  const response = await page.goto(`/${locale}`);
  expect(response?.status()).toBe(200);
  await expect(page.locator('html')).toHaveAttribute('lang', locale);

  const newsPanel = page.locator('.production-news-panel');
  const newsArticle = newsPanel.locator('.production-news-list article').filter({ hasText: title }).first();
  const titleLink = newsArticle.getByRole('link', { name: title, exact: true });
  const excerpt = newsArticle.locator('p').last();

  await expect(excerpt).toContainText(body.slice(0, 80));
  await expectReadableWrappingAndContainment(titleLink, 'article');
  await expectReadableWrappingAndContainment(excerpt, 'article');

  const announcement = page.locator('[data-content-state="AVAILABLE"] .notice').filter({ hasText: title }).first();
  const announcementHeading = announcement.getByRole('heading', { level: 3, name: title, exact: true });
  const announcementBody = announcement.locator('p').last();
  await expect(announcementBody).toHaveText(body);
  await expectReadableWrappingAndContainment(announcementHeading, 'article.notice');
  await expectReadableWrappingAndContainment(announcementBody, 'article.notice');
  await expectNoHorizontalOverflow(page);
}

async function signInAsContentScaleAdministrator(page) {
  runBinary('php', [
    'scripts/acceptance/seed-browser-announcements.php',
    'seed-identity',
    adminEmail,
    adminPassword,
    adminRecoveryCode,
    'confirmed',
    'cms.news.manage,portal.announcements.manage',
  ]);

  await login(page, adminEmail, adminPassword);
  await completeMfaChallenge(page, adminRecoveryCode);
}

// Evidence marker: @portal-content-scale long bilingual news and managed pages wrap and remain contained
test(contentScaleMarker, async ({ page }) => {
  const fixture = page.__contentScaleFixture;

  for (const localized of [
    {
      locale: 'en',
      title: fixture.english_title,
      body: fixture.english_body,
    },
    {
      locale: 'pl',
      title: fixture.polish_title,
      body: fixture.polish_body,
    },
  ]) {
    const indexResponse = await page.goto(`/${localized.locale}/news`);
    expect(indexResponse?.status()).toBe(200);
    await expect(page.locator('html')).toHaveAttribute('lang', localized.locale);

    const newsLink = page.getByRole('link', { name: localized.title, exact: true });
    await expectReadableWrappingAndContainment(newsLink, 'article.card');
    await expectNoHorizontalOverflow(page);

    await expectLongDetail(page, {
      path: `/${localized.locale}/news/${fixture.news_slug}`,
      locale: localized.locale,
      title: localized.title,
      body: localized.body,
    });

    await expectLongDetail(page, {
      path: `/${localized.locale}/pages/${fixture.page_slug}`,
      locale: localized.locale,
      title: localized.title,
      body: localized.body,
    });

    await expectLongHomepageContent(page, localized);

    await expectLongDetail(page, {
      path: `/${localized.locale}/legal/terms`,
      locale: localized.locale,
      title: localized.title,
      body: localized.body,
    });
  }

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});

// Evidence marker: @portal-content-scale-admin long CMS localization and bounded administrator pagination remain contained
test(adminScaleMarker, async ({ page }) => {
  const fixture = page.__contentScaleFixture;
  await signInAsContentScaleAdministrator(page);

  let response = await page.goto('/admin/news');
  expect(response?.status()).toBe(200);
  const longNewsCell = page.getByRole('cell', { name: fixture.english_title, exact: true });
  await expectReadableWrappingAndContainment(longNewsCell, 'td');
  await expectScrollableTableContainment(longNewsCell, '.table-region');
  await expectNoHorizontalOverflow(page);

  response = await page.goto('/admin/news?page=2');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('cell', { name: 'Content Scale News 002', exact: true })).toBeVisible();
  await expect(page.getByRole('cell', { name: 'Content Scale News 001', exact: true })).toBeVisible();
  await expectNoHorizontalOverflow(page);

  response = await page.goto(`/admin/news/${fixture.news_id}/translations/pl`);
  expect(response?.status()).toBe(200);
  const sourceSection = page.locator('section[aria-labelledby="english-source-heading"]');
  const sourceTitle = sourceSection.getByText(fixture.english_title, { exact: true });
  const sourceBody = sourceSection.locator('pre.content-body');
  await expect(sourceBody).toHaveText(fixture.english_body);
  await expectReadableWrappingAndContainment(sourceTitle, 'section');
  await expectReadableWrappingAndContainment(sourceBody, 'section');
  await expect(page.getByLabel('Polish title')).toHaveValue(fixture.polish_title);
  const polishBody = page.getByLabel('Polish content (plain text)');
  await expect(polishBody).toHaveValue(fixture.polish_body);
  await expectTextareaWrappingAndContainment(polishBody);
  await expectNoHorizontalOverflow(page);

  response = await page.goto('/admin/announcements');
  expect(response?.status()).toBe(200);
  const longAnnouncementCell = page.getByRole('cell', { name: fixture.english_title, exact: true });
  await expectReadableWrappingAndContainment(longAnnouncementCell, 'td');
  await expectScrollableTableContainment(longAnnouncementCell, '.table-wrap');
  await expectNoHorizontalOverflow(page);

  response = await page.goto('/admin/announcements?page=2');
  expect(response?.status()).toBe(200);
  await expect(page.getByRole('cell', { name: 'Content Scale Announcement 002', exact: true })).toBeVisible();
  await expect(page.getByRole('cell', { name: 'Content Scale Announcement 001', exact: true })).toBeVisible();
  await expectNoHorizontalOverflow(page);

  await logout(page);
  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});
