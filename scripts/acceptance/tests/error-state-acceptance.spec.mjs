import fs from 'node:fs';
import path from 'node:path';
import { test, expect } from '@playwright/test';
import {
  allowExpectedHttpFailure,
  attachDiagnostics,
  installDiagnostics,
  repoRoot,
  runArtisan,
  runBinary,
} from './helpers.mjs';

const evidenceMarker = '@portal-global-errors real localized 404 419 429 and 500 lifecycle';
const appBaseURL = process.env.ACCEPTANCE_BASE_URL ?? 'http://127.0.0.1:8080';
const evidencePath = path.join(repoRoot, 'scripts/acceptance/coverage/error-state-evidence.json');
const evidence = JSON.parse(fs.readFileSync(evidencePath, 'utf8'));
const expectedStatuses = [404, 419, 429, 500];
const expectedProjects = [
  'error-states-chromium-desktop',
  'error-states-chromium-tablet',
  'error-states-chromium-mobile',
];

if (evidence.schema_version !== 1 || evidence.issue !== 353 || evidence.parent_issue !== 326) {
  throw new Error('Global error evidence identity does not match Issue #353 and parent #326.');
}
if (JSON.stringify(evidence.statuses?.map((entry) => entry.code)) !== JSON.stringify(expectedStatuses)) {
  throw new Error('Global error evidence must define the exact 404, 419, 429 and 500 status order.');
}
if (evidence.statuses?.find((entry) => entry.code === 419)?.trigger !== 'cross-site-browser-form-with-invalid-csrf-token') {
  throw new Error('Global error evidence must bind 419 to the real cross-site browser-form trigger.');
}
if (JSON.stringify(evidence.locales) !== JSON.stringify(['en', 'pl'])) {
  throw new Error('Global error evidence must cover exact English and Polish locales.');
}
if (JSON.stringify(evidence.projects) !== JSON.stringify(expectedProjects) || evidence.retries !== 0) {
  throw new Error('Global error evidence projects or zero-retry policy drifted.');
}
if (evidence.evidence?.file !== 'scripts/acceptance/tests/error-state-acceptance.spec.mjs'
  || evidence.evidence?.marker !== evidenceMarker) {
  throw new Error('Global error evidence does not point to the executable marker.');
}
if (!fs.readFileSync(path.join(repoRoot, evidence.evidence.file), 'utf8').includes(evidenceMarker)) {
  throw new Error('Global error executable evidence marker is missing.');
}

const headings = {
  en: {
    404: 'We could not find that page',
    419: 'The security token expired',
    429: 'Slow down and try again',
    500: 'Oteryn could not complete this request',
  },
  pl: {
    404: 'Nie udało się znaleźć tej strony',
    419: 'Token bezpieczeństwa wygasł',
    429: 'Zwolnij i spróbuj ponownie',
    500: 'Oteryn nie mógł wykonać tego żądania',
  },
};

test.setTimeout(240_000);
test.describe.configure({ retries: 0, mode: 'serial' });

test.beforeEach(async ({ page }, testInfo) => {
  test.skip(!evidence.projects.includes(testInfo.project.name), 'Dedicated Error State Acceptance projects only.');
  runBinary('php', ['scripts/acceptance/seed-community-data.php']);
  runArtisan('cache:clear');
  runArtisan('view:clear');
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  if (!evidence.projects.includes(testInfo.project.name)) return;
  runArtisan('cache:clear');
  runArtisan('view:clear');
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

async function expectNoHorizontalOverflow(page) {
  const dimensions = await page.evaluate(() => ({
    documentWidth: document.documentElement.scrollWidth,
    viewportWidth: document.documentElement.clientWidth,
  }));
  expect(dimensions.documentWidth).toBeLessThanOrEqual(dimensions.viewportWidth + 1);
}

async function expectKeyboardReachableRecoveryAction(page) {
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
  });
  await page.keyboard.press('Tab');
  await page.keyboard.press('Tab');
  const focused = page.locator(':focus');
  await expect(focused).toHaveCount(1);
  expect(await focused.evaluate((element) => element.tagName)).toBe('A');
  expect(await focused.getAttribute('href')).toBeTruthy();
}

async function expectErrorSurface(page, response, status, locale) {
  expect(response?.status()).toBe(status);
  expect(response?.headers()['content-language']).toBe(locale);
  await expect(page.locator('html')).toHaveAttribute('lang', locale);
  await expect(page.locator('meta[name="robots"]')).toHaveAttribute('content', 'noindex, nofollow');
  await expect(page.locator('.error-code')).toHaveText(String(status));
  await expect(page.getByRole('heading', { name: headings[locale][status], exact: true })).toBeVisible();
  await expect(page.locator('.action-row a').first()).toBeVisible();
  await expectNoHorizontalOverflow(page);
  await expectKeyboardReachableRecoveryAction(page);
}

async function prove404(page, locale, projectSlug) {
  const response = await page.goto(`/${locale}/acceptance-missing-${projectSlug}`);
  await expectErrorSurface(page, response, 404, locale);
}

