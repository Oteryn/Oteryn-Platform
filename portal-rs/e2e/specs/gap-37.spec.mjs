import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {test, expect} from '../fixtures.mjs';
import {submit} from '../helpers.mjs';
import {run as withPortal} from '../api/harness.mjs';
import {query, seededAccount, seededLogin, seededStaff} from '../staff-rbac-helpers.mjs';

const shell = '#portal-header, #podglad-konta, footer.foot';
const unavailableReads = new Set([
  'online', 'rankings/experience', 'guilds', 'houses', 'deaths', 'wiki', 'wiki/creatures',
  'atlas', 'atlas/delivery', 'stats/guilds', 'stats/rankings/yesterday',
  'account/journal/reports', 'account/game-logins', 'pass', 'atlas/places',
]);

async function crawlShell(page, lang) {
  const targets = await page.locator(shell).locator('a[href]').evaluateAll(anchors => anchors.map(link => ({
    url: link.href, label: link.textContent.trim() || link.getAttribute('aria-label'),
  })));
  const current = new URL(page.url());
  const visited = new Set();
  for (const {url, label} of targets) {
    const target = new URL(url);
    if (target.origin !== current.origin || visited.has(url)) continue;
    visited.add(url);
    expect(label, url).toBeTruthy();
    if (target.pathname === current.pathname && target.search === current.search && target.hash) {
      expect(await page.locator(`[id="${target.hash.slice(1)}"]`).count(), url).toBe(1);
      continue;
    }
    const response = await page.request.get(url, {headers: {accept: 'text/html'}});
    const section = target.pathname.replace(`/${lang}/`, '');
    expect(new URL(response.url()).origin, url).toBe(current.origin);
    expect(response.status(), url).toBe(unavailableReads.has(section) ? 503 : 200);
    if (target.hash) {
      expect(await response.text(), url).toContain(`id="${target.hash.slice(1)}"`);
    }
    if (response.status() === 503) {
      expect(response.headers()['cache-control'], url).toContain('no-store');
      expect(await response.text(), url).toContain('id="portal-header"');
    }
  }
}

async function openNav(page, info) {
  if (info.project.use.viewport.width === 390) {
    await page.locator('.om-fallback-menu > summary').click();
  }
}

