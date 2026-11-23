import { test, expect } from '../fixtures.mjs';
import { messages, seedPlayer } from '../public-depth-helpers.mjs';
import { waitForLoginBudget } from '../helpers.mjs';

async function enterMain(page) {
  await page.keyboard.press('Tab');
  await expect(page.locator('a[href="#main"]').first()).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('main')).toBeFocused();
}

async function tabTo(page, target) {
  await expect(target).toBeVisible();
  for (let tabs = 0; tabs < 100; tabs++) {
    if (await target.evaluate(node => document.activeElement === node)) return;
    await page.keyboard.press('Tab');
  }
  await expect(target).toBeFocused();
}

async function follow(page, target) {
  await tabTo(page, target);
  const [response] = await Promise.all([
    page.waitForNavigation(),
    page.keyboard.press('Enter'),
  ]);
  return response;
}

async function openMenu(page, selector) {
  const menu = typeof selector === 'string' ? page.locator(selector) : selector;
  if (await menu.isVisible()) {
    await tabTo(page, menu);
    await page.keyboard.press('Enter');
  }
}

test('sign-in validation and shop result are reachable by keyboard', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const login = messages('templates/account/mfa', lang);
  const shop = messages('crates/store/src', `web_${lang}`);
  const actor = await seedPlayer(lang);

  await waitForLoginBudget(2);
  await page.goto(`/${lang}/login`);
  await enterMain(page);
  const email = page.getByRole('textbox', { name: login['mfa-email'], exact: true });
  const password = page.getByLabel(login['mfa-password'], { exact: true });
  const submit = page.getByRole('button', { name: login['mfa-login'], exact: true });
  await page.keyboard.press('Tab');
  await expect(email).toBeFocused();
  await page.keyboard.type('malformed');
  await page.keyboard.press('Tab');
  await expect(password).toBeFocused();
  await page.keyboard.type('Wrong password');
  await page.keyboard.press('Tab');
  await expect(submit).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(email).toBeFocused();
  expect(await email.evaluate(field => field.validity.typeMismatch)).toBe(true);
  await page.keyboard.press('Control+A');
  await page.keyboard.type(actor.email);
  await page.keyboard.press('Tab');
  await page.keyboard.press('Tab');
  await page.keyboard.press('Enter');

  const error = page.getByRole('alert').filter({ hasText: login['mfa-error'] });
  await expect(error).toBeFocused();
  await expect(email).toHaveAccessibleDescription(login['mfa-error']);
  expect(await error.ariaSnapshot()).toContain(login['mfa-error']);
  await page.keyboard.press('Tab');
  await expect(email).toBeFocused();
  await page.keyboard.type(actor.email);
  await page.keyboard.press('Tab');
  await expect(password).toBeFocused();
  await page.keyboard.type(actor.password);
  await page.keyboard.press('Tab');
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(new RegExp(`/${lang}/?$`));

  await openMenu(page, '#portal-menu-toggle');
  await follow(page, page.locator(`#portal-header a.om-shop[href="/${lang}/shop"]`));
  await expect(page).toHaveURL(new RegExp(`/${lang}/shop$`));
  await enterMain(page);
  const filter = page.getByRole('searchbox', { name: shop['offer-filter'] });
  await page.keyboard.press('Tab');
  await expect(filter).toBeFocused();
  await page.keyboard.type('premium');
  await page.keyboard.press('Tab');
  await expect(page.getByRole('button', { name: shop['apply-filter'] })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/\/shop\?q=premium$/);
  const result = page.getByRole('status').filter({ hasText: shop.unavailable });
  await expect(result).toBeFocused();
  await expect(filter).toHaveValue('premium');
  await page.keyboard.press('Tab');
  await expect(filter).toBeFocused();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);

  await page.keyboard.press('Control+A');
  await page.keyboard.type('<invalid>');
  await page.keyboard.press('Enter');
  const invalid = page.getByRole('alert').filter({ hasText: shop.invalid });
  await expect(invalid).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(filter).toBeFocused();
});


test('a visitor reaches the atlas and navigates a localized data table by keyboard', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  await page.goto(`/${lang}/news`);
  await openMenu(page, '#portal-header details.om-fallback-menu > summary');
  const world = page.locator('details.om-section').filter({ has: page.locator('#portal-world-panel') });
  await openMenu(page, world.locator('summary'));
  const atlas = page.locator(`#portal-world-panel a[href="/${lang}/atlas"]`);
  const unavailable = await follow(page, atlas);
  expect(unavailable.status()).toBe(503);
  expect(unavailable.headers()['cache-control']).toContain('no-store');
  await expect(page.getByRole('main')).toBeVisible();
  await expect(page.locator('main canvas')).toHaveCount(0);
  await enterMain(page);

  await follow(page, page.locator(`footer a[href="/${lang}/events"]`));
  await expect(page).toHaveURL(new RegExp(`/${lang}/events$`));
  await enterMain(page);
  await follow(page, page.locator(`main a[href="/${lang}/events/calendar/month"]`));
  await expect(page).toHaveURL(new RegExp(`/${lang}/events/calendar/month$`));
  const table = page.getByRole('table');
  const month = await page.locator('#event-month-title').textContent();
  await expect(table).toHaveAccessibleName(month);
  await expect(table.getByRole('columnheader')).toHaveCount(7);
  await expect(table.getByRole('columnheader').first()).toHaveAccessibleName(lang === 'pl' ? /Pon/i : /Mon/i);
  await enterMain(page);
  await follow(page, page.locator('main a[rel=next]'));
  await expect(table).not.toHaveAccessibleName(month);
  await expect(table.getByRole('columnheader')).toHaveCount(7);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test('comparison validation is exposed before unavailable chart data', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const copy = messages('templates/stats/character', lang);
  const target = `/${lang}/stats/compare`;
  const response = await page.goto(`${target}?metric=unsupported`);
  expect(response.status()).toBe(400);
  await expect(page.getByRole('alert').filter({ hasText: copy.invalid })).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(page.getByLabel(copy.start, { exact: true })).toBeFocused();
  await expect(page.getByLabel(copy.metric, { exact: true })).toBeVisible();
  await expect(page.getByRole('table')).toHaveCount(0);
  await expect(page.locator('main figure')).toHaveCount(0);
});