async function prove419(page, locale) {
  const target = new URL(`/register?locale=${encodeURIComponent(locale)}`, appBaseURL).toString();
  await page.goto('data:text/html,<title>Cross-site CSRF acceptance probe</title>');

  const responsePromise = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return response.request().method() === 'POST'
      && url.origin === new URL(appBaseURL).origin
      && url.pathname === '/register';
  });

  await page.evaluate((formTarget) => {
    const form = document.createElement('form');
    form.method = 'POST';
    form.action = formTarget;

    for (const [name, value] of [
      ['_token', 'acceptance-explicitly-invalid-csrf-token'],
      ['email', 'csrf-error-probe@example.test'],
    ]) {
      const input = document.createElement('input');
      input.name = name;
      input.value = value;
      form.append(input);
    }

    document.body.append(form);
    form.submit();
  }, target);

  const response = await responsePromise;
  await page.waitForLoadState('domcontentloaded');
  await expectErrorSurface(page, response, 419, locale);
  await expect(page.locator('body')).not.toContainText('csrf-error-probe@example.test');
  await expect(page.locator('body')).not.toContainText('acceptance-explicitly-invalid-csrf-token');
}

async function prove429(page, locale, projectSlug) {
  runArtisan('cache:clear');
  const email = `error-${projectSlug}-${locale}@example.test`;
  let limitedResponse = null;

  try {
    await page.goto(`/login?locale=${locale}`);

    for (let attempt = 1; attempt <= 6; attempt += 1) {
      await page.locator('#email').fill(email);
      await page.locator('#password').fill('AcceptanceWrongPassword!234');

      const [postResponse] = await Promise.all([
        page.waitForResponse((response) => {
          const url = new URL(response.url());
          return response.request().method() === 'POST' && url.pathname === '/login';
        }),
        page.waitForNavigation({ waitUntil: 'domcontentloaded' }),
        page.locator('button[type="submit"]').click(),
      ]);

      if (attempt < 6) {
        expect([302, 303]).toContain(postResponse.status());
        await expect(page.locator('#email')).toBeVisible();
      } else {
        limitedResponse = postResponse;
      }
    }

    await expectErrorSurface(page, limitedResponse, 429, locale);
    const retryAfter = Number.parseInt(limitedResponse.headers()['retry-after'] ?? '', 10);
    expect(Number.isInteger(retryAfter)).toBe(true);
    expect(retryAfter).toBeGreaterThan(0);
    expect(retryAfter).toBeLessThanOrEqual(60);
    await expect(page.locator('body')).not.toContainText(email);
  } finally {
    runArtisan('cache:clear');
  }
}

async function prove500(page, locale) {
  const highscoreView = path.join(repoRoot, 'resources/views/game/highscores.blade.php');
  const unavailableView = `${highscoreView}.error-state-unavailable`;
  let viewMoved = false;

  if (fs.existsSync(unavailableView)) fs.rmSync(unavailableView, { force: true });

  try {
    const baseline = await page.goto(`/${locale}/highscores`);
    expect(baseline?.status()).toBe(200);

    fs.renameSync(highscoreView, unavailableView);
    viewMoved = true;
    runArtisan('view:clear');

    const response = await page.goto(`/${locale}/highscores`, { waitUntil: 'domcontentloaded' });
    await expectErrorSurface(page, response, 500, locale);
    allowExpectedHttpFailure(page.__acceptanceDiagnostics, { status: 500, pathname: `/${locale}/highscores` });

    const body = await page.locator('body').innerText();
    for (const forbidden of [
      'InvalidArgumentException',
      'View [game.highscores] not found',
      highscoreView,
      process.env.DB_DATABASE,
      process.env.CANARY_DB_DATABASE,
      process.env.DB_PASSWORD,
      process.env.MARIADB_ROOT_PASSWORD,
      'SQLSTATE',
      'Stack trace',
    ].filter((value) => typeof value === 'string' && value !== '')) {
      expect(body).not.toContain(forbidden);
    }
  } finally {
    if (viewMoved && fs.existsSync(unavailableView) && !fs.existsSync(highscoreView)) {
      fs.renameSync(unavailableView, highscoreView);
    } else if (fs.existsSync(unavailableView) && fs.existsSync(highscoreView)) {
      fs.rmSync(unavailableView, { force: true });
    }
    runArtisan('view:clear');
  }

  const recovered = await page.goto(`/${locale}/highscores`);
  expect(recovered?.status()).toBe(200);
  await expect(page.locator('h1')).toBeVisible();
}

// Evidence marker: @portal-global-errors real localized 404 419 429 and 500 lifecycle
test('@portal-global-errors real localized 404, 419, 429 and 500 lifecycle', async ({ page }, testInfo) => {
  expect(evidence.projects).toContain(testInfo.project.name);
  expect(process.env.APP_DEBUG).toBe('false');

  const projectSlug = testInfo.project.name.replaceAll(/[^a-z0-9]+/gu, '-');
  for (const locale of evidence.locales) {
    await prove404(page, locale, projectSlug);
    await prove419(page, locale);
    await prove429(page, locale, projectSlug);
    await prove500(page, locale);
  }
});