test('guest, player and staff can follow the localized portal navigation', async ({page, browser}, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const route = suffix => `/${lang}/${suffix}`;
  // The local league producer publishes its initial snapshot after the listener starts.
  await expect.poll(async () => (await page.request.get(route('league'))).status(), {
    timeout: 30_000,
  }).toBe(200);
  expect((await page.goto(route('news'))).status()).toBe(200);
  await openNav(page, info);
  await page.locator(`#portal-navigation a[href="${route('start')}"]`).click();
  await expect(page).toHaveURL(route('start'));
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.locator('#jak-zaczac')).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await expect(page.locator(`a[href="${route('account/privacy/export')}"]`)).toHaveCount(0);
  await expect(page.locator(`a[href="${route('atlas/places')}"]`)).toHaveCount(0);
  await expect(page.locator(`a[href="${route('pass')}"]`)).toHaveCount(0);
  await expect(page.locator(`#portal-navigation a[href="${route('bazaar')}"]`)).toHaveCount(1);
  await page.locator(`#jak-zaczac a[href="${route('download')}"]`).click();
  await expect(page).toHaveURL(route('download'));
  await expect(page.locator('main h1')).toHaveText(lang === 'pl' ? 'Pobierz klienta' : 'Client downloads');
  await expect(page.locator('main a[href^="https://downloads.oteryn.com"]')).toHaveCount(0);
  await crawlShell(page, lang);
  for (const [legacy, canonical] of [['/getting-started', '/pl/start'], ['/download', '/pl/download']]) {
    const response = await page.request.get(legacy);
    expect(response.status(), legacy).toBe(200);
    expect(new URL(response.url()).pathname).toBe(canonical);
  }
  expect((await page.request.get(route('download/unknown-channel'))).status()).toBe(404);
  expect((await page.request.get(route('account/privacy/export'), {maxRedirects: 0})).headers().location).toBe(route('login'));

  const player = await seededAccount(lang);
  await seededLogin(page, lang, player);
  expect((await page.request.get(route('account/privacy/export'), {maxRedirects: 0})).status()).toBe(403);
  await page.goto(route('start'));
  await crawlShell(page, lang);
  await openNav(page, info);
  const menu = page.locator('.om-fallback-account');
  await menu.locator('summary').click();
  for (const suffix of ['mfa', 'account/security/email', 'account/consents', 'account/privacy/export', 'account/badges', 'account/bazaar']) {
    await expect(menu.locator(`a[href="${route(suffix)}"]`)).toBeVisible();
  }
  await menu.locator(`a[href="${route('mfa')}"]`).click();
  await expect(page).toHaveURL(route('mfa'));
  await expect(page.locator('main form[action$="/mfa/enroll"]')).toBeVisible();
  await page.goto(route('start'));
  await openNav(page, info);
  await menu.locator('summary').click();
  await menu.locator(`a[href="${route('account/privacy/export')}"]`).click();
  await expect(page).toHaveURL(`${route('account/sudo')}?continue=account-export`);
  await submit(page, 'main form', {current_password: player.password});
  await expect(page).toHaveURL(route('account/privacy/export'));
  const exportReady = page.waitForEvent('download');
  await page.locator('main form button').click();
  const exported = await exportReady;
  expect(exported.suggestedFilename()).toBe('account-data.json');
  expect(await exported.failure()).toBeNull();
  expect((await page.request.get('/admin/roles', {maxRedirects: 0})).status()).toBe(403);

  const staff = await browser.newContext({
    baseURL: new URL(page.url()).origin,
    ignoreHTTPSErrors: true,
    locale: lang,
    viewport: info.project.use.viewport,
  });
  try {
    const screen = await staff.newPage();
    await seededStaff(screen, lang, 'platform_admin');
    expect((await screen.goto(`/admin?lang=${lang}`)).status()).toBe(200);
    const roles = screen.locator('main a[href="/admin/roles"]').first();
    await expect(roles).toBeVisible();
    await roles.click();
    await expect(screen).toHaveURL(/\/admin\/roles/);
    await screen.goto(route('start'));
    await crawlShell(screen, lang);
  } finally {
    await staff.close();
  }
});

test('flag-disabled destinations stay out of guest and player navigation', async ({page}, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  await isolatedPortal(info, {
    OTERYN_BAZAAR_ENABLED: 'false',
    OTERYN_DOWNLOADS_ENABLED: 'false',
    OTERYN_ACCOUNT_OVERVIEW_ENABLED: 'false',
    OTERYN_ACCOUNT_MFA: 'false',
    OTERYN_ACCOUNT_EMAIL_CHANGE: 'false',
    OTERYN_ACCOUNT_EXPORT_ENABLED: 'false',
  }, async portal => {
    const route = suffix => `${portal.origin}/${lang}/${suffix}`;
    expect((await page.goto(route('start'))).status()).toBe(200);
    for (const suffix of ['bazaar', 'download']) {
      await expect(page.locator(`a[href="/${lang}/${suffix}"]`)).toHaveCount(0);
    }
    expect((await page.request.get(route('download'))).status()).toBe(404);
    await loginOnPortal(page, portal, lang);
    expect((await page.goto(route('start'))).status()).toBe(200);
    await openNav(page, info);
    const menu = page.locator('.om-fallback-account');
    await menu.locator('summary').click();
    for (const path of ['/account', ...['account/bazaar', 'mfa', 'account/security/email', 'account/privacy/export'].map(suffix => `/${lang}/${suffix}`)]) {
      await expect(menu.locator(`a[href="${path}"]`)).toHaveCount(0);
    }
  });
});

