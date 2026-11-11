import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  completeMfaChallenge,
  installDiagnostics,
  login,
  logout,
  runBinary,
} from './helpers.mjs';

const eventScaleMarker = '@portal-content-scale-events long bilingual event content and bounded administrator pagination remain contained';
const adminEmail = 'content-scale-events-administrator@example.test';
const adminPassword = 'ContentScaleEvents!234';
const adminRecoveryCode = 'CONTENT-SCALE-EVENTS-01';

test.setTimeout(150_000);
test.describe.configure({ retries: 0, mode: 'serial' });

test.beforeEach(async ({ page }) => {
  page.__eventScaleFixture = JSON.parse(
    runBinary('php', ['scripts/acceptance/seed-content-scale-events.php']),
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
      throw new Error(`Expected ${selector} containment for event scale evidence.`);
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
    const region = element.closest('.table-wrap');
    const table = region?.querySelector('table');
    if (!(region instanceof HTMLElement) || !(table instanceof HTMLTableElement)) {
      throw new Error('Expected event cell inside a table-wrap table.');
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

async function signInAsEventManager(page) {
  runBinary('php', [
    'scripts/acceptance/seed-browser-announcements.php',
    'seed-identity',
    adminEmail,
    adminPassword,
    adminRecoveryCode,
    'confirmed',
    'events.manage',
  ]);
  await login(page, adminEmail, adminPassword);
  await completeMfaChallenge(page, adminRecoveryCode);
}

// Evidence marker: @portal-content-scale-events long bilingual event content and bounded administrator pagination remain contained
test(eventScaleMarker, async ({ page }) => {
  const fixture = page.__eventScaleFixture;

  for (const localized of [
    {
      locale: 'en',
      slug: fixture.event_english_slug,
      title: fixture.event_english_title,
      summary: fixture.event_english_summary,
      body: fixture.event_english_body,
    },
    {
      locale: 'pl',
      slug: fixture.event_polish_slug,
      title: fixture.event_polish_title,
      summary: fixture.event_polish_summary,
      body: fixture.event_polish_body,
    },
  ]) {
    let response = await page.goto(`/${localized.locale}/events`);
    expect(response?.status()).toBe(200);
    await expect(page.locator('html')).toHaveAttribute('lang', localized.locale);

    const eventCard = page.locator('article.card').filter({ hasText: localized.title }).first();
    const eventLink = eventCard.getByRole('link', { name: localized.title, exact: true });
    const eventSummary = eventCard.getByText(localized.summary, { exact: true });
    await expectReadableWrappingAndContainment(eventLink, 'article.card');
    await expectReadableWrappingAndContainment(eventSummary, 'article.card');
    await expectNoHorizontalOverflow(page);

    response = await page.goto(`/${localized.locale}/events/${localized.slug}`);
    expect(response?.status()).toBe(200);
    const article = page.locator('main article').first();
    const heading = article.getByRole('heading', { level: 1, name: localized.title, exact: true });
    const summary = article.getByText(localized.summary, { exact: true });
    const body = article.locator('.content-copy p').first();
    await expect(body).toHaveText(localized.body);
    await expectReadableWrappingAndContainment(heading, '.page-header');
    await expectReadableWrappingAndContainment(summary, '.page-header');
    await expectReadableWrappingAndContainment(body, '.content-copy');
    await expectNoHorizontalOverflow(page);
  }

  await signInAsEventManager(page);
  let response = await page.goto('/admin/events');
  expect(response?.status()).toBe(200);
  const longEventId = page.getByRole('cell', { name: String(fixture.event_id), exact: true });
  await expectTableContainment(longEventId);
  await expectNoHorizontalOverflow(page);

  response = await page.goto('/admin/events?page=2');
  expect(response?.status()).toBe(200);
  for (const id of fixture.event_page_two_ids) {
    await expect(page.getByRole('cell', { name: String(id), exact: true })).toBeVisible();
  }
  await expectNoHorizontalOverflow(page);
  await logout(page);

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});
