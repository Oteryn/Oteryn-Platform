import { test, expect } from '@playwright/test';
import {
  allowExpectedHttpFailure,
  attachDiagnostics,
  installDiagnostics,
  runBinary,
} from './helpers.mjs';

const mariadbRootPassword = process.env.MARIADB_ROOT_PASSWORD;
const canaryDatabase = process.env.CANARY_DB_DATABASE;
const canaryReadUser = process.env.CANARY_DB_USERNAME;
const redisRuntimeUser = process.env.CANARY_RUNTIME_REDIS_USERNAME;

function requireEnvironment(name, value) {
  if (!value) {
    throw new Error(`${name} is required for resilience acceptance.`);
  }

  return value;
}

function mariadbRoot(sql) {
  return runBinary('mariadb', [
    '--protocol=tcp',
    '-h127.0.0.1',
    '-uroot',
    `-p${requireEnvironment('MARIADB_ROOT_PASSWORD', mariadbRootPassword)}`,
    '-e',
    sql,
  ]);
}

function setRedisRuntimeAcl(rule) {
  return runBinary('redis-cli', [
    'ACL',
    'SETUSER',
    requireEnvironment('CANARY_RUNTIME_REDIS_USERNAME', redisRuntimeUser),
    rule,
  ]);
}

test.setTimeout(120_000);
test.describe.configure({ retries: 0, mode: 'serial' });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@resilience Canary public read fails closed and recovers after grant restoration', async ({ page }) => {
  const database = requireEnvironment('CANARY_DB_DATABASE', canaryDatabase);
  const username = requireEnvironment('CANARY_DB_USERNAME', canaryReadUser);

  const initial = await page.goto('/online');
  expect(initial?.status()).toBe(200);
  await expect(page.getByText('Acceptance Hero')).toBeVisible();

  try {
    mariadbRoot(`REVOKE SELECT ON \`${database}\`.cluster_sessions FROM '${username}'@'%';`);

    allowExpectedHttpFailure(page.__acceptanceDiagnostics, { status: 503, pathname: '/online' });
    const failed = await page.goto('/online');
    expect(failed?.status()).toBe(503);
    await expect(page.locator('body')).not.toContainText('SQLSTATE');
    await expect(page.locator('body')).not.toContainText(requireEnvironment('MARIADB_ROOT_PASSWORD', mariadbRootPassword));
  } finally {
    mariadbRoot(`GRANT SELECT ON \`${database}\`.cluster_sessions TO '${username}'@'%';`);
  }

  const recovered = await page.goto('/online');
  expect(recovered?.status()).toBe(200);
  await expect(page.getByText('Acceptance Hero')).toBeVisible();
});

test('@resilience Redis runtime view degrades and recovers after ACL restoration', async ({ page }) => {
  const initial = await page.goto('/servers');
  expect(initial?.status()).toBe(200);
  await expect(page.getByText(/Runtime:\s*ONLINE/u)).toBeVisible();
  await expect(page.getByText(/Players online:\s*1/u)).toBeVisible();

  try {
    setRedisRuntimeAcl('-hmget');

    const failed = await page.goto('/servers');
    expect(failed?.status()).toBe(200);
    await expect(page.getByRole('status')).toContainText('The live runtime dependency is temporarily unavailable');
    await expect(page.getByText(/Runtime:\s*Unavailable/u)).toBeVisible();
  } finally {
    setRedisRuntimeAcl('+hmget');
  }

  const recovered = await page.goto('/servers');
  expect(recovered?.status()).toBe(200);
  await expect(page.getByText(/Runtime:\s*ONLINE/u)).toBeVisible();
  await expect(page.getByText(/Players online:\s*1/u)).toBeVisible();
});
