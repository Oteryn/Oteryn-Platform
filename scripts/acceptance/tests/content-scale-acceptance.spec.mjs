import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  installDiagnostics,
  runBinary,
} from './helpers.mjs';

const contentScaleMarker = '@portal-content-scale long bilingual news and managed pages wrap and remain contained';

test.setTimeout(120_000);
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
  }
});
