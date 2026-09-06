import { test, expect } from '@playwright/test';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import {
  repoRoot, testedSha, runBinary, login, logout,
  assertAccessibilitySmoke,
} from './helpers.mjs';

const output = resolve(repoRoot, 'artifacts/acceptance/portal-review');
const viewports = [
  ['desktop', { width: 1440, height: 1000 }],
  ['phone', { width: 390, height: 844 }],
  ['tablet', { width: 820, height: 1180 }],
  ['wide', { width: 1920, height: 1080 }],
];
const records = [];

test.describe.configure({ retries: 0 });
// Never collect authentication traces, input values, session state or secrets.
test.use({ trace: 'off', screenshot: 'off', video: 'off' });

test.beforeAll(() => {
  runBinary('php', ['scripts/acceptance/seed-homepage-navigation-seo.php']);
  runBinary('php', ['scripts/acceptance/seed-browser-support-legal.php', 'seed-public']);
  mkdirSync(output, { recursive: true });
});

async function capture(page, name, status, { guestHtml = false } = {}) {
  await page.evaluate(() => document.fonts.ready);
  await expect(page.locator('main')).toBeVisible();
  await expect(page.locator('img')).not.toHaveCount(0);
  const brokenAssets = await page.locator('img').evaluateAll((images) => images
    .filter((image) => !image.complete || image.naturalWidth === 0).map((image) => image.getAttribute('src')));
  expect(brokenAssets, `${name}: missing image assets`).toEqual([]);
  const dimensions = await page.evaluate(() => ({ width: innerWidth, scrollWidth: document.documentElement.scrollWidth }));
  await page.screenshot({ path: resolve(output, `${name}.png`), fullPage: true, animations: 'disabled' });
  if (guestHtml) {
    const html = await page.evaluate(() => {
      const clone = document.documentElement.cloneNode(true);
      for (const input of clone.querySelectorAll('input')) input.removeAttribute('value');
      for (const node of clone.querySelectorAll('meta[name="csrf-token"], input[name="_token"]')) node.remove();
      for (const node of clone.querySelectorAll('[src], [href], [action]')) {
        for (const attribute of ['src', 'href', 'action']) {
          const value = node.getAttribute(attribute);
          if (!value) continue;
          try {
            const url = new URL(value, location.href);
            if (url.origin === location.origin) node.setAttribute(attribute, `${url.pathname}${url.search}${url.hash}`);
          } catch { /* Non-URL references remain untouched. */ }
        }
      }
      return `<!DOCTYPE html>\n${clone.outerHTML}`;
    });
    writeFileSync(resolve(output, `${name}.html`), html);
  }
  records.push({ screenshot: `${name}.png`, viewport: page.viewportSize(), status, ...dimensions });
  writeFileSync(resolve(output, 'manifest.json'), JSON.stringify({
    exactHead: testedSha, runtime: 'real Laravel HTTP; isolated synthetic fixtures', records,
  }, null, 2));
  expect(dimensions.scrollWidth, `${name}: document overflow`).toBeLessThanOrEqual(dimensions.width + 1);
}

