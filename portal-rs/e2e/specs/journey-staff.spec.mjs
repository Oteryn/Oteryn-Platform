import { randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { register, submit, waitForLoginBudget } from '../helpers.mjs';
import { action } from '../staff-rbac-helpers.mjs';

async function openAccountMenu(page) {
  if (await page.locator('#portal-header.om-interior').count()) {
    await page.locator('.om-fallback-menu > summary').click();
    await page.locator('.om-fallback-account > summary').click();
    return page.locator('.om-fallback-account[open]');
  }
  const toggle = page.locator('#portal-menu-toggle');
  if (await toggle.isVisible()) await toggle.click();
  await page.locator('[data-account-link]').click();
  const menu = page.locator('#podglad-konta');
  await expect(menu).toBeVisible();
  return menu;
}

test('the player opens a ticket and reports an incident for staff review', async ({ page, browser }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const pl = lang === 'pl';
  const base = `/${lang}/support`;
  const ticketTitle = pl ? 'Zgłoszenia pomocy' : 'Support tickets';
  const subject = `Client download ${randomUUID()}`;
  const question = 'Where can I find the Linux client?';
  const reportTarget = `Player ${randomUUID()}`;
  const evidence = 'The player repeatedly sent abusive messages in public chat.';
  const main = page.getByRole('main');
  let ticketPath;
  let reportPath;

  await test.step('The player verifies their address and opens a signed-in session', async () => {
    const account = await register(page, lang);
    await expect(main.getByRole('status')).toHaveText(pl
      ? 'Twój adres e-mail został zweryfikowany.'
      : 'Your e-mail has been verified.');
    await page.locator('.om-fallback-menu > summary').click();
    await page.locator(`#portal-navigation a.om-login[href="/${lang}/login"]`).click();
    await waitForLoginBudget();
    await submit(page, 'main form', { email: account.email, password: account.password });
    await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
    await expect(page.locator('[data-account-link]')).toHaveCount(1);
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
  });

  await test.step('The player asks support about a download and finds the open ticket in their list', async () => {
    const menu = await openAccountMenu(page);
    await menu.locator(`a[href="${base}/tickets"]`).click();
    await expect(main.getByRole('heading', { level: 1 })).toHaveText(ticketTitle);
    await main.getByRole('link', { name: pl ? 'Utwórz zgłoszenie' : 'Create ticket' }).click();
    const form = `main form[action="${base}/tickets"]`;
    await expect(main.getByText(pl ? 'Wiadomość' : 'Message', { exact: true })).toBeVisible();
    await action(page, form, { category: 'technical', subject, body: question });
    await expect(page).toHaveURL(/\/support\/tickets\/[a-f0-9-]+$/);
    ticketPath = new URL(page.url()).pathname;
    await expect(main).toContainText(question);
    await expect(main).toContainText(pl ? 'Status: Otwarte' : 'Status: Open');
    await main.getByRole('link', { name: ticketTitle, exact: true }).click();
    await main.getByRole('link', { name: subject, exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`${ticketPath}$`));
    await expect(main).toContainText(question);
  });

  await test.step('The player corrects a mismatched category and sees the submitted report in history', async () => {
    const menu = await openAccountMenu(page);
    await menu.locator(`a[href="${base}/reports"]`).click();
    await expect(main.getByRole('heading', { level: 1 })).toHaveText(pl ? 'Zgłoszenia' : 'Reports');
    await main.getByRole('link', { name: pl ? 'Nowe zgłoszenie' : 'New report' }).click();
    const form = `main form[action="${base}/reports"]`;
    const fields = { report_type: 'player', target_reference: reportTarget, evidence_summary: evidence };
    await action(page, form, { ...fields, category: 'misinformation' });
    await expect(main.getByRole('alert')).toContainText(pl ? 'Sprawdź rodzaj zgłoszenia' : 'Check the report type');
    await expect(main.locator('[name=target_reference]')).toHaveValue(reportTarget);
    await action(page, form, { ...fields, category: 'harassment' });
    await expect(page).toHaveURL(/\/support\/reports\/[a-f0-9-]+$/);
    reportPath = new URL(page.url()).pathname;
    await expect(main).toContainText(evidence);
    await expect(main).toContainText(pl ? 'Status: Wysłane' : 'Status: Submitted');
    await main.getByRole('link', { name: pl ? 'Historia (ostatnie 30 dni)' : 'History (last 30 days)' }).click();
    await main.getByRole('link', { name: reportTarget, exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`${reportPath}$`));
    await expect(main).toContainText(evidence);
  });

  await test.step('A signed-out visitor cannot open either private conversation', async () => {
    const anonymous = await browser.newContext({ ignoreHTTPSErrors: true });
    try {
      for (const path of [ticketPath, reportPath]) {
        const response = await anonymous.request.get(new URL(path, page.url()).href, { maxRedirects: 0 });
        expect(response.status()).toBe(303);
        expect(response.headers().location).toBe(`/${lang}/login`);
      }
    } finally {
      await anonymous.close();
    }
  });
});
