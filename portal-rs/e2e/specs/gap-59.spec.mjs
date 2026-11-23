import { randomInt } from 'node:crypto';
import { messages } from '../public-depth-helpers.mjs';
import { test, expect } from '../fixtures.mjs';
import { action, seededAccount, seededLogin, seededStaff } from '../staff-rbac-helpers.mjs';

test('Atlas reports appear in staff moderation and return decisions to their reporter', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const origin = new URL(info.project.use.baseURL).origin;
  const history = `/${lang}/support/reports`;
  const staffQueue = '/admin/moderation/reports';
  const contextOptions = {
    baseURL: origin, ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport,
  };
  const staffContext = await browser.newContext(contextOptions);
  const otherContext = await browser.newContext(contextOptions);
  try {
    await seededLogin(page, lang, await seededAccount(lang));
    await page.goto(`/${lang}/`);

    const markers = await page.request.get('/atlas/private-markers');
    expect(markers.status()).toBe(200);
    const { csrf_token } = await markers.json();
    const headers = { origin, 'x-csrf-token': csrf_token };
    const intake = '/atlas/map-reports';
    const report = { x: randomInt(1, 65536), y: randomInt(1, 65536), floor: 7, text: 'A map correction from the Atlas browser flow.' };
    expect((await page.request.post(intake, { headers: { origin }, data: report })).status()).toBe(403);
    expect((await page.request.post(intake, { headers, data: { ...report, x: -1 } })).status()).toBe(422);
    expect((await page.request.post(intake, { headers, data: { ...report, text: 'a'.repeat(2001) } })).status()).toBe(422);
    expect((await page.request.post(intake, { headers, data: report })).status()).toBe(201);

    const staff = await staffContext.newPage();
    await seededStaff(staff, lang, 'platform_admin');
    expect((await staff.goto('/admin')).status()).toBe(200);
    const queueLink = staff.locator(`main nav a[href="${staffQueue}"]`);
    await expect(queueLink).toHaveText(lang === 'pl' ? 'Moderacja zgłoszeń' : 'Report moderation');
    await queueLink.click();
    await expect(staff).toHaveURL(new URL(staffQueue, origin).href);
    await staff.getByRole('link', { name: `Atlas · (${report.x}, ${report.y}, ${report.floor})`, exact: true }).click();
    const staffDetail = new URL(staff.url()).pathname;
    const id = staffDetail.split('/').at(-1);
    await expect(staff.locator('main')).toContainText(report.text);
    const outcome = 'Atlas landmark corrected <north of the bridge>.';
    const notes = 'Internal map review record.';
    await action(staff, 'form:has(input[value=moderate])', {
      status: 'actioned', public_outcome: outcome, moderator_notes: notes,
    });
    await expect(staff).toHaveURL(new RegExp(`/${id}$`));
    await expect(staff.locator('main')).toContainText(outcome);

    if (info.project.use.viewport.width <= 960) {
      await page.locator('#portal-menu-toggle').click();
    }
    await page.locator('[data-account-link]').click();
    await page.locator(`#podglad-konta a[href="${history}"]`).click();
    await expect(page).toHaveURL(new URL(history, origin).href);
    const entry = page.locator(`main article:has(a[href="${history}/${id}"])`);
    await expect(entry).toContainText(lang === 'pl' ? 'Błąd mapy' : 'Map error');
    await expect(entry).toContainText(lang === 'pl' ? 'Rozwiązane' : 'Resolved');
    await expect(entry).toContainText(outcome);
    await entry.getByRole('link').click();
    await expect(page.locator('main')).toContainText(report.text);
    await expect(page.locator('main')).toContainText(outcome);
    await expect(page.locator('main')).not.toContainText(notes);
    for (const screen of [page, staff]) {
      expect(await screen.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    }

    const other = await otherContext.newPage();
    await seededLogin(other, lang, await seededAccount(lang));
    const refusal = await other.request.get(`${history}/${id}`);
    expect(refusal.status()).toBe(404);
    expect(await refusal.text()).not.toContain(outcome);
    expect((await other.request.get(staffDetail)).status()).toBe(403);
  } finally {
    await Promise.all([staffContext.close(), otherContext.close()]);
  }
});

test('i18n-parity for the reports page', () => {
  for (const [directory, required] of [
    ['crates/support/src/reports', ['atlas', 'map-error', 'resolved', 'rejected', 'outcome']],
    ['templates/admin', ['reports']],
  ]) {
    const [en, pl] = ['en', 'pl'].map(lang => messages(directory, lang));
    expect(Object.keys(en).sort()).toEqual(Object.keys(pl).sort());
    for (const key of required) {
      expect(en[key]).toBeTruthy();
      expect(pl[key]).toBeTruthy();
    }
  }
});
