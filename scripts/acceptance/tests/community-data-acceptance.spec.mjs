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

const stressEvidenceMarker = '@portal-community-stress long values multi-page results internal 500 containment and recovery';
const communityCoveragePath = path.join(
  repoRoot,
  'scripts/acceptance/coverage/surfaces/community-data-completeness.json',
);
const communityCoverage = JSON.parse(fs.readFileSync(communityCoveragePath, 'utf8'));
const stressEvidence = communityCoverage.state_evidence_extensions?.find(
  (entry) => entry.surface_id === 'public.game-data' && entry.issue === 350,
);
const requiredStressStates = [
  'very-long-values',
  'large-result-set-pagination',
  'internal-error-500-contained',
  'internal-error-restored',
];

if (!stressEvidence) {
  throw new Error('Missing Issue #350 public.game-data state evidence extension.');
}
if (JSON.stringify(stressEvidence.states?.map((state) => state.id)) !== JSON.stringify(requiredStressStates)) {
  throw new Error('Issue #350 state evidence extension does not define the exact required state order.');
}
if (stressEvidence.evidence?.file !== 'scripts/acceptance/tests/community-data-acceptance.spec.mjs') {
  throw new Error('Issue #350 state evidence points at an unexpected evidence file.');
}
if (stressEvidence.evidence?.marker !== stressEvidenceMarker) {
  throw new Error('Issue #350 state evidence marker does not match the executable browser marker.');
}
if (stressEvidence.retries !== 0) {
  throw new Error('Issue #350 state evidence must remain zero retry.');
}

const currentSpec = fs.readFileSync(path.join(repoRoot, stressEvidence.evidence.file), 'utf8');
if (!currentSpec.includes(stressEvidenceMarker)) {
  throw new Error('Issue #350 executable browser marker is missing from the referenced evidence file.');
}

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

async function expectReadableWrappingAndContainment(
  locator,
  { containerSelector, expectedOverflowWrap = null },
) {
  await expect(locator).toBeVisible();

  const metrics = await locator.evaluate((element, options) => {
    const container = element.closest(options.containerSelector);
    if (!(container instanceof HTMLElement)) {
      throw new Error(`Expected ${options.containerSelector} containment for long-value evidence.`);
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
    const style = getComputedStyle(element);

    return {
      lineCount: lineTops.length,
      overflowWrap: style.overflowWrap,
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
  }, { containerSelector });

  expect(metrics.lineCount).toBeGreaterThanOrEqual(2);
  expect(metrics.elementRect.left).toBeGreaterThanOrEqual(metrics.containerRect.left - 1);
  expect(metrics.elementRect.right).toBeLessThanOrEqual(metrics.containerRect.right + 1);
  expect(metrics.elementRect.top).toBeGreaterThanOrEqual(metrics.containerRect.top - 1);
  expect(metrics.elementRect.bottom).toBeLessThanOrEqual(metrics.containerRect.bottom + 1);
  if (expectedOverflowWrap !== null) expect(metrics.overflowWrap).toBe(expectedOverflowWrap);
}

async function expectScrollableTableContainment(locator) {
  const metrics = await locator.evaluate((element) => {
    const region = element.closest('.table-region');
    const table = region?.querySelector('table');
    if (!(region instanceof HTMLElement) || !(table instanceof HTMLTableElement)) {
      throw new Error('Expected long highscore value inside a table-region table.');
    }

    const tableRect = table.getBoundingClientRect();
    const elementRect = element.getBoundingClientRect();

    return {
      overflowX: getComputedStyle(region).overflowX,
      clientWidth: region.clientWidth,
      scrollWidth: region.scrollWidth,
      tableWidth: tableRect.width,
      elementLeft: elementRect.left,
      elementRight: elementRect.right,
      tableLeft: tableRect.left,
      tableRight: tableRect.right,
    };
  });

  expect(['auto', 'scroll']).toContain(metrics.overflowX);
  expect(metrics.scrollWidth).toBeGreaterThanOrEqual(metrics.clientWidth);
  expect(metrics.tableWidth).toBeLessThanOrEqual(metrics.scrollWidth + 1);
  expect(metrics.elementLeft).toBeGreaterThanOrEqual(metrics.tableLeft - 1);
  expect(metrics.elementRight).toBeLessThanOrEqual(metrics.tableRight + 1);
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
  const profileLink = heroCard.getByRole('link', { name: 'Manage public profile' });
  const profileHref = await profileLink.getAttribute('href');
  expect(profileHref).not.toBeNull();
  expect(new URL(profileHref, page.url()).pathname).toBe('/account/characters/Acceptance%20Hero/profile');
  await page.goto(profileHref);

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
test('@portal-community-stress long values, multi-page results, internal 500 containment and recovery', async ({ page }, testInfo) => {
  expect(stressEvidence.projects).toContain(testInfo.project.name);
  expect(stressEvidence.states.map((state) => state.id)).toEqual(requiredStressStates);
  expect(process.env.APP_DEBUG).toBe('false');

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
    const longNameLink = page.getByRole('link', { name: longName, exact: true });
    await expectReadableWrappingAndContainment(longNameLink, {
      containerSelector: 'td',
      expectedOverflowWrap: 'anywhere',
    });
    await expectScrollableTableContainment(longNameLink);
    await expect(page.getByRole('link', { name: 'Next', exact: true })).toBeVisible();
    await expectNoHorizontalOverflow(page);

    await page.getByRole('link', { name: 'Next', exact: true }).click();
    await expect(page.getByRole('link', { name: 'Matrix Character 050', exact: true })).toBeVisible();
    await expectNoHorizontalOverflow(page);

    await page.goto(`/characters/${encodeURIComponent(longName)}`);
    const longNameHeading = page.getByRole('heading', { name: longName, exact: true });
    const longCommentText = page.getByText(longComment, { exact: true });
    await expectReadableWrappingAndContainment(longNameHeading, {
      containerSelector: '.page-header',
    });
    await expectReadableWrappingAndContainment(longCommentText, {
      containerSelector: '.card',
      expectedOverflowWrap: 'anywhere',
    });
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