for (const [viewportName, viewport] of viewports) {
  test(`@smoke @portal-review public surfaces render without overflow at ${viewportName}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.goto('/en');
    const destinations = await page.locator('.primary-nav a[href]').evaluateAll((links) => [...new Set(links
      .map((link) => new URL(link.href, location.href))
      .filter((url) => url.origin === location.origin && /^\/en\//u.test(url.pathname))
      .map((url) => url.pathname))]);
    const paths = ['desktop', 'phone'].includes(viewportName)
      ? ['/en', '/pl', '/login', '/register', ...destinations]
      : ['/en', '/pl', '/login', '/en/highscores', '/en/wiki'];
    for (const path of paths) {
      const response = await page.goto(path);
      if (path === '/en/wiki/catalog' && response?.status() === 503) {
        await expect(page.locator('main')).toContainText(/unavailable/iu);
      } else {
        expect(response?.status(), path).toBe(200);
      }
      await capture(page, `${viewportName}-${path.slice(1).replaceAll('/', '-')}`, response.status(), { guestHtml: true });
    }
    await page.goto('/en');
    await expect(page.locator('#home-hero-title')).toHaveText('OTERYN');
    await expect(page.locator('.production-hero-world')).toBeInViewport();
    await assertAccessibilitySmoke(page);
  });
}

test('@smoke @portal-review mobile keyboard menu, search, locale and no-JS fallback', async ({ page, browser }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/en');
  const menu = page.locator('.mobile-nav');
  const trigger = menu.locator('summary');
  await trigger.focus();
  await page.keyboard.press('Enter');
  await expect(menu).toHaveAttribute('open', '');
  await page.keyboard.press('Tab');
  await expect(menu.locator('nav a').first()).toBeFocused();
  await capture(page, 'phone-menu-open', 200);
  await page.keyboard.press('Escape');
  await expect(menu).not.toHaveAttribute('open', '');
  await expect(trigger).toBeFocused();
  await trigger.click();
  await menu.locator('nav a[href$="/highscores"]').click();
  await expect(page).toHaveURL(/\/en\/highscores$/u);
  await page.goto('/en');
  await page.getByLabel('Character name').fill('Acceptance Hero');
  await page.getByRole('button', { name: 'Search', exact: true }).click();
  await expect(page).toHaveURL(/\/characters\/Acceptance%20Hero$/u);
  await expect(page.getByRole('heading', { name: 'Acceptance Hero', exact: true })).toBeVisible();
  await capture(page, 'phone-character-profile', 200);
  await page.goto('/en');
  await trigger.click();
  await menu.locator('.language-switcher a[lang="pl"]').click();
  await expect(page).toHaveURL(/\/pl$/u);
  await expect(page.locator('html')).toHaveAttribute('lang', 'pl');

  const context = await browser.newContext({ baseURL: new URL(page.url()).origin, javaScriptEnabled: false, viewport: { width: 390, height: 844 } });
  try {
    const fallback = await context.newPage();
    await fallback.goto('/en');
    await fallback.locator('.mobile-nav summary').click();
    await expect(fallback.locator('.mobile-nav-panel')).toBeVisible();
    await fallback.locator('.mobile-nav-panel nav a[href$="/servers"]').click();
    await expect(fallback).toHaveURL(/\/en\/servers$/u);
  } finally {
    await context.close();
  }
});

test('@smoke @portal-review actual authenticated account and player navigation', async ({ page }) => {
  const email = 'portal.review@example.test';
  const password = 'Portal-Review-9!Only';
  // Existing acceptance-only fixture provisions a disposable, non-MFA account.
  runBinary('php', ['scripts/acceptance/seed-account-overview-state.php', email, password, 'ready']);
  await login(page, email, password);
  for (const [name, viewport] of viewports.filter(([name]) => ['desktop', 'phone'].includes(name))) {
    await page.setViewportSize(viewport);
    for (const path of ['/account', '/account/characters/create', '/en']) {
      const response = await page.goto(path);
      expect(response?.status(), path).toBe(200);
      await capture(page, `${name}-authenticated-${path.slice(1).replaceAll('/', '-')}`, response.status());
      await assertAccessibilitySmoke(page);
    }
    await expect(page.locator('.preview-hero-actions a[href$="/account"]')).toBeVisible();
    await expect(page.locator('.preview-hero-actions a[href$="/register"]')).toHaveCount(0);
  }
  await logout(page);
  await page.goto('/account');
  await expect(page).toHaveURL(/\/login$/u);
});

test('@smoke @portal-review empty, unavailable, form error, narrow and reduced-motion states', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  for (const scenario of ['empty', 'news-outage']) {
    await page.setExtraHTTPHeaders({ 'X-Oteryn-Acceptance-Today-Scenario': scenario });
    const response = await page.goto('/en/today');
    expect(response?.status()).toBe(200);
    if (scenario === 'empty') {
      for (const kind of ['announcements', 'events', 'news']) {
        await expect(page.locator(`[data-today-card="${kind}"]`)).toHaveAttribute('data-content-state', 'empty');
      }
    } else {
      await expect(page.locator('[data-today-state="partial"]')).toBeVisible();
      await expect(page.locator('[data-today-card="news"]')).toHaveAttribute('data-content-state', 'unavailable');
    }
    await capture(page, `phone-today-${scenario}`, response.status());
  }
  await page.setExtraHTTPHeaders({});
  const missing = await page.goto('/portal-redesign-missing-page');
  expect(missing?.status()).toBe(404);
  await capture(page, 'phone-not-found', missing.status());
  await page.goto('/login');
  await page.locator('input[name="email"]').fill('unknown-portal-review@example.test');
  await page.locator('input[name="password"]').fill('Invalid-Review-9!Password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  // Remove test credentials from the visible fields before taking evidence.
  await page.locator('input[name="email"]').fill('');
  await page.locator('input[name="password"]').fill('');
  await capture(page, 'phone-login-error', 200);
  await page.setViewportSize({ width: 320, height: 740 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  for (const path of ['/pl', '/login', '/en/highscores']) {
    const response = await page.goto(path);
    expect(response?.status()).toBe(200);
    await capture(page, `narrow-${path.slice(1).replaceAll('/', '-')}`, response.status());
  }
  await page.setViewportSize({ width: 820, height: 1180 });
  await page.goto('/pl');
  await page.evaluate(() => { document.documentElement.style.fontSize = '200%'; });
  await capture(page, 'tablet-text-200-percent', 200);
});
