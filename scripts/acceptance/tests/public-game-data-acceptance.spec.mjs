import { test, expect } from '@playwright/test';
import {
  allowExpectedHttpFailure,
  attachDiagnostics,
  installDiagnostics,
  runBinary,
} from './helpers.mjs';

test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('Flow 6b — public game data, pagination, empty and controlled dependency-failure states', async ({ page }) => {
  await page.goto('/');
  await page.getByLabel('Character name').fill('Acceptance Hero');
  await page.getByRole('button', { name: 'Search' }).click();
  await expect(page.getByRole('heading', { name: 'Acceptance Hero' })).toBeVisible();

  await page.goto('/guilds/Acceptance%20Guild');
  await expect(page.getByRole('heading', { name: 'Acceptance Guild' })).toBeVisible();
  await expect(page.getByText('Acceptance Guildmate')).toBeVisible();

  await page.goto('/online');
  await expect(page.getByText('Acceptance Hero')).toBeVisible();
  await expect(page.getByText(/Acceptance \(Channel ID 1\)/u)).toBeVisible();

  await page.goto('/highscores');
  await expect(page.getByRole('heading', { name: 'Level highscores' })).toBeVisible();
  await expect(page.getByText('Acceptance Hero')).toBeVisible();

  await page.goto('/highscores?page=999');
  await expect(page.getByText('No active characters found.')).toBeVisible();

  await page.goto('/servers');
  await expect(page.locator('p').filter({ hasText: 'Runtime:' })).toContainText('ONLINE');
  await expect(page.locator('p').filter({ hasText: 'Players online:' })).toContainText('1');

  runBinary('redis-cli', ['DEL', 'cluster:channel:1:runtime']);
  await page.goto('/servers');
  await expect(page.locator('p').filter({ hasText: 'Runtime:' })).toContainText('Unknown');

  runBinary('redis-cli', ['HSET', 'cluster:channel:1:runtime', 'channel_id', '1', 'status', 'INVALID', 'players_online', '1']);
  runBinary('redis-cli', ['PEXPIRE', 'cluster:channel:1:runtime', '3600000']);
  await page.goto('/servers');
  await expect(page.getByText(/live runtime dependency is temporarily unavailable/u)).toBeVisible();

  runBinary('redis-cli', ['DEL', 'cluster:channel:1:runtime']);
  runBinary('redis-cli', ['HSET', 'cluster:channel:1:runtime', 'channel_id', '1', 'status', 'ONLINE', 'players_online', '1']);
  runBinary('redis-cli', ['PEXPIRE', 'cluster:channel:1:runtime', '3600000']);

  const rootPassword = process.env.MARIADB_ROOT_PASSWORD;
  const canaryDb = process.env.CANARY_DB_DATABASE;
  const readonlyUser = process.env.CANARY_DB_USERNAME;
  expect(rootPassword).toBeTruthy();
  expect(canaryDb).toBeTruthy();
  expect(readonlyUser).toBeTruthy();

  runBinary('mariadb', [
    '--protocol=tcp', '-h127.0.0.1', '-uroot', `-p${rootPassword}`,
    '-e', `REVOKE SELECT ON \`${canaryDb}\`.cluster_sessions FROM '${readonlyUser}'@'%';`,
  ]);
  try {
    const response = await page.goto('/online');
    expect(response?.status()).toBe(503);
    allowExpectedHttpFailure(page.__acceptanceDiagnostics, { status: 503, pathname: '/online' });
    await expect(page.locator('body')).not.toContainText('SQLSTATE');
    await expect(page.locator('body')).not.toContainText(rootPassword);
  } finally {
    runBinary('mariadb', [
      '--protocol=tcp', '-h127.0.0.1', '-uroot', `-p${rootPassword}`,
      '-e', `GRANT SELECT ON \`${canaryDb}\`.cluster_sessions TO '${readonlyUser}'@'%';`,
    ]);
  }
});
