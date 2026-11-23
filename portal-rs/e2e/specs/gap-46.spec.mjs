import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { login, pool, register } from '../helpers.mjs';

const label = {
  en: { journal: 'Inventory and history', bank: 'Bank history', unavailable: 'Unavailable' },
  pl: { journal: 'Przedmioty i historia', bank: 'Historia banku', unavailable: 'Niedostępne' },
};

test('account navigation reaches the inventory journal and bank availability', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const words = label[lang];
  const index = `/${lang}/account/journal/items`;
  const db = pool();
  const name = `Journal world ${randomUUID()}`;
  let world;

  try {
    // Registry metadata only. The local stack has no qualified G-10 journal feed.
    const { rows } = await db.query('INSERT INTO world_status(id,name) VALUES (uuidv7(),$1) RETURNING id', [name]);
    world = rows[0].id;
    const anonymous = await page.request.get(index, { maxRedirects: 0 });
    expect(anonymous.status()).toBe(303);
    const account = await register(page, lang);
    await login(page, lang, account);
    await page.goto(`/${lang}`);
    if (info.project.use.viewport.width === 390) {
      const menu = page.locator('#portal-menu-toggle');
      await menu.click();
      await expect(menu).toHaveAttribute('aria-expanded', 'true');
    }
    await page.locator('#portal-header [data-account-link]').click();
    await page.locator('#podglad-konta').getByRole('link', { name: words.journal, exact: true }).click();

    await expect(page).toHaveURL(new RegExp(index + '$'));
    await expect(page.getByRole('heading', { name: words.journal, exact: true })).toBeVisible();
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);

    const bank = `/${lang}/panel/history/${world}/bank`;
    const response = page.waitForResponse(r => r.request().isNavigationRequest() && new URL(r.url()).pathname === bank);
    await page.getByRole('link', { name, exact: true }).click();
    const unavailable = await response;
    expect(unavailable.status()).toBe(503);
    expect(unavailable.headers()['cache-control']).toContain('no-store');
    await expect(page.getByRole('heading', { name: words.bank, exact: true })).toBeVisible();
    await expect(page.getByRole('status')).toHaveText(words.unavailable);
    await expect(page.locator('#portal-header')).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    expect((await page.request.get(`${bank}?cursor=invalid`)).status()).toBe(400);
    expect((await page.request.get(`/${lang}/panel/items/${world}/${world}?location=bank`)).status()).toBe(400);
    await page.getByRole('main').getByRole('link', { name: words.journal, exact: true }).click();
    await expect(page).toHaveURL(new RegExp(index + '$'));
  } finally {
    try {
      if (world) await db.query('DELETE FROM world_status WHERE id=$1', [world]);
    } finally {
      await db.end();
    }
  }
});

test('i18n-parity for the inventory journal', () => {
  const messages = lang => {
    const url = new URL(`../../crates/playerdata/src/items/${lang}.ftl`, import.meta.url);
    return [...readFileSync(url, 'utf8').matchAll(/^(items-[\w-]+) *= *\S/gm)]
      .map(([, key]) => key)
      .sort();
  };
  const english = messages('en');
  expect(english).toContain('items-journal');
  expect(new Set(english).size).toBe(english.length);
  expect(messages('pl')).toEqual(english);
});
