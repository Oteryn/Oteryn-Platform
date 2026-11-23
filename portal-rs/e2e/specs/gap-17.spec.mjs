import { readFileSync } from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { query, seededStaff } from '../staff-rbac-helpers.mjs';

const text = {
  en: {
    world: 'World',
    guilds: { submit: 'Search', unavailable: 'Guild information is unavailable.' },
    houses: { submit: 'Show houses', unavailable: 'House information is unavailable. Please try again later.' },
  },
  pl: {
    world: 'Świat',
    guilds: { submit: 'Szukaj', unavailable: 'Informacje o gildiach są niedostępne.' },
    houses: { submit: 'Pokaż domy', unavailable: 'Informacje o domach są niedostępne. Spróbuj ponownie później.' },
  },
};

async function expectSkipLink(page) {
  await page.locator('a[href="#main"]').first().focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('main#main')).toBeFocused();
}

for (const section of ['guilds', 'houses']) {
  test(`visitor opens ${section} and checks availability in a selected world`, async ({ page }, info) => {
    const lang = info.project.metadata.locale;
    const path = `/${lang}/${section}`;
    const labels = text[lang];

    await page.goto(`/${lang}/news`);
    const navigation = page.locator('#portal-navigation');
    const link = navigation.locator(`a[href="${path}"]`).first();
    await expect(link).toBeAttached();
    const menu = page.locator('#portal-header details.om-fallback-menu > summary');
    if (await menu.isVisible()) await menu.click();
    const panel = link.locator('xpath=ancestor::details[1]');
    if (await panel.count()) {
      if (!await panel.evaluate(element => element.open)) {
        await panel.locator(':scope > summary').click();
      }
    }
    await link.click();
    await expect(page).toHaveURL(new RegExp(`${path}$`));

    const worldPicker = page.getByLabel(labels.world, { exact: true });
    await worldPicker.selectOption({ label: 'Local Bazaar World' });
    const world = await worldPicker.inputValue();
    const [selection] = await Promise.all([
      page.waitForResponse(response =>
        response.request().isNavigationRequest() && new URL(response.url()).searchParams.get('world') === world),
      page.getByRole('button', { name: labels[section].submit, exact: true }).click(),
    ]);
    expect(selection.status()).toBe(503);
    expect(selection.headers()['cache-control']).toContain('no-store');
    await expect(page.getByRole('banner')).toBeVisible();
    await expectSkipLink(page);
    await expect(worldPicker).toHaveValue(world);
    await expect(page.locator('main').getByRole('status')).toHaveText(labels[section].unavailable);
    await expect(page.locator('main article')).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    expect((await page.request.get(`${path}?world=invalid`)).status()).toBe(400);
    if (section === 'guilds') {
      expect((await page.request.get(`${path}?q=Red%20Rose%0A`)).status()).toBe(400);
    }
    expect((await page.request.get(`${path}?world=00000000-0000-7000-8000-000000000000`)).status()).toBe(404);
  });
}

test('world operator can read the release decision', async ({ page }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const path = `/admin/${lang}/world-directories`;
  const consolePath = `/console/worlds?lang=${lang}`;
  const staff = await seededStaff(page, lang, 'platform_admin');
  expect((await page.request.get(path)).status()).toBe(403);
  await page.goto(consolePath);
  const link = page.locator(`a[href="${path}"]`);
  await expect(link).toHaveCount(0);

  await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='game_viewer'", [staff]);
  expect((await page.goto(consolePath)).status()).toBe(200);
  const [response] = await Promise.all([
    page.waitForResponse(response => response.request().isNavigationRequest() && new URL(response.url()).pathname === path),
    link.click(),
  ]);
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toContain('no-store');
  await expect(page.getByRole('banner')).toBeVisible();
  await expectSkipLink(page);
  await expect(page.locator('main')).toContainText(
    lang === 'pl' ? 'Oczekuje na kwalifikację po stronie gry' : 'Awaiting Game qualification',
  );
  await expect(page.locator('main form')).toHaveCount(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
  expect((await page.request.get('/admin/xx/world-directories')).status()).toBe(404);
});

test('i18n-parity for the world directory catalogues', () => {
  for (const directory of ['guilds', 'houses', 'world_directories']) {
    const [en, pl] = ['en', 'pl'].map(lang => {
      const catalog = new URL(`../../templates/${directory}/${lang}.ftl`, import.meta.url);
      return [...readFileSync(catalog, 'utf8').matchAll(/^(\w[\w-]*)\s*=\s*(\S.*)$/gm)]
        .map(([, key]) => key)
        .sort();
    });
    expect(en.length).toBeGreaterThan(0);
    expect(pl).toEqual(en);
  }
  for (const lang of ['en', 'pl']) {
    const catalog = new URL(`../../crates/admin/src/console/${lang}.ftl`, import.meta.url);
    expect(readFileSync(catalog, 'utf8')).toMatch(/^world-directories\s*=\s*\S/m);
  }
});
