import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  installDiagnostics,
  runBinary,
} from './helpers.mjs';

test.describe.configure({ retries: 0, mode: 'serial' });

test.beforeEach(async ({ page }) => {
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
});
