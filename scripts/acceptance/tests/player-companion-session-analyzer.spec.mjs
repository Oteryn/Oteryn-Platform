import { test, expect } from '@playwright/test';
import {
  assertAccessibilitySmoke,
  attachDiagnostics,
  evidenceScreenshot,
  installDiagnostics,
  login,
  register,
  uniqueEmail,
} from './helpers.mjs';

const password = 'AcceptancePlayerCompanion!234';
const desktopViewport = { width: 1440, height: 1000 };
const mobileViewport = { width: 390, height: 844 };
const rawSentinel = 'PRIVATE RAW SENTINEL MUST NOT SURVIVE';
const sessionLog = `Session: 01:00h
XP Gain: 3,600,000
Loot: 600,000
Supplies: 200,000
Balance: 400,000
${rawSentinel}
Alice
Loot: 400,000
Supplies: 100,000
Balance: 300,000
Bob
Loot: 200,000
Supplies: 100,000
Balance: 100,000`;

test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@portal-account Player Companion — private Hunt Session Analyzer create, validation, owner isolation, responsive detail and delete', async ({ page, browser }) => {
  const email = uniqueEmail('player-companion-session');

  await register(page, email, password);
  await login(page, email, password);
  await expect(page).toHaveURL(/\/account$/u);

  const toolLink = page.getByRole('link', { name: 'Open session analyzer' });
  await expect(toolLink).toBeVisible();
  await toolLink.click();
  await expect(page).toHaveURL(/\/account\/tools\/session-analyzer$/u);
  await expect(page.getByRole('heading', { name: 'Hunt Session Analyzer', level: 1 })).toBeVisible();
  await expect(page.getByTestId('session-analysis-empty')).toBeVisible();

  await page.getByLabel('Session label').fill('Acceptance duo');
  await page.getByLabel('Session log').fill(sessionLog);
  await page.getByRole('button', { name: 'Analyze and save privately' }).click();

  await expect(page).toHaveURL(/\/account\/tools\/session-analyzer\/\d+$/u);
  await expect(page.getByRole('status')).toContainText('Session analysis saved privately.');
  await expect(page.getByRole('heading', { name: 'Acceptance duo', level: 1 })).toBeVisible();
  await expect(page.getByTestId('session-analysis-metrics')).toContainText('400,000');
  await expect(page.getByTestId('session-analysis-settlements')).toContainText('Alice');
  await expect(page.getByTestId('session-analysis-settlements')).toContainText('Bob');
  await expect(page.locator('body')).not.toContainText(rawSentinel);
  const privateAnalysisUrl = page.url();

  const foreignContext = await browser.newContext();
  try {
    const foreignPage = await foreignContext.newPage();
    const foreignEmail = uniqueEmail('player-companion-foreign');
    await register(foreignPage, foreignEmail, password);
    await login(foreignPage, foreignEmail, password);
    const foreignResponse = await foreignPage.goto(privateAnalysisUrl);
    expect(foreignResponse?.status()).toBe(404);
    await expect(foreignPage.locator('body')).not.toContainText('Acceptance duo');
  } finally {
    await foreignContext.close();
  }

  await assertAccessibilitySmoke(page);
  await evidenceScreenshot(page, 'player-companion-session-analyzer-detail-desktop');

  await page.setViewportSize(mobileViewport);
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Acceptance duo', level: 1 })).toBeVisible();
  await assertAccessibilitySmoke(page);
  await evidenceScreenshot(page, 'player-companion-session-analyzer-detail-mobile');

  await page.setViewportSize(desktopViewport);
  await page.getByRole('link', { name: 'Back to analyzer' }).click();
  await expect(page.getByText('Acceptance duo', { exact: true })).toBeVisible();
  await page.getByRole('link', { name: 'Open analysis' }).click();

  page.once('dialog', (dialog) => dialog.accept());
  await page.getByRole('button', { name: 'Delete analysis' }).click();
  await expect(page).toHaveURL(/\/account\/tools\/session-analyzer$/u);
  await expect(page.getByRole('status')).toContainText('Session analysis deleted.');
  await expect(page.getByTestId('session-analysis-empty')).toBeVisible();

  await page.getByLabel('Session log').fill('unsupported private input');
  await page.getByRole('button', { name: 'Analyze and save privately' }).click();
  await expect(page.getByRole('alert')).toContainText('supported Session duration');
  await expect(page.getByLabel('Session log')).toHaveValue('');

  await page.getByLabel('Session log').evaluate((element, value) => {
    element.removeAttribute('maxlength');
    element.value = value;
    element.dispatchEvent(new Event('input', { bubbles: true }));
  }, `Session: 01:00h\nLoot: ${'1'.repeat(65_536)}`);
  await page.getByRole('button', { name: 'Analyze and save privately' }).click();
  await expect(page.getByRole('alert')).toContainText('at most 65,535 bytes');
  await expect(page.getByLabel('Session log')).toHaveValue('');
});
