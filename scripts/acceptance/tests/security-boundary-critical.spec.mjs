import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  installDiagnostics,
  login,
  register,
  runBinary,
  uniqueCharacterName,
  uniqueEmail,
} from './helpers.mjs';

const regularPassword = 'AcceptanceSecurityBoundary!234';

function seedReadyAccount(email) {
  runBinary('php', [
    'scripts/acceptance/seed-account-overview-state.php',
    email,
    regularPassword,
    'ready',
  ]);
}

function findSessionCookie(cookies) {
  return cookies.find((cookie) => cookie.httpOnly && cookie.path === '/' && cookie.name !== 'XSRF-TOKEN');
}

async function submitCharacterWithInjectedAccount(page, name, injectedAccountId) {
  await page.goto('/account/characters/create');
  await page.getByLabel('Character name').fill(name);
  await page.getByLabel('Vocation').selectOption('4');
  await page.getByLabel('Sex').selectOption('1');

  const characterForm = page.locator('form').filter({ has: page.getByLabel('Character name') });
  await characterForm.evaluate((form, accountId) => {
    const input = document.createElement('input');
    input.type = 'hidden';
    input.name = 'account_id';
    input.value = String(accountId);
    form.appendChild(input);
  }, injectedAccountId);

  await page.getByRole('button', { name: 'Create character' }).click();
}

test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@smoke @security-boundary login rotates the browser session and preserves configured cookie protections', async ({ page, context }) => {
  const email = uniqueEmail('security-session');
  seedReadyAccount(email);

  await page.goto('/login');
  const beforeSession = findSessionCookie(await context.cookies());
  expect(beforeSession).toBeDefined();
  expect(beforeSession?.httpOnly).toBe(true);
  expect(beforeSession?.sameSite).toBe('Lax');
  expect(beforeSession?.path).toBe('/');
  expect(beforeSession?.secure).toBe(false);
  const beforeSessionValue = beforeSession?.value;

  await login(page, email, regularPassword);
  await expect(page).toHaveURL(/\/$/u);

  const afterSession = findSessionCookie(await context.cookies());
  expect(afterSession).toBeDefined();
  expect(afterSession?.httpOnly).toBe(true);
  expect(afterSession?.sameSite).toBe('Lax');
  expect(afterSession?.path).toBe('/');
  expect(afterSession?.secure).toBe(false);
  expect(afterSession?.value).not.toBe(beforeSessionValue);
});

test('@smoke @security-boundary injected foreign account ownership is ignored by character creation', async ({ page }) => {
  const actorEmail = uniqueEmail('security-owner');
  const foreignEmail = uniqueEmail('security-foreign');
  await register(page, actorEmail, regularPassword);
  await register(page, foreignEmail, regularPassword);

  // Game-account binding is not provisioned without the legacy game database; the injected id is a fixed foreign value.
  const foreignGameAccountId = 987654;

  await login(page, actorEmail, regularPassword);
  await expect(page).toHaveURL(/\/$/u);

  const characterName = uniqueCharacterName('Secure');
  await submitCharacterWithInjectedAccount(page, characterName, foreignGameAccountId);
  // Character creation is switched off: the injected account id cannot create or take ownership of anything.
  await expect(page.getByRole('alert')).toContainText('Character creation is temporarily unavailable.');
});
