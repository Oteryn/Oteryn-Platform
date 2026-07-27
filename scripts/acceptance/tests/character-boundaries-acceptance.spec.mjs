import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  installDiagnostics,
  login,
  register,
  runBinary,
  runPhpState,
  uniqueCharacterName,
  uniqueEmail,
} from './helpers.mjs';

function insertAcceptanceCharacter(name, accountId) {
  const rootPassword = process.env.MARIADB_ROOT_PASSWORD;
  const canaryDb = process.env.CANARY_DB_DATABASE;
  expect(rootPassword).toBeTruthy();
  expect(canaryDb).toBeTruthy();

  runBinary('mariadb', [
    '--protocol=tcp',
    '-h127.0.0.1',
    '-uroot',
    `-p${rootPassword}`,
    '-e',
    `INSERT INTO \`${canaryDb}\`.players (name, account_id) VALUES ('${name}', ${Number(accountId)});`,
  ]);
}

async function submitCharacter(page, name, injectedAccountId = null) {
  await page.goto('/account/characters/create');
  await page.getByLabel('Character name').fill(name);
  await page.getByLabel('Vocation').selectOption('4');
  await page.getByLabel('Sex').selectOption('1');

  if (injectedAccountId !== null) {
    const characterForm = page.locator('form').filter({ has: page.getByLabel('Character name') });
    await characterForm.evaluate((form, accountId) => {
      const input = document.createElement('input');
      input.type = 'hidden';
      input.name = 'account_id';
      input.value = String(accountId);
      form.appendChild(input);
    }, injectedAccountId);
  }

  await page.getByRole('button', { name: 'Create character' }).click();
}

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@portal-account Flow 5b — character validation, duplicate, ownership injection and quota boundaries respect rate limits', async ({ browser, page }) => {
  const email = uniqueEmail('character-boundary');
  const password = 'AcceptanceCharacter!234';
  await register(page, email, password);
  const binding = runPhpState('binding', email);
  expect(binding.status).toBe('ready');
  await login(page, email, password);
  await expect(page).toHaveURL(/\/$/u);

  await submitCharacter(page, 'Bad123');
  await expect(page.getByRole('alert')).toContainText('Use only ASCII letters and spaces.');

  await submitCharacter(page, 'God');
  await expect(page.getByRole('alert')).toContainText('reserved');

  const duplicateName = uniqueCharacterName('Dup');
  insertAcceptanceCharacter(duplicateName, binding.canary_account_id);
  await submitCharacter(page, duplicateName);
  await expect(page.getByRole('status')).toContainText(`Character ${duplicateName} already exists on your account.`);

  const foreignContext = await browser.newContext();
  const foreignPage = await foreignContext.newPage();
  try {
    const foreignEmail = uniqueEmail('character-foreign');
    const foreignPassword = 'AcceptanceForeign!234';
    await register(foreignPage, foreignEmail, foreignPassword);
    const foreignBinding = runPhpState('binding', foreignEmail);
    expect(foreignBinding.status).toBe('ready');

    const ownedName = uniqueCharacterName('Owned');
    await submitCharacter(page, ownedName, foreignBinding.canary_account_id);
    await expect(page.getByRole('status')).toContainText(`Character ${ownedName} created.`);
    const owner = runPhpState('character-owner', ownedName, email);
    expect(owner.canary_account_id).toBe(binding.canary_account_id);
  } finally {
    await foreignContext.close();
  }

  const quotaContext = await browser.newContext();
  const quotaPage = await quotaContext.newPage();
  try {
    const quotaEmail = uniqueEmail('character-quota');
    const quotaPassword = 'AcceptanceQuota!234';
    await register(quotaPage, quotaEmail, quotaPassword);
    const quotaBinding = runPhpState('binding', quotaEmail);
    expect(quotaBinding.status).toBe('ready');

    for (let index = 0; index < 10; index += 1) {
      insertAcceptanceCharacter(uniqueCharacterName('Quota'), quotaBinding.canary_account_id);
    }

    await login(quotaPage, quotaEmail, quotaPassword);
    await expect(quotaPage).toHaveURL(/\/$/u);
    const overQuota = uniqueCharacterName('Limit');
    await submitCharacter(quotaPage, overQuota);
    await expect(quotaPage.getByRole('alert')).toContainText('maximum number of active characters');
  } finally {
    await quotaContext.close();
  }
});
