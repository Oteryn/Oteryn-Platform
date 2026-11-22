import { test, expect } from '../fixtures.mjs';
import { isolatedAdmin } from '../staff-rbac-helpers.mjs';

test('economy analytics localizes the unavailable view', async ({ browser, page }, info) => {
  const lang = info.project.metadata.locale;
  expect((await page.goto('/admin/analytics/economy')).status()).toBe(403);

  const staff = await isolatedAdmin(browser, info);
  try {
    const response = await staff.page.goto('/admin/analytics/economy');
    expect(response.status()).toBe(200);
    await expect(staff.page.locator('html')).toHaveAttribute('lang', lang);
    const status = staff.page.locator('[data-state="unavailable"]');
    await expect(status).toHaveCount(5);
    const pl = 'Wymagana jest zaakceptowana baza ponownej synchronizacji oraz aktualne dane z Gry';
    const en = 'accepted resync baseline and fresh Game evidence required';
    await expect(status.first()).toContainText(lang === 'pl' ? pl : en);
    await expect(staff.page.locator('main')).not.toContainText(lang === 'pl' ? en : pl);
    expect((await staff.page.request.get('/admin/analytics/economy?world=invalid')).status()).toBe(400);
  } finally {
    await staff.context.close();
  }
});
