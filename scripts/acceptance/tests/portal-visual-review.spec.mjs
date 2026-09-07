import { test, expect } from '@playwright/test';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { repoRoot, testedSha, runBinary, register, login, logout, completeMfaChallenge, uniqueEmail, uniqueCharacterName, assertAccessibilitySmoke } from './helpers.mjs';
import { revealPublicNavigationLink } from './portal-navigation.mjs';

const output = resolve(repoRoot, 'artifacts/acceptance/portal-review');
const viewports = [
  ['desktop', { width: 1440, height: 1000 }],
  ['phone', { width: 390, height: 844 }],
  ['tablet', { width: 820, height: 1180 }],
  ['wide', { width: 1920, height: 1080 }],
];
const email = uniqueEmail('portal.review');
const password = 'Portal-Review-9!Only';
let auction;

test.setTimeout(180_000);
test.describe.configure({ retries: 0 });
// No session exports, form values, auth HTML, recovery URLs or raw traces.
test.use({ trace: 'off', screenshot: 'off', video: 'off' });

test.beforeAll(() => {
  mkdirSync(output, { recursive: true });
  const routes = JSON.parse(runBinary('php', ['artisan', 'route:list', '--json', '--except-vendor']));
  const vendorConsent = JSON.parse(runBinary('php', ['artisan', 'route:list', '--json', '--name=passport.authorizations.authorize']));
  routes.push(...vendorConsent);
  writeFileSync(resolve(output, 'routes.json'), JSON.stringify(routes.filter((route) =>
    route.method.includes('GET') && !/^(admin|api|internal|health|sanctum)(\/|$)/u.test(route.uri))
    .map(({ method, uri, name }) => ({ method, uri, name })), null, 2));
  for (const args of [
    ['scripts/acceptance/seed.php', 'seed'],
    ['scripts/acceptance/seed-homepage-navigation-seo.php'],
    ['scripts/acceptance/seed-browser-support-legal.php', 'seed-public'],
    ['scripts/acceptance/seed-public-wiki.php'],
    ['scripts/acceptance/seed-game-catalog.php'],
    ['scripts/acceptance/seed-downloads-state.php', 'seed-portability'],
  ]) runBinary('php', args);
  auction = JSON.parse(runBinary('php', ['scripts/acceptance/seed-marketplace.php', uniqueEmail('portal.bazaar'), password]));
});

async function capture(page, name, status) {
  await page.evaluate(() => document.fonts.ready);
  await expect(page.locator('main')).toBeVisible();
  await expect(page.locator('link[href$="/css/portal-art-direction.css"]')).toHaveCount(1);
  const defects = await page.evaluate(() => {
    const missing = [...document.images].filter((image) => !image.complete || image.naturalWidth === 0)
      .map((image) => image.getAttribute('src')?.startsWith('data:') ? '[inline image]' : image.getAttribute('src'));
    const unlabelled = [...document.querySelectorAll('input:not([type="hidden"]), select, textarea')]
      .filter((control) => !control.labels?.length && !control.hasAttribute('aria-label') && !control.hasAttribute('aria-labelledby'))
      .map((control) => control.id || control.name || control.tagName);
    return { missing, unlabelled, width: innerWidth, scrollWidth: document.documentElement.scrollWidth,
      family: document.body.dataset.portalFamily || 'error', language: document.documentElement.lang };
  });
  await page.screenshot({ path: resolve(output, `${name}.png`), fullPage: true, animations: 'disabled',
    // Mask by locator before rasterization, not by editing a screenshot afterwards.
    mask: [page.locator('.mfa-qr-code, .mfa-manual-setup code, .recovery-codes, .recovery-key-value, .secure-output')],
  });
  const manifestPath = resolve(output, 'manifest.json');
  const previous = existsSync(manifestPath) ? JSON.parse(readFileSync(manifestPath, 'utf8')) : {};
  const records = previous.exactHead === testedSha ? previous.records.filter((record) => record.screenshot !== `${name}.png`) : [];
  records.push({ screenshot: `${name}.png`, viewport: page.viewportSize(), status, ...defects });
  writeFileSync(manifestPath, JSON.stringify({ exactHead: testedSha,
    runtime: 'real Laravel HTTP; acceptance-only synthetic fixtures; no production data', records }, null, 2));
  expect.soft(defects.missing, `${name}: broken assets`).toEqual([]);
  expect.soft(defects.unlabelled, `${name}: unlabelled controls`).toEqual([]);
  expect.soft(defects.scrollWidth, `${name}: document overflow`).toBeLessThanOrEqual(defects.width + 1);
}

