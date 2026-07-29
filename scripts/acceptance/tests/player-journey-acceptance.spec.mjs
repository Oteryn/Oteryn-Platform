import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  completeMfaChallenge,
  enrollMfa,
  installDiagnostics,
  login,
  logout,
  register,
  runPhpState,
  uniqueCharacterName,
  uniqueEmail,
  waitForDifferentTotp,
} from './helpers.mjs';

test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

async function createCharacter(page, name) {
  await page.goto('/account/characters/create');
  await page.getByLabel('Character name').fill(name);
  await page.getByLabel('Vocation').selectOption('4');
  await page.getByLabel('Sex').selectOption('1');
  await page.getByRole('button', { name: 'Create character' }).click();
}

test('@portal-account Flows 1-2 — new and returning player complete the production-like browser journey', async ({ page }) => {
  const email = uniqueEmail('player-journey');
  const password = 'AcceptanceJourney!234';
  const character = uniqueCharacterName('Journey');

  await register(page, email, password);
  const binding = runPhpState('binding', email);
  expect(binding.status).toBe('ready');
  expect(binding.canary_account_id).toBeGreaterThan(0);

  await login(page, email, password);
  await expect(page).toHaveURL(/\/$/u);

  const mfa = await enrollMfa(page, password);
  let lastTotp = mfa.enrollmentCode;

  await createCharacter(page, character);
  await expect(page.getByRole('status')).toContainText(`Character ${character} created.`);

  const owner = runPhpState('character-owner', character, email);
  expect(owner.canary_account_id).toBe(binding.canary_account_id);

  await page.goto(`/characters/${encodeURIComponent(character)}`);
  await expect(page.getByRole('heading', { name: character })).toBeVisible();
  const overview = page.getByRole('region', { name: 'Overview' });
  await expect(overview.getByText('Level', { exact: true })).toBeVisible();
  await expect(overview.getByText('8', { exact: true })).toBeVisible();
  await expect(overview.getByText('Vocation', { exact: true })).toBeVisible();
  await expect(overview.getByText('Knight', { exact: true })).toBeVisible();

  await logout(page);
  await login(page, email, password);
  await expect(page.getByRole('heading', { name: 'Complete your sign in' })).toBeVisible();

  const returnCode = await waitForDifferentTotp(mfa.secret, lastTotp);
  await completeMfaChallenge(page, returnCode);
  lastTotp = returnCode;
  await expect(page).toHaveURL(/\/$/u);
  expect(lastTotp).not.toBe(mfa.enrollmentCode);

  await page.goto('/account/characters/create');
  await expect(page.getByRole('heading', { name: 'Create a character' })).toBeVisible();

  await logout(page);
  await page.goto('/account/characters/create');
  await expect(page).toHaveURL(/\/login$/u);
});
