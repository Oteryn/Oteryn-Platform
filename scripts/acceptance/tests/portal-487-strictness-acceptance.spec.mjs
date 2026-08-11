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

const adminPassword = 'AcceptancePortal487Admin!234';
const supportPassword = 'AcceptancePortal487Support!234';
const adminMarker = '@portal-487-strictness admin-cms not-found csrf-419 server-failure recovery';
const publicMarker = '@portal-487-strictness public not-found accessibility overflow server-failure recovery';
const supportMarker = '@portal-487-strictness support-moderation not-found csrf-419 rate-429 server-failure recovery';

function portalFixture(...args) {
  return JSON.parse(runBinary('php', ['scripts/acceptance/seed-browser-portal-487-strictness.php', ...args]));
}

function supportFixture(...args) {
  return JSON.parse(runBinary('php', ['scripts/acceptance/seed-browser-support-moderation.php', ...args]));
}

function restoreAllAvailability() {
  portalFixture('restore-all');
}

function seedAdmin() {
  const email = uniqueEmail('portal-487-admin');
  const recoveryCode = 'P487-ADMIN-RECOVERY-01';
  JSON.parse(runBinary('php', ['scripts/acceptance/seed-browser-admin.php', email, adminPassword, recoveryCode]));
  portalFixture('grant-admin-permissions');

  return { email, password: adminPassword, recoveryCode };
}

function seedSupportOwner() {
  const label = `portal-487-support-${Math.random().toString(16).slice(2, 10)}`;
  const email = uniqueEmail(label);
  const recoveryCodes = ['P487-SUPPORT-RECOVERY-01', 'P487-SUPPORT-RECOVERY-02'];
  const result = supportFixture(
    'seed-identity',
    email,
    supportPassword,
    recoveryCodes.join(','),
    'unconfirmed',
    '',
  );

  return { ...result, email, password: supportPassword };
}

async function signInAdmin(page, admin) {
  await login(page, admin.email, admin.password);
  await completeMfaChallenge(page, admin.recoveryCode);
}

async function assertNoOverflow(page) {
  const dimensions = await page.evaluate(() => ({
    viewport: window.innerWidth,
    document: document.documentElement.scrollWidth,
  }));
  expect(dimensions.document, `Unexpected horizontal overflow on ${page.url()}`).toBeLessThanOrEqual(dimensions.viewport + 1);
}

async function csrfStatus(page, path, method = 'POST') {
  const responsePromise = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return response.request().method() === 'POST' && url.pathname === path;
  });

  await page.evaluate(({ target, verb }) => {
    const form = document.createElement('form');
    form.method = 'POST';
    form.action = target;

    const token = document.createElement('input');
    token.type = 'hidden';
    token.name = '_token';
    token.value = 'acceptance-explicitly-invalid-csrf-token';
    form.append(token);

    if (verb !== 'POST') {
      const override = document.createElement('input');
      override.type = 'hidden';
      override.name = '_method';
      override.value = verb;
      form.append(override);
    }

    document.body.append(form);
    form.submit();
  }, { target: path, verb: method });

  const response = await responsePromise;
  await page.waitForLoadState('domcontentloaded');
  return response.status();
}

async function expectServerFailureRecovery(page, surface, path) {
  let response = await page.goto(path);
  expect(response?.status(), `Expected healthy precondition for ${surface}`).toBe(200);
  await assertNoOverflow(page);

  try {
    portalFixture('make-unavailable', surface);
    response = await page.goto(path, { waitUntil: 'domcontentloaded' });
    expect(response?.status(), `Expected injected server failure for ${surface}`).toBe(500);
  } finally {
    portalFixture('restore', surface);
  }

  page.__acceptanceDiagnostics.serverErrors = [];
  response = await page.goto(path);
  expect(response?.status(), `Expected recovered surface for ${surface}`).toBe(200);
  await assertNoOverflow(page);
}

test.setTimeout(420_000);
test.describe.configure({ retries: 0, mode: 'serial' });

test.beforeEach(async ({ page }) => {
  restoreAllAvailability();
  supportFixture('reset');
  runArtisan('cache:clear');
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  try {
    restoreAllAvailability();
    await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
  } finally {
    supportFixture('reset');
    runArtisan('cache:clear');
  }
});