async function visit(page, path, name, expectedStatus = 200) {
  const response = await page.goto(path);
  expect.soft(response?.status(), name).toBe(expectedStatus);
  await capture(page, name, response?.status());
}

const extraPublic = [
  ['/en/pages/about-oteryn', 'about'],
  ['/en/news/long-title-layout-probe', 'news-detail'],
  ['/en/events/acceptance-tournament', 'event-detail'],
  ['/en/characters/Acceptance%20Hero', 'character-profile'],
  ['/en/guilds/Acceptance%20Guild', 'guild-detail'],
  ['/en/wiki/first-login', 'wiki-article'],
  ['/en/wiki/category/getting-started', 'wiki-category'],
  ['/en/wiki/search?q=first+login', 'wiki-search'],
  ['/en/wiki/search?q=no-such-guide-portal-review', 'wiki-empty'],
  ['/en/wiki/items/fixture-sword', 'catalog-item'],
  ['/en/wiki/creatures/fixture-rat', 'catalog-creature'],
  ['/login', 'login'], ['/register', 'register'], ['/forgot-password', 'forgot-password'],
  ['/reset-password/acceptance-placeholder', 'reset-password-form'], ['/recovery-key', 'recovery-key-form'],
];

for (const [viewportName, viewport] of viewports) {
  test(`@smoke @portal-review real public page families at ${viewportName}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.goto('/en');
    const destinations = await page.locator('.primary-nav a[href], .public-footer-group a[href]').evaluateAll((links) =>
      [...new Set(links.map((link) => new URL(link.href, location.href))
        .filter((url) => url.origin === location.origin && url.pathname.startsWith('/en/'))
        .map((url) => url.pathname))]);
    const pages = ['desktop', 'phone'].includes(viewportName)
      ? [['/en', 'home'], ['/pl', 'home-pl'], ...destinations.map((path) => [path, path.slice(4).replaceAll('/', '-')]),
        ...extraPublic, [`/bazaar/${auction.auction_id}`, 'bazaar-detail'], ['/register?locale=pl', 'register-pl']]
      : [['/en', 'home'], ['/pl', 'home-pl'], ['/login', 'login'], ['/en/highscores', 'highscores'],
        ['/en/wiki/first-login', 'wiki-article'], ['/en/download', 'download'], ['/en/servers', 'servers']];
    for (const [path, name] of pages) await visit(page, path, `${viewportName}-${name}`);
    await page.goto('/en');
    await expect(page.locator('#home-hero-title')).toHaveText('OTERYN');
    await expect(page.locator('.realm-pulse')).toBeInViewport();
    await assertAccessibilitySmoke(page);
  });
}

test('@smoke @portal-review grouped navigation, mobile keyboard, locale, search and no-JS fallback', async ({ page, browser }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto('/en');
  await revealPublicNavigationLink(page, 'Highscores');
  await capture(page, 'desktop-world-menu-open', 200);
  await page.keyboard.press('Escape');
  await page.setViewportSize({ width: 390, height: 844 });
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
  await (await revealPublicNavigationLink(page, 'Highscores')).click();
  await expect(page).toHaveURL(/\/en\/highscores$/u);
  await page.goto('/en');
  await page.getByLabel('Character name').fill('Acceptance Hero');
  await page.getByRole('button', { name: 'Search', exact: true }).click();
  await expect(page).toHaveURL(/\/characters\/Acceptance%20Hero$/u);
  await page.goto('/en');
  await trigger.click();
  await menu.locator('.language-switcher a[lang="pl"]').click();
  await expect(page.locator('html')).toHaveAttribute('lang', 'pl');
  const context = await browser.newContext({ baseURL: new URL(page.url()).origin, javaScriptEnabled: false, viewport: { width: 390, height: 844 } });
  try {
    const fallback = await context.newPage();
    await fallback.goto('/en');
    await fallback.locator('.mobile-nav summary').click();
    await expect(fallback.locator('.mobile-nav-panel')).toBeVisible();
    await fallback.locator('.mobile-nav-panel nav a[href$="/servers"]').click();
    await expect(fallback).toHaveURL(/\/en\/servers$/u);
  } finally { await context.close(); }
});

test('@smoke @portal-review actual authenticated hub, characters, support, security and tools', async ({ page }) => {
  // A read-model binding is not a provisioned game account. Register through Laravel.
  await register(page, email, password);
  await login(page, email, password);
  await page.goto('/account?locale=en');
  await expect(page.locator('[data-account-state]')).toHaveAttribute('data-account-state', 'ready');
  const privatePages = [
    ['/account', 'account'], ['/account/characters/create', 'character-create'],
    ['/account/security', 'security'], ['/mfa', 'mfa'], ['/password/change', 'change-password'],
    ['/account/payments', 'payments'], ['/account/bazaar', 'my-bazaar'], ['/account/bazaar/sell', 'bazaar-create'],
    ['/account/tools/session-analyzer', 'player-tools'], ['/support/tickets', 'support-tickets'],
    ['/support/tickets/create', 'support-ticket-create'], ['/support/reports', 'support-reports'],
    ['/support/reports/create', 'support-report-create'], ['/support/enforcement', 'enforcement'],
    ['/account?locale=pl', 'account-pl'], ['/account/security?locale=pl', 'security-pl'], ['/en', 'home-authenticated'],
  ];
  for (const [name, viewport] of viewports.filter(([name]) => ['desktop', 'phone'].includes(name))) {
    await page.setViewportSize(viewport);
    for (const [path, suffix] of privatePages) await visit(page, path, `${name}-${suffix}`);
    await expect(page.locator('.realm-hero-actions a[href*="/account"]').first()).toBeVisible();
    await expect(page.locator('.realm-hero-actions a[href*="/register"]')).toHaveCount(0);
  }
  await page.goto('/account/characters/create?locale=en');
  const characterName = uniqueCharacterName('Portal');
  await page.locator('input[name="name"]').fill(characterName);
  await page.locator('select[name="vocation"]').selectOption('1');
  await page.locator('select[name="sex"]').selectOption('1');
  await page.locator('form[method="POST"] button[type="submit"]').last().click();
  await page.goto('/account?locale=en');
  await expect(page.locator('.character-roster-row')).toContainText(characterName);
  const profilePath = `/account/characters/${encodeURIComponent(characterName)}/profile?locale=en`;
  for (const [name, viewport] of viewports.filter(([name]) => ['desktop', 'phone'].includes(name))) {
    await page.setViewportSize(viewport);
    await visit(page, '/account?locale=en', `${name}-account-populated`);
    await visit(page, profilePath, `${name}-character-profile-settings`);
  }

  await page.goto('/support/tickets/create?locale=en');
  await page.getByLabel('Category').selectOption('technical');
  await page.getByLabel('Subject').fill('Portal review help request');
  await page.getByLabel('Initial message').fill('A synthetic visual review conversation, not a real support request.');
  await page.getByRole('button', { name: 'Open ticket', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Portal review help request', exact: true })).toBeVisible();
  const ticketPath = new URL(page.url()).pathname;
  await page.goto('/support/reports/create?locale=en');
  await page.getByLabel('Report type').selectOption('player');
  await page.getByLabel('Category').selectOption('cheating');
  await page.getByLabel('Target', { exact: true }).fill('Synthetic Review Target');
  await page.getByLabel('Evidence summary').fill('Isolated fixture only. No real player is being reported.');
  await page.getByRole('button', { name: 'Submit report', exact: true }).click();
  await expect(page.getByText('Synthetic Review Target', { exact: true })).toBeVisible();
  const reportPath = new URL(page.url()).pathname;
  for (const [name, viewport] of viewports.filter(([name]) => ['desktop', 'phone'].includes(name))) {
    await page.setViewportSize(viewport);
    await visit(page, `${ticketPath}?locale=en`, `${name}-support-ticket-detail`);
    await visit(page, `${reportPath}?locale=en`, `${name}-support-report-detail`);
  }

  await page.goto('/account/tools/session-analyzer?locale=en');
  await page.getByLabel('Session label').fill('Portal review duo');
  await page.getByLabel('Session log').fill('Session: 01:00h\nXP Gain: 3,600,000\nLoot: 600,000\nSupplies: 200,000\nBalance: 400,000\nAlice\nLoot: 400,000\nSupplies: 100,000\nBob\nLoot: 200,000\nSupplies: 100,000');
  await page.getByRole('button', { name: 'Analyze and save privately' }).click();
  await expect(page.getByRole('heading', { name: 'Portal review duo', level: 1 })).toBeVisible();
  for (const [name, viewport] of viewports.filter(([name]) => ['desktop', 'phone'].includes(name))) {
    await page.setViewportSize(viewport);
    await capture(page, `${name}-player-tools-detail`, 200);
  }
  for (const state of ['pending', 'recoverable', 'conflict']) {
    runBinary('php', ['scripts/acceptance/seed-account-overview-state.php', 'seed', email, state]);
    await visit(page, '/account?locale=en', `phone-account-${state}`);
    await expect(page.locator('[data-account-state]')).toHaveAttribute('data-account-state', state);
  }
  runBinary('php', ['scripts/acceptance/seed-account-overview-state.php', 'seed', email, 'ready']);
  await logout(page);
  await page.goto('/account');
  await expect(page).toHaveURL(/\/login$/u);
});

test('@smoke @portal-review truthful edge states, phone 320, 200 percent text and reduced motion', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  for (const scenario of ['empty', 'news-outage']) {
    await page.setExtraHTTPHeaders({ 'X-Oteryn-Acceptance-Today-Scenario': scenario });
    await visit(page, '/en/today', `phone-today-${scenario}`);
    if (scenario === 'empty') {
      for (const kind of ['announcements', 'events', 'news']) await expect(page.locator(`[data-today-card="${kind}"]`)).toHaveAttribute('data-content-state', 'empty');
    } else {
      await expect(page.locator('[data-today-card="news"]')).toHaveAttribute('data-content-state', 'unavailable');
    }
  }
  await page.setExtraHTTPHeaders({});
  await visit(page, '/portal-redesign-missing-page', 'phone-not-found', 404);
  await page.goto('/login?locale=en');
  await page.locator('input[name="email"]').fill('unknown-portal-review@example.test');
  await page.locator('input[name="password"]').fill('Invalid-Review-9!Password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await page.locator('input[name="email"]').fill('');
  await page.locator('input[name="password"]').fill('');
  await capture(page, 'phone-login-error', 200);
  await page.setViewportSize({ width: 320, height: 740 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  for (const [path, name] of [['/pl', 'home-pl'], ['/login?locale=pl', 'login-pl'], ['/en/highscores', 'highscores'], ['/en/download', 'download']]) await visit(page, path, `narrow-${name}`);
  await page.setViewportSize({ width: 820, height: 1180 });
  await page.goto('/pl');
  await page.evaluate(() => { document.documentElement.style.fontSize = '200%'; });
  await capture(page, 'tablet-text-200-percent', 200);
});


test('@smoke @portal-review MFA challenge belongs to the identity system', async ({ page }) => {
  const mfaEmail = uniqueEmail('portal.mfa.review');
  runBinary('php', ['scripts/acceptance/seed-browser-events.php', 'seed-identity', mfaEmail, password, 'PORTAL-REVIEW-MFA-01', 'confirmed', '']);
  await page.goto('/login?locale=en');
  await login(page, mfaEmail, password);
  await expect(page).toHaveURL(/\/mfa\/challenge$/u);
  for (const [name, viewport] of viewports.filter(([name]) => ['desktop', 'phone'].includes(name))) {
    await page.setViewportSize(viewport);
    await capture(page, `${name}-mfa-challenge`, 200);
    await assertAccessibilitySmoke(page);
  }
});


test('@smoke @portal-review administration and every registered administrator navigation family', async ({ page }) => {
  const adminEmail = uniqueEmail('portal.admin.review');
  const recoveryCode = 'PORTAL-ADMIN-REVIEW-01';
  runBinary('php', ['scripts/acceptance/seed-browser-admin.php', adminEmail, password, recoveryCode]);
  await login(page, adminEmail, password);
  await completeMfaChallenge(page, recoveryCode);
  await page.goto('/admin');
  const destinations = await page.locator('.admin-sidebar .admin-nav a').evaluateAll((links) =>
    [...new Set(links.map((link) => new URL(link.href).pathname))]);
  expect(destinations.length).toBeGreaterThanOrEqual(16);
  for (const [name, viewport] of viewports.filter(([name]) => ['desktop', 'phone'].includes(name))) {
    await page.setViewportSize(viewport);
    for (const path of destinations) {
      await visit(page, path, `${name}-admin-${path.replace(/^\/admin\/?/u, '').replaceAll('/', '-') || 'dashboard'}`);
    }
    for (const path of ['/admin/news/create', '/admin/pages/create', '/admin/events/create', '/admin/announcements/create', '/admin/downloads/create']) {
      await visit(page, path, `${name}-${path.slice(1).replaceAll('/', '-')}`);
    }
  }
  await page.goto('/admin?locale=pl');
  await capture(page, 'phone-admin-localized-shell', 200);
  const menu = page.locator('.admin-mobile-nav');
  await menu.locator('summary').click();
  await expect(menu.locator('nav')).toBeVisible();
  await capture(page, 'phone-admin-menu', 200);
  await assertAccessibilitySmoke(page);
});
