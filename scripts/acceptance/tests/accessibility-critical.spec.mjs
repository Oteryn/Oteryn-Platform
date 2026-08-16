import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  installDiagnostics,
  logout,
  runBinary,
  uniqueCharacterName,
  uniqueEmail,
} from './helpers.mjs';

const accountPassword = 'AcceptanceAccessibilityAccount!234';
const browserAdminPassword = 'Acceptance-Browser-Admin-9!Pass';
const browserAdminRecoveryCode = 'BROWSER-00001';

function seedReadyAccount(email) {
  return JSON.parse(runBinary('php', [
    'scripts/acceptance/seed-account-overview-state.php',
    email,
    accountPassword,
    'ready',
  ]));
}

function seedVisualFixtures() {
  runBinary('php', ['scripts/acceptance/seed.php', 'seed']);
}

function seedBrowserAdmin(email) {
  runBinary('php', [
    'scripts/acceptance/seed-browser-admin.php',
    email,
    browserAdminPassword,
    browserAdminRecoveryCode,
  ]);
}

async function tabTo(page, locator, label, maxTabs = 100) {
  const target = locator.first();
  await expect(target, `${label} must exist before keyboard traversal`).toBeVisible();

  for (let index = 0; index < maxTabs; index += 1) {
    await page.keyboard.press('Tab');
    const focused = await target.evaluate((element) => element === document.activeElement);
    if (!focused) continue;

    const indicatorVisible = await target.evaluate((element) => {
      const style = getComputedStyle(element);
      return element.matches(':focus-visible')
        && (style.outlineStyle !== 'none'
          || style.outlineWidth !== '0px'
          || style.boxShadow !== 'none');
    });
    expect(indicatorVisible, `${label} must expose a visible keyboard focus indicator`).toBeTruthy();
    return;
  }

  const active = await page.evaluate(() => {
    const element = document.activeElement;
    if (!(element instanceof HTMLElement)) return 'non-html-element';
    return `${element.tagName.toLowerCase()}#${element.id || 'no-id'}.${element.className || 'no-class'}`;
  });
  throw new Error(`Keyboard traversal did not reach ${label} within ${maxTabs} Tab presses; active=${active}`);
}

async function keyboardLogin(page, email, password) {
  await page.goto('/login');

  const emailInput = page.getByLabel('Email');
  await tabTo(page, emailInput, 'login email');
  await page.keyboard.type(email);

  const passwordInput = page.getByLabel('Password');
  await tabTo(page, passwordInput, 'login password');
  await page.keyboard.type(password);

  const submit = page.getByRole('button', { name: 'Sign in' });
  await tabTo(page, submit, 'login submit');
  await page.keyboard.press('Enter');
  await page.waitForURL((url) => url.pathname !== '/login');
}

test.setTimeout(120_000);
test.describe.configure({ mode: 'serial', retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@accessibility production homepage navigation and character search are keyboard operable', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Answer the call of Oteryn' })).toBeVisible();

  const newsLink = page.getByRole('navigation', { name: 'Public navigation' }).getByRole('link', { name: 'News' });
  await tabTo(page, newsLink, 'public News navigation link');

  const characterName = page.getByLabel('Character name');
  await tabTo(page, characterName, 'homepage character search');
  await page.keyboard.type('Acceptance Hero');

  const search = page.getByRole('button', { name: 'Search' });
  await tabTo(page, search, 'homepage character search submit');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { name: 'Acceptance Hero' })).toBeVisible();
});

test('@accessibility login and password-recovery forms are keyboard reachable and activatable', async ({ page }) => {
  const email = uniqueEmail('accessibility-login');
  seedReadyAccount(email);

  await keyboardLogin(page, email, accountPassword);
  await expect(page).toHaveURL(/\/$/u);

  await logout(page);
  await page.goto('/forgot-password');

  const recoveryEmail = page.getByLabel('Email');
  await tabTo(page, recoveryEmail, 'password recovery email');
  await page.keyboard.type(email);

  const recoverySubmit = page.getByRole('button', { name: 'Send reset link' });
  await tabTo(page, recoverySubmit, 'password recovery submit');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('status')).toBeVisible();
});