// Evidence marker: @portal-487-strictness admin-cms not-found csrf-419 server-failure recovery
test(adminMarker, async ({ page }) => {
  const admin = seedAdmin();
  await signInAdmin(page, admin);

  const notFoundPaths = [
    '/admin/news/999999999/edit',
    '/admin/portal/homepage/preview/not-a-real-template',
    '/admin/announcements/999999999/edit',
    '/admin/downloads/999999999/edit',
    '/admin/events/999999999/edit',
    '/admin/support-content/not-a-real-key/edit',
    '/admin/support/tickets/999999999',
  ];

  for (const path of notFoundPaths) {
    const response = await page.goto(path, { waitUntil: 'domcontentloaded' });
    expect(response?.status(), `Expected exact-surface 404 at ${path}`).toBe(404);
  }

  const csrfProbes = [
    ['/admin/news', 'POST'],
    ['/admin/portal/homepage/active', 'PUT'],
    ['/admin/announcements', 'POST'],
    ['/admin/downloads', 'POST'],
    ['/admin/events', 'POST'],
    ['/admin/support-content/support', 'PUT'],
    ['/admin/support/tickets/999999999/reply', 'POST'],
  ];

  for (const [path, method] of csrfProbes) {
    expect(await csrfStatus(page, path, method), `Expected CSRF expiry response for ${method} ${path}`).toBe(419);
  }

  const failureSurfaces = [
    ['admin.core-rbac-cms-audit', '/admin/news'],
    ['admin.homepage-template-selector', '/admin/portal/homepage'],
    ['announcements.admin-localization-home-composition', '/admin/announcements'],
    ['downloads.public-admin-localization', '/admin/downloads'],
    ['events.public-admin', '/admin/events'],
    ['support-legal.public-admin-localization', '/admin/support-content'],
    ['support.moderation-lifecycle', '/admin/support/tickets'],
  ];

  for (const [surface, path] of failureSurfaces) {
    await expectServerFailureRecovery(page, surface, path);
  }
});

// Evidence marker: @portal-487-strictness public not-found accessibility overflow server-failure recovery
test(publicMarker, async ({ page }) => {
  const failureSurfaces = [
    ['public.home-and-seo', '/'],
    ['public.localization-core', '/en'],
    ['public.news-and-managed-pages', '/news'],
  ];

  for (const [surface, path] of failureSurfaces) {
    await expectServerFailureRecovery(page, surface, path);
  }

  const layoutPaths = [
    '/download',
    '/events',
    '/deaths',
    '/highscores',
    '/',
    '/en',
    '/news',
    '/support',
  ];

  for (const path of layoutPaths) {
    const response = await page.goto(path);
    expect(response?.status(), `Expected renderable strictness surface at ${path}`).toBe(200);
    await assertNoOverflow(page);
  }

  const accessibilityPaths = ['/download', '/events', '/highscores', '/en', '/news', '/support'];
  for (const path of accessibilityPaths) {
    await page.goto(path);
    await assertAccessibilitySmoke(page);
  }

  let response = await page.goto('/not-a-real-portal-487-route', { waitUntil: 'domcontentloaded' });
  expect(response?.status()).toBe(404);
  response = await page.goto('/news/not-a-real-portal-487-article', { waitUntil: 'domcontentloaded' });
  expect(response?.status()).toBe(404);
  response = await page.goto('/characters/not-a-real-portal-487-character', { waitUntil: 'domcontentloaded' });
  expect(response?.status()).toBe(404);
  response = await page.goto('/guilds/not-a-real-portal-487-guild', { waitUntil: 'domcontentloaded' });
  expect(response?.status()).toBe(404);
});

// Evidence marker: @portal-487-strictness support-moderation not-found csrf-419 rate-429 server-failure recovery
test(supportMarker, async ({ page }) => {
  const owner = seedSupportOwner();
  await login(page, owner.email, owner.password);
  await page.waitForURL((url) => url.pathname !== '/login');

  let response = await page.goto('/support/tickets/999999999', { waitUntil: 'domcontentloaded' });
  expect(response?.status()).toBe(404);
  expect(await csrfStatus(page, '/support/tickets', 'POST')).toBe(419);

  await page.goto('/support/tickets/create');
  const csrfToken = await page.locator('input[name="_token"]').inputValue();
  expect(csrfToken).toBeTruthy();

  const statuses = [];
  for (let attempt = 1; attempt <= 7; attempt += 1) {
    const status = await page.evaluate(async ({ token, sequence }) => {
      const form = new URLSearchParams({
        _token: token,
        category: 'technical',
        subject: `Portal 487 rate limit ${sequence}`,
        message: `Bounded support rate-limit evidence ${sequence}`,
      });
      const result = await fetch('/support/tickets', {
        method: 'POST',
        credentials: 'same-origin',
        headers: {
          Accept: 'text/html,application/xhtml+xml',
          'Content-Type': 'application/x-www-form-urlencoded;charset=UTF-8',
        },
        body: form.toString(),
      });
      return result.status;
    }, { token: csrfToken, sequence: attempt });
    statuses.push(status);
  }

  expect(statuses.slice(0, 6).every((status) => status !== 419 && status !== 429 && status < 500)).toBe(true);
  expect(statuses[6]).toBe(429);

  try {
    portalFixture('make-unavailable', 'support.moderation-lifecycle');
    response = await page.goto('/support/tickets', { waitUntil: 'domcontentloaded' });
    expect(response?.status()).toBe(500);
  } finally {
    portalFixture('restore', 'support.moderation-lifecycle');
  }

  page.__acceptanceDiagnostics.serverErrors = [];
  response = await page.goto('/support/tickets');
  expect(response?.status()).toBe(200);
  await assertNoOverflow(page);
});