async function isolatedPortal(info, portalOptions, exercise) {
  const port = 18900 + info.parallelIndex * 10;
  await withPortal({
    harnessEnv: {
      PORTAL_E2E_HTTP_PORT: String(port),
      PORTAL_E2E_INTERNAL_PORT: String(port + 1),
      PORTAL_E2E_ADMISSION_PORT: String(port + 2),
      PORTAL_E2E_GATEWAY_PORT: String(port + 3),
      PORTAL_API_NATIVE_PORT: String(port + 4),
      PORTAL_API_SMTP_PORT: String(port + 5),
      PORTAL_E2E_KEEP_DATABASE: '0',
    },
    portalOptions,
  }, exercise);
}

async function loginOnPortal(page, portal, lang) {
  // The isolated harness supplies its own account and origin.
  const target = `${portal.origin}/${lang}/login`;
  expect((await page.goto(target)).status()).toBe(200);
  const form = page.locator('main form');
  await form.locator('[name="email"]').fill(portal.account.email);
  await form.locator('[name="password"]').fill(portal.account.password);
  await form.locator('button[type="submit"]').click();
  await expect(page).toHaveURL(`${portal.origin}/${lang}/`);
}

test('i18n-parity preserves the destination and localizes the download entry', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const other = lang === 'pl' ? 'en' : 'pl';
  const labels = {en: 'Client downloads', pl: 'Pobierz klienta'};
  expect((await page.goto(`/${lang}/download`)).status()).toBe(200);
  await expect(page.locator('main h1')).toHaveText(labels[lang]);
  await page.locator(`footer .foot-langs a[hreflang="${other}"]`).click();
  await expect(page).toHaveURL(`/${other}/download`);
  await expect(page.locator('html')).toHaveAttribute('lang', other);
  await expect(page.locator('main h1')).toHaveText(labels[other]);
});


test('the guide opens a signed stable client release', async ({page}, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  await isolatedPortal(info, {OTERYN_DOWNLOADS_ENABLED: 'true'}, async portal => {
    const [actor] = await query(
      "INSERT INTO account_roles(account_id, role_id) SELECT a.id, r.id FROM accounts a CROSS JOIN roles r WHERE a.email=$1 AND r.key='platform_admin' RETURNING account_id",
      [portal.account.email], portal.databaseURL,
    );
    const bundle = fileURLToPath(new URL('../../crates/downloads/tests/fixtures/origin-v1/stable', import.meta.url));
    const published = spawnSync(
      process.env.PORTAL_E2E_BINARY,
      ['downloads-publish', '--channel', 'stable', '--actor', actor.account_id, '--dir', bundle],
      {env: {...process.env, OTERYN_DATABASE_URL: portal.databaseURL,
        OTERYN_DOWNLOADS_ENABLED: 'true', OTERYN_DOWNLOADS_ROOT_STABLE: `${bundle}/metadata/1.root.json`}},
    );
    expect(published.status, 'the signed stable bundle was published').toBe(0);
    const start = `${portal.origin}/${lang}/start`;
    expect((await page.goto(start)).status()).toBe(200);
    await page.locator(`#jak-zaczac a[href="/${lang}/download"]`).click();
    await page.locator(`main a[href="/${lang}/download/stable"]`).click();
    await expect(page).toHaveURL(`${portal.origin}/${lang}/download/stable`);
    await expect(page.locator('main').getByRole('link', {name: 'Windows (x86_64)'})).toHaveAttribute(
      'href',
      'https://downloads.oteryn.com/updates/stable/targets/channels/stable/releases/rel-a/windows/x86_64/bc4a71180870f7945155fbb02f4b0a2e3faa2a62d6d31b7039013055ed19869a.client.zip',
    );
    const legacy = await page.request.get(`${portal.origin}/download/stable`);
    expect(legacy.status()).toBe(200);
    expect(new URL(legacy.url()).pathname).toBe('/pl/download/stable');
    expect((await page.request.get(`${portal.origin}/${lang}/download/beta`)).status()).toBe(404);
  });
});