test('@accessibility account-security privacy controls are keyboard reachable and activatable', async ({ page }) => {
  const email = uniqueEmail('accessibility-security');
  seedReadyAccount(email);

  await keyboardLogin(page, email, accountPassword);
  await page.goto('/account/security');
  await expect(page.getByRole('heading', { name: 'Security and lifecycle' })).toBeVisible();

  const privacy = page.locator('section[aria-labelledby="privacy-heading"]');
  const association = privacy.getByLabel('Allow public characters to show an account association when that feature is delivered.');
  await tabTo(page, association, 'account security association privacy checkbox');
  await page.keyboard.press('Space');
  await expect(association).toBeChecked();

  const status = privacy.getByLabel('Allow public online/status visibility when supported by a public profile.');
  await tabTo(page, status, 'account security status privacy checkbox');
  await page.keyboard.press('Space');
  await expect(status).toBeChecked();

  const save = privacy.getByRole('button', { name: 'Save privacy settings' });
  await tabTo(page, save, 'account security privacy submit');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('status')).toContainText('Account privacy settings have been updated.');
  await expect(association).toBeChecked();
  await expect(status).toBeChecked();
});

test('@accessibility Account Overview reaches character creation and its controls through keyboard traversal', async ({ page }) => {
  const email = uniqueEmail('accessibility-account');
  seedReadyAccount(email);

  await keyboardLogin(page, email, accountPassword);
  await page.goto('/account');
  await expect(page.getByRole('heading', { name: 'Account overview' })).toBeVisible();

  const createLink = page.getByRole('link', { name: 'Create a character' });
  await tabTo(page, createLink, 'account create-character link');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { name: 'Create a character' })).toBeVisible();

  const nameInput = page.getByLabel('Character name');
  await tabTo(page, nameInput, 'character name');
  await page.keyboard.type(uniqueCharacterName('Key'));

  const vocation = page.getByLabel('Vocation');
  await tabTo(page, vocation, 'character vocation');
  await page.keyboard.press('ArrowDown');

  const sex = page.getByLabel('Sex');
  await tabTo(page, sex, 'character sex');
  await page.keyboard.press('ArrowDown');

  const createButton = page.getByRole('button', { name: 'Create character' });
  await tabTo(page, createButton, 'character create submit');
  await page.keyboard.press('Shift+Tab');
  await expect(sex).toBeFocused();
});

test('@accessibility MFA challenge and admin table-to-form navigation work with keyboard only', async ({ page }) => {
  const adminEmail = uniqueEmail('accessibility-admin');
  seedVisualFixtures();
  seedBrowserAdmin(adminEmail);

  await keyboardLogin(page, adminEmail, browserAdminPassword);
  await expect(page.getByRole('heading', { name: 'Complete your sign in' })).toBeVisible();

  const challenge = page.getByLabel('Authenticator or recovery code');
  await tabTo(page, challenge, 'MFA challenge code');
  await page.keyboard.type(browserAdminRecoveryCode);

  const verify = page.getByRole('button', { name: 'Verify and sign in' });
  await tabTo(page, verify, 'MFA challenge submit');
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/\/$/u);

  await page.goto('/admin/pages');
  const row = page.locator('tr').filter({ hasText: 'about-oteryn' });
  const edit = row.getByRole('link', { name: 'Edit' });
  await tabTo(page, edit, 'managed-page edit link');
  await page.keyboard.press('Enter');

  const title = page.getByLabel('Title');
  await tabTo(page, title, 'managed-page title');

  const body = page.getByLabel('Body (plain text)');
  await tabTo(page, body, 'managed-page body');

  const publishAt = page.getByLabel('Publish at');
  await tabTo(page, publishAt, 'managed-page publish date');

  const save = page.getByRole('button', { name: 'Save' });
  await tabTo(page, save, 'managed-page save');
  await page.keyboard.press('Shift+Tab');
  await expect(publishAt).toBeFocused();
});
