import {createHash, randomUUID} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {test, expect} from '../fixtures.mjs';
import {pool} from '../helpers.mjs';

const fixture = channel => fileURLToPath(new URL(
  `../../crates/downloads/tests/fixtures/origin-v1/${channel}/`, import.meta.url));
const binary = () => process.env.PORTAL_E2E_BINARY
  || path.resolve(process.env.CARGO_TARGET_DIR || '../target', 'debug/oteryn-portal');

async function publishFixtures() {
  const database = pool();
  const connection = await database.connect();
  const temporary = mkdtempSync(path.join(tmpdir(), 'portal-release-'));
  try {
    await connection.query('SELECT pg_advisory_lock($1)', [37492235]);
    const {rows} = await connection.query(
      "SELECT release_id FROM client_releases JOIN client_channels ON client_channels.active_publication_id = client_releases.id WHERE name = 'stable'");
    if (rows.length) {
      expect(rows).toEqual([{release_id: 'rel-a'}]);
      return;
    }

    const actor = await connection.query(
      "INSERT INTO accounts(id,email,password_hash,status,locale) VALUES (uuidv7(),$1,'unused','active','en') RETURNING id",
      [`release-publisher-${randomUUID()}@example.invalid`]);
    const actorId = actor.rows[0].id;
    const membership = await connection.query(
      "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'", [actorId]);
    expect(membership.rowCount).toBe(1);

    const publish = (channel, dir) => spawnSync(binary(),
      ['downloads-publish', '--channel', channel, '--actor', actorId, '--dir', dir],
      {encoding: 'utf8', env: {...process.env,
        OTERYN_DATABASE_URL: database.options.connectionString,
        OTERYN_DOWNLOADS_ENABLED: 'true',
        [`OTERYN_DOWNLOADS_ROOT_${channel.toUpperCase()}`]: path.join(fixture(channel), 'metadata/1.root.json'),
      }});

    const beta = path.join(temporary, 'beta');
    cpSync(fixture('beta'), beta, {recursive: true});
    const policyFile = path.join(beta, 'policy-v1.json');
    const policy = JSON.parse(readFileSync(policyFile, 'utf8'));
    policy.current_version_display = 'Unverified fixture';
    writeFileSync(policyFile, JSON.stringify(policy));
    const rejected = publish('beta', beta);
    expect(rejected.status, 'beta candidate was refused').toBe(1);
    const stable = publish('stable', fixture('stable'));
    expect(stable.status, 'signed stable release was published').toBe(0);
  } finally {
    await connection.query('SELECT pg_advisory_unlock($1)', [37492235]);
    connection.release();
    await database.end();
    rmSync(temporary, {recursive: true, force: true});
  }
}

const copy = {
  en: {home: 'Get the client', guide: 'Client setup guide', update: 'Update or repair the client',
    recovery: 'Account recovery and support', unavailable: 'No verified release is available for this channel.',
    package: 'Download Windows (x86_64)'},
  pl: {home: 'Pobierz klienta', guide: 'Jak zainstalować klienta', update: 'Aktualizacja i naprawa klienta',
    recovery: 'Odzyskiwanie konta i pomoc', unavailable: 'W tym kanale nie ma zweryfikowanego wydania.',
    package: 'Pobierz Windows (x86_64)'},
};

