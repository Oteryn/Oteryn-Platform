import fs from 'node:fs';
import path from 'node:path';
import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  installDiagnostics,
  login,
  logout,
  repoRoot,
  runArtisan,
  runBinary,
} from './helpers.mjs';

test.setTimeout(180_000);
test.describe.configure({ retries: 0, mode: 'serial' });

test.beforeEach(async ({ page }) => {
  runBinary('php', ['scripts/acceptance/seed-community-data.php']);
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

function acceptanceDatabaseContext() {
  const rootPassword = process.env.MARIADB_ROOT_PASSWORD;
  const canaryDb = process.env.CANARY_DB_DATABASE;

  expect(rootPassword).toBeTruthy();
  expect(canaryDb).toBeTruthy();

  return { rootPassword, canaryDb };
}

function sqlString(value) {
  return `'${value.replaceAll("'", "''")}'`;
}

function runCanarySql({ rootPassword, canaryDb }, sql) {
  return runBinary('mariadb', [
    '--protocol=tcp',
    '-h127.0.0.1',
    '-uroot',
    `-p${rootPassword}`,
    canaryDb,
    '-e',
    sql,
  ]);
}

function resetStressRows(database) {
  runCanarySql(database, [
    'DELETE FROM cluster_sessions WHERE player_id BETWEEN 9100 AND 9199',
    'DELETE FROM player_deaths WHERE player_id BETWEEN 9100 AND 9199',
    'DELETE FROM guild_membership WHERE player_id BETWEEN 9100 AND 9199',
    'DELETE FROM players WHERE id BETWEEN 9100 AND 9199',
  ].join('; '));
}

function seedStressRows(database, longName, longComment) {
  const generatedRows = Array.from({ length: 75 }, (_, index) => {
    const id = 9100 + index;
    const name = `Matrix Character ${String(index + 1).padStart(3, '0')}`;
    const level = 5000 - index;

    return `(${id}, ${sqlString(name)}, 9001, ${level}, 4, ${level * 1000}, '', 0)`;
  });

  generatedRows.push(`(9199, ${sqlString(longName)}, 9001, 9999, 4, 9999000, ${sqlString(longComment)}, 0)`);

  runCanarySql(database, [
    'INSERT INTO players (id, name, account_id, level, vocation, experience, comment, deletion) VALUES',
    generatedRows.join(',\n'),
  ].join('\n'));
}

// Evidence marker: @portal-community complete rankings, privacy-aware profile, owner preferences, deaths, guild search, localization, resilience and responsive lifecycle
test('@portal-community complete rankings, privacy-aware profile, deaths, guild search, localization, resilience and responsive lifecycle', async ({ page }) => {
  await page.goto('/highscores?category=magic&vocation=4&scope=global');
  await expect(page.getByRole('heading', { name: 'Highscores' })).toBeVisible();
  await expect(page.getByText('Acceptance Hero')).toBeVisible();
  await expect(page.getByText('Characters are global in the current Oteryn server model.')).toBeVisible();
  await expect(page.getByRole('cell', { name: '12', exact: true })).toBeVisible();
  await expectNoHorizontalOverflow(page);

  await page.goto('/characters/Acceptance%20Hero');
  await expect(page.getByRole('heading', { name: 'Acceptance Hero' })).toBeVisible();
  await expect(page.getByText('A deterministic public hero comment.')).toBeVisible();
  await expect(page.getByText('Acceptance Hall')).toBeVisible();
  await expect(page.getByText('Acceptance Dragon')).toBeVisible();
  await expect(page.getByText('Acceptance Guildmate')).toBeVisible();
  await expect(page.getByRole('region', { name: 'Public status' }).getByText('Online', { exact: true })).toBeVisible();
  await expect(page.getByText(/1 recorded player kill/u)).toBeVisible();
  await expect(page.locator('body')).not.toContainText('sink@example.invalid');
  await expect(page.locator('body')).not.toContainText('9001');
  await expectNoHorizontalOverflow(page);

  await page.goto('/deaths');
  await expect(page.getByRole('heading', { name: 'Latest deaths' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Acceptance Hero', exact: true })).toBeVisible();
  await expect(page.getByText('Acceptance Dragon')).toBeVisible();
  await expect(page.getByText(/no authoritative world-transfer source/u)).toBeVisible();
  await expectNoHorizontalOverflow(page);

  await page.goto('/guilds?q=Acceptance');
  await expect(page.getByRole('heading', { name: 'Guild directory' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Acceptance Guild', exact: true })).toBeVisible();
  await expect(page.getByText(/Guild administration is not exposed by the Platform/u)).toBeVisible();
  await expectNoHorizontalOverflow(page);

  await page.goto('/guilds?q=No%20such%20guild');
  await expect(page.getByText('No guilds match this search.')).toBeVisible();

  await page.goto('/pl/deaths');
  await expect(page.getByRole('heading', { name: 'Ostatnie zgony' })).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('lang', 'pl');

  const rootPassword = process.env.MARIADB_ROOT_PASSWORD;
  const canaryDb = process.env.CANARY_DB_DATABASE;
  const readonlyUser = process.env.CANARY_DB_USERNAME;
  expect(rootPassword).toBeTruthy();
  expect(canaryDb).toBeTruthy();
  expect(readonlyUser).toBeTruthy();

  runBinary('mariadb', [
    '--protocol=tcp', '-h127.0.0.1', '-uroot', `-p${rootPassword}`,
    '-e', `REVOKE SELECT ON \`${canaryDb}\`.player_deaths FROM '${readonlyUser}'@'%';`,
  ]);

  try {
    const response = await page.goto('/pl/deaths');
    expect(response?.status()).toBe(503);
    await expect(page.getByRole('heading', { name: 'Dane społeczności są tymczasowo niedostępne' })).toBeVisible();
    await expect(page.locator('body')).not.toContainText('SQLSTATE');
    await expect(page.locator('body')).not.toContainText(rootPassword);
  } finally {
    runBinary('mariadb', [
      '--protocol=tcp', '-h127.0.0.1', '-uroot', `-p${rootPassword}`,
      '-e', `GRANT SELECT ON \`${canaryDb}\`.player_deaths TO '${readonlyUser}'@'%';`,
    ]);
  }

  await page.goto('/deaths');
  await expect(page.getByRole('heading', { name: 'Latest deaths' })).toBeVisible();

  await login(
    page,
    'community-acceptance@example.test',
    'acceptance-community-not-a-user-password',
  );
  await page.goto('/account');
  await expect(page.getByRole('heading', { name: 'Account overview' })).toBeVisible();
  const heroCard = page.locator('article.card').filter({ hasText: 'Acceptance Hero' });
  await expect(heroCard.getByText('Default public profile settings')).toBeVisible();
  await heroCard.getByRole('link', { name: 'Manage public profile' }).click();

  await expect(page.getByRole('heading', { name: 'Acceptance Hero' })).toBeVisible();
  await page.getByLabel('Public comment').fill('<script>acceptance-profile-xss</script> Platform owner comment');
  await page.getByLabel('Show other public characters on this account').check();
  await page.getByLabel('Show online state and last login/logout').check();
  await page.getByLabel('Show guild and rank').uncheck();
  await page.getByLabel('Show house ownership').uncheck();
  await page.getByLabel('Show skills').uncheck();
  await page.getByLabel('Show recent deaths').uncheck();
  await page.getByLabel('Show player-kill statistics').uncheck();
  await page.getByLabel('Mark as my main character').check();
  await page.getByRole('button', { name: 'Save public profile settings' }).click();
  await expect(page.getByText('Public profile settings were updated.')).toBeVisible();
  await expectNoHorizontalOverflow(page);

  await page.goto('/characters/Acceptance%20Hero');
  await expect(page.getByText('<script>acceptance-profile-xss</script> Platform owner comment', { exact: true })).toBeVisible();
  await expect(page.locator('script:has-text("acceptance-profile-xss")')).toHaveCount(0);
  await expect(page.getByText('Main character', { exact: true })).toBeVisible();
  await expect(page.getByText('Guild details are private.')).toBeVisible();
  await expect(page.getByText('House details are private.')).toBeVisible();
  await expect(page.getByText('Skills are private.')).toBeVisible();
  await expect(page.getByText('Death history is private.')).toBeVisible();
  await expect(page.getByText('Player-kill statistics are private.')).toBeVisible();
  await expect(page.getByText('Acceptance Guildmate')).toBeVisible();
  await expect(page.getByRole('region', { name: 'Public status' }).getByText('Online', { exact: true })).toBeVisible();
  await expect(page.locator('body')).not.toContainText('9001');
  await expectNoHorizontalOverflow(page);

  await page.goto('/account/characters/Acceptance%20Hero/profile?locale=pl');
  await expect(page.locator('html')).toHaveAttribute('lang', 'pl');
  await expect(page.getByText('Wybierz pola widoczne publicznie dla tej postaci.')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Zapisz ustawienia profilu publicznego' })).toBeVisible();
  await expectNoHorizontalOverflow(page);

  await logout(page);
});

// Evidence marker: @portal-community-stress long values multi-page results internal 500 containment and recovery
test('@portal-community-stress long values, multi-page results, internal 500 containment and recovery', async ({ page }) => {
  const database = acceptanceDatabaseContext();
  const longName = `Boundary ${Array.from({ length: 20 }, (_, index) => `Segment${String(index + 1).padStart(2, '0')}`).join(' ')}`;
  const longComment = `Long public profile boundary ${'C'.repeat(220)}`;
  const highscoreView = path.join(repoRoot, 'resources/views/game/highscores.blade.php');
  const unavailableView = `${highscoreView}.acceptance-unavailable`;
  let viewMoved = false;

  resetStressRows(database);
  seedStressRows(database, longName, longComment);

  try {
    await page.goto('/highscores');
    await expect(page.getByRole('link', { name: longName, exact: true })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Next', exact: true })).toBeVisible();
    await expectNoHorizontalOverflow(page);

    await page.getByRole('link', { name: 'Next', exact: true }).click();
    await expect(page.getByRole('link', { name: 'Matrix Character 050', exact: true })).toBeVisible();
    await expectNoHorizontalOverflow(page);

    await page.goto(`/characters/${encodeURIComponent(longName)}`);
    await expect(page.getByRole('heading', { name: longName, exact: true })).toBeVisible();
    await expect(page.getByText(longComment, { exact: true })).toBeVisible();
    await expectNoHorizontalOverflow(page);

    if (fs.existsSync(unavailableView)) {
      fs.rmSync(unavailableView, { force: true });
    }
    fs.renameSync(highscoreView, unavailableView);
    viewMoved = true;
    runArtisan('view:clear');

    const failedResponse = await page.goto('/highscores', { waitUntil: 'domcontentloaded' });
    expect(failedResponse?.status()).toBe(500);

    const errorBody = await page.locator('body').innerText();
    expect(errorBody.trim()).not.toBe('');
    for (const forbidden of [
      'InvalidArgumentException',
      'View [game.highscores] not found',
      highscoreView,
      database.canaryDb,
      database.rootPassword,
      'SQLSTATE',
      'Stack trace',
    ]) {
      expect(errorBody).not.toContain(forbidden);
    }

    fs.renameSync(unavailableView, highscoreView);
    viewMoved = false;
    runArtisan('view:clear');

    await page.goto('/highscores?page=2');
    await expect(page.getByRole('link', { name: 'Matrix Character 050', exact: true })).toBeVisible();
    await expectNoHorizontalOverflow(page);
  } finally {
    if (viewMoved && fs.existsSync(unavailableView) && !fs.existsSync(highscoreView)) {
      fs.renameSync(unavailableView, highscoreView);
    } else if (fs.existsSync(unavailableView) && fs.existsSync(highscoreView)) {
      fs.rmSync(unavailableView, { force: true });
    }

    runArtisan('view:clear');
    resetStressRows(database);
  }
});
