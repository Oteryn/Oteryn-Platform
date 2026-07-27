import { test, expect } from '@playwright/test';
import {
  assertAccessibilitySmoke,
  attachDiagnostics,
  evidenceScreenshot,
  installDiagnostics,
  login,
  register,
  runBinary,
  uniqueEmail,
} from './helpers.mjs';

const password = 'AccountOverview!234';
const desktopViewport = { width: 1440, height: 1100 };
const mobileViewport = { width: 390, height: 844 };

function runPhpState(action, ...args) {
  return JSON.parse(runBinary('php', ['scripts/acceptance/seed-account-overview-state.php', action, ...args]));
}

function seedState(email, state) {
  return runPhpState('seed', email, state);
}

async function assertState(page, state) {
  const expected = {
    missing: ['Game account unavailable', 'Your game account could not be found.'],
    pending: ['Game account setup in progress', 'Character creation will become available after setup completes.'],
    conflict: ['Game account conflict', 'Contact support before creating a character.'],
    recoverable: ['Game account setup needs attention', 'Retry game account setup'],
    ready: ['Ready', 'Your game account setup is complete and character creation is available.'],
  }[state];

  for (const text of expected) {
    await expect(page.getByText(text, { exact: true }).first()).toBeVisible();
  }

  await assertAccessibilitySmoke(page);
  const dimensions = await page.evaluate(() => ({
    viewport: window.innerWidth,
    document: document.documentElement.scrollWidth,
  }));
  expect(dimensions.document).toBeLessThanOrEqual(dimensions.viewport + 1);
}

test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@portal-account Account Overview — authorization, status matrix, responsive evidence and recoverable retry', async ({ page }) => {
  await page.goto('/account');
  await expect(page).toHaveURL(/\/login$/u);

  const email = uniqueEmail('account-overview');
  await register(page, email, password);
  await login(page, email, password);

  const states = ['missing', 'pending', 'conflict', 'ready'];
  for (const state of states) {
    const fixture = seedState(email, state);

    await page.setViewportSize(desktopViewport);
    await page.goto('/account');
    await assertState(page, state);
    await evidenceScreenshot(page, `account-overview-${state}-desktop`);

    const body = await page.locator('body').innerText();
    if (fixture.canary_account_id) {
      expect(body).not.toContain(`Canary account ID: ${fixture.canary_account_id}`);
      expect(body).not.toContain(`Game account ID: ${fixture.canary_account_id}`);
      await expect(page.getByText(String(fixture.canary_account_id), { exact: true })).toHaveCount(0);
    }
    if (fixture.provisioning_name) {
      expect(body).not.toContain(fixture.provisioning_name);
    }

    await page.setViewportSize(mobileViewport);
    await page.reload();
    await assertState(page, state);
    await evidenceScreenshot(page, `account-overview-${state}-mobile`);
  }

  seedState(email, 'recoverable');
  await page.setViewportSize(desktopViewport);
  await page.goto('/account');
  await page.getByRole('button', { name: 'Retry game account setup' }).click();
  await expect(page).toHaveURL(/\/account$/u);
  await expect(page.getByRole('status')).toContainText('Game account setup completed.');
  await expect(page.getByText('Ready', { exact: true })).toBeVisible();

  const binding = runPhpState('binding', email);
  expect(binding.status).toBe('ready');
  expect(binding.canary_account_id).toBeGreaterThan(0);
  const readyBody = await page.locator('body').innerText();
  expect(readyBody).not.toContain(`Canary account ID: ${binding.canary_account_id}`);
  expect(readyBody).not.toContain(`Game account ID: ${binding.canary_account_id}`);
  await expect(page.getByText(String(binding.canary_account_id), { exact: true })).toHaveCount(0);
  await evidenceScreenshot(page, 'account-overview-retry-success-desktop');
});