test('a new player finds the signed client and follows setup and recovery', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const label = copy[lang];

  expect((await page.goto(`/${lang}/`)).status()).toBe(200);
  const entry = page.locator('#jak-zaczac').getByRole('link', {name: label.home, exact: true});
  await expect(entry).toHaveAttribute('href', `/${lang}/download`);
  await publishFixtures();
  await entry.click();
  await expect(page).toHaveURL(new RegExp(`/${lang}/download$`));
  const main = page.getByRole('main');
  await expect(main.getByRole('heading', {level: 1})).toHaveText(label.home);
  await expect(main).toContainText('1.0.0');

  const bytes = Buffer.from('package');
  const digest = createHash('sha256').update(bytes).digest('hex');
  await expect(main).toContainText(digest);
  const artifact = main.getByRole('link', {name: label.package});
  const artifactPath = `/${lang}/download/stable/rel-a/windows/x86_64`;
  await expect(artifact).toHaveAttribute('href', artifactPath);
  const expectedPackage = `https://downloads.oteryn.com/updates/stable/targets/channels/stable/releases/rel-a/windows/x86_64/${digest}.client.zip`;

  // Follow the real portal request up to its package-origin handoff, then serve the
  // bytes from the signed fixture in place of the distribution origin.
  await page.route(`**${artifactPath}`, async route => {
    const redirect = await route.fetch({maxRedirects: 0});
    expect(redirect.status()).toBe(307);
    expect(redirect.headers().location).toBe(expectedPackage);
    expect(redirect.headers()['cache-control']).toBe('no-store');
    await route.fulfill({status: 200, body: bytes, headers: {
      'content-type': 'application/octet-stream', 'content-disposition': 'attachment; filename="client.zip"',
    }});
  }, {times: 1});
  const downloaded = page.waitForEvent('download');
  await artifact.click();
  const file = await downloaded;
  expect(file.suggestedFilename()).toBe('client.zip');
  expect(createHash('sha256').update(readFileSync(await file.path())).digest('hex')).toBe(digest);

  for (const suffix of ['/download?channel=preview', '/download/stable/bad%20build/windows/x86_64']) {
    const response = await page.request.get(`/${lang}${suffix}`);
    expect(response.status()).toBe(400);
  }
  expect((await page.request.get(`/${lang}/download/stable/unknown/windows/x86_64`)).status()).toBe(404);
  const betaLink = main.locator('a[href$="?channel=beta"]');
  await betaLink.click();
  await expect(page).toHaveURL(new RegExp(`/${lang}/download\\?channel=beta$`));
  await expect(main).toContainText(label.unavailable);
  await expect(main.locator('a[href*="/download/beta/"]')).toHaveCount(0);
  await expect(main).not.toContainText('Unverified fixture');

  await main.getByRole('link', {name: label.guide, exact: true}).click();
  await expect(page).toHaveURL(new RegExp(`/${lang}/start$`));
  await expect(main.getByRole('heading', {level: 1})).toHaveText(label.guide);
  await expect(main.getByRole('heading', {name: label.update})).toBeVisible();
  const recovery = main.getByRole('link', {name: label.recovery, exact: true});
  await expect(recovery).toHaveAttribute('href', `/${lang}/support`);
  await recovery.click();
  await expect(page).toHaveURL(new RegExp(`/${lang}/support$`));
  await main.locator(`a[href="/${lang}/forgot-password"]`).click();
  await expect(main.locator(`form[action="/${lang}/forgot-password"]`)).toBeVisible();
});

test('i18n-parity for the onboarding pages', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  for (const [route, heading] of [['download', copy[lang].home], ['start', copy[lang].guide]]) {
    expect((await page.goto(`/${lang}/${route}`)).status()).toBe(200);
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    const main = page.getByRole('main');
    await expect(main.getByRole('heading', {level: 1})).toHaveText(heading);
    await expect(page.locator(`footer a[hreflang="${lang === 'pl' ? 'en' : 'pl'}"][href$="/${route}"]`)).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    expect(await main.locator('h2, p, a, li').allTextContents()).not.toContain('');
  }
  const keys = locale => [...readFileSync(new URL(`../../crates/ui/i18n/${locale}.ftl`, import.meta.url), 'utf8')
    .matchAll(/^(onboarding-[\w-]+)\s*=\s*\S/gm)].map(([, key]) => key).sort();
  expect(keys('en').length).toBeGreaterThan(0);
  expect(keys('pl')).toEqual(keys('en'));
});
