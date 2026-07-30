import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  completeMfaChallenge,
  installDiagnostics,
  login,
  logout,
  runBinary,
} from './helpers.mjs';

const wikiScaleMarker = '@portal-content-scale-wiki long bilingual articles stable multi-page search and administrator pagination remain contained';
const adminEmail = 'content-scale-wiki-administrator@example.test';
const adminPassword = 'ContentScaleWiki!234';
const adminRecoveryCode = 'CONTENT-SCALE-WIKI-01';

test.setTimeout(180_000);
test.describe.configure({ retries: 0, mode: 'serial' });

test.beforeEach(async ({ page }) => {
  page.__wikiScaleFixture = JSON.parse(
    runBinary('php', ['scripts/acceptance/seed-content-scale-wiki.php']),
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
      throw new Error(`Expected ${selector} containment for Wiki scale evidence.`);
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
      elementLeft: elementRect.left,
      elementRight: elementRect.right,
      elementTop: elementRect.top,
      elementBottom: elementRect.bottom,
      containerLeft: containerRect.left,
      containerRight: containerRect.right,
      containerTop: containerRect.top,
      containerBottom: containerRect.bottom,
    };
  }, containerSelector);

  expect(metrics.lineCount).toBeGreaterThanOrEqual(2);
  expect(metrics.elementLeft).toBeGreaterThanOrEqual(metrics.containerLeft - 1);
  expect(metrics.elementRight).toBeLessThanOrEqual(metrics.containerRight + 1);
  expect(metrics.elementTop).toBeGreaterThanOrEqual(metrics.containerTop - 1);
  expect(metrics.elementBottom).toBeLessThanOrEqual(metrics.containerBottom + 1);
}

async function expectTableContainment(locator) {
  const metrics = await locator.evaluate((element) => {
    const region = element.closest('.table-region');
    const table = region?.querySelector('table');
    if (!(region instanceof HTMLElement) || !(table instanceof HTMLTableElement)) {
      throw new Error('Expected Wiki record inside a table-region table.');
    }
    const elementRect = element.getBoundingClientRect();
    const tableRect = table.getBoundingClientRect();
    return {
      overflowX: getComputedStyle(region).overflowX,
      clientWidth: region.clientWidth,
      scrollWidth: region.scrollWidth,
      elementLeft: elementRect.left,
      elementRight: elementRect.right,
      tableLeft: tableRect.left,
      tableRight: tableRect.right,
    };
  });

  expect(['auto', 'scroll']).toContain(metrics.overflowX);
  expect(metrics.scrollWidth).toBeGreaterThanOrEqual(metrics.clientWidth);
  expect(metrics.elementLeft).toBeGreaterThanOrEqual(metrics.tableLeft - 1);
  expect(metrics.elementRight).toBeLessThanOrEqual(metrics.tableRight + 1);
}

async function expectTextareaWrappingAndContainment(locator) {
  await expect(locator).toBeVisible();
  const metrics = await locator.evaluate((element) => {
    const container = element.closest('.form-field');
    if (!(element instanceof HTMLTextAreaElement) || !(container instanceof HTMLElement)) {
      throw new Error('Expected Wiki markdown textarea inside a form-field.');
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

async function searchResultHrefs(page) {
  return page.locator('.card-grid > article.card h2 a').evaluateAll((links) => links.map((link) => link.getAttribute('href')));
}

async function signInAsWikiEditor(page) {
  runBinary('php', [
    'scripts/acceptance/seed-browser-wiki-reconciliation.php',
    'seed-identity',
    adminEmail,
    adminPassword,
    adminRecoveryCode,
    'confirmed',
    'wiki.access,wiki.articles.manage',
  ]);
  await login(page, adminEmail, adminPassword);
  await completeMfaChallenge(page, adminRecoveryCode);
}

// Evidence marker: @portal-content-scale-wiki long bilingual articles stable multi-page search and administrator pagination remain contained
test(wikiScaleMarker, async ({ page }) => {
  const fixture = page.__wikiScaleFixture;

  for (const localized of [
    {
      locale: 'en',
      slug: fixture.wiki_english_slug,
      title: fixture.wiki_english_title,
      summary: fixture.wiki_english_summary,
      body: fixture.wiki_english_body,
    },
    {
      locale: 'pl',
      slug: fixture.wiki_polish_slug,
      title: fixture.wiki_polish_title,
      summary: fixture.wiki_polish_summary,
      body: fixture.wiki_polish_body,
    },
  ]) {
    let response = await page.goto(`/${localized.locale}/wiki/${localized.slug}`);
    expect(response?.status()).toBe(200);
    await expect(page.locator('html')).toHaveAttribute('lang', localized.locale);

    const heading = page.getByRole('heading', { level: 1, name: localized.title, exact: true });
    const summary = page.getByText(localized.summary, { exact: true });
    const body = page.locator('.wiki-markdown p').filter({ hasText: localized.body }).first();
    await expect(body).toHaveText(localized.body);
    await expectReadableWrappingAndContainment(heading, '.wiki-article-header');
    await expectReadableWrappingAndContainment(summary, '.wiki-article-header');
    await expectReadableWrappingAndContainment(body, '.wiki-markdown');
    await expectNoHorizontalOverflow(page);

    response = await page.goto(`/${localized.locale}/wiki/search?q=${encodeURIComponent(fixture.wiki_query)}`);
    expect(response?.status()).toBe(200);
    const firstPageCards = page.locator('.card-grid > article.card');
    await expect(firstPageCards).toHaveCount(12);
    const firstPageHrefs = await searchResultHrefs(page);
    expect(new Set(firstPageHrefs).size).toBe(12);

    const next = page.locator('a[rel="next"]');
    await expect(next).toBeVisible();
    await next.click();
    await expect(page).toHaveURL(/page=2/u);
    const secondPageCards = page.locator('.card-grid > article.card');
    await expect(secondPageCards).toHaveCount(12);
    const secondPageHrefs = await searchResultHrefs(page);
    expect(new Set(secondPageHrefs).size).toBe(12);
    expect(secondPageHrefs.some((href) => firstPageHrefs.includes(href))).toBe(false);

    const secondPageUrl = page.url();
    await page.goto(secondPageUrl);
    expect(await searchResultHrefs(page)).toEqual(secondPageHrefs);
    await expectNoHorizontalOverflow(page);
  }

  await signInAsWikiEditor(page);
  let response = await page.goto('/admin/wiki/articles');
  expect(response?.status()).toBe(200);
  const longTitle = page.getByText(fixture.wiki_english_title, { exact: true });
  await expectReadableWrappingAndContainment(longTitle, 'td');
  await expectTableContainment(longTitle);
  await expectNoHorizontalOverflow(page);

  response = await page.goto('/admin/wiki/articles?page=2');
  expect(response?.status()).toBe(200);
  for (const id of fixture.wiki_admin_page_two_ids) {
    await expect(page.getByText(`ID ${id} · version 1`, { exact: true })).toBeVisible();
  }
  await expectNoHorizontalOverflow(page);

  response = await page.goto(`/admin/wiki/articles/${fixture.wiki_article_id}/edit`);
  expect(response?.status()).toBe(200);
  await expect(page.getByLabel('Title').first()).toHaveValue(fixture.wiki_english_title);
  const markdown = page.getByLabel('Markdown source').first();
  await expect(markdown).toContainText(fixture.wiki_english_body);
  await expectTextareaWrappingAndContainment(markdown);
  await expectNoHorizontalOverflow(page);
  await logout(page);

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});
