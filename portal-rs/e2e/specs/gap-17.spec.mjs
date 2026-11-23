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
    await expect(worldPicker).toHaveValue(world);
    await expect(page.locator('main').getByRole('status')).toHaveText(labels[section].unavailable);
    await expect(page.locator('main article')).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    expect((await page.request.get(`${path}?world=invalid`)).status()).toBe(400);
    expect((await page.request.get(`${path}?world=00000000-0000-7000-8000-000000000000`)).status()).toBe(404);
  });
}

test('world operator can read the release decision', async ({ page }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const path = `/admin/${lang}/world-directories`;
  const staff = await seededStaff(page, lang, 'platform_admin');
  expect((await page.request.get(path)).status()).toBe(403);

  await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='game_viewer'", [staff]);
  const response = await page.goto(path);
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toContain('no-store');
  await expect(page.locator('main')).toContainText(
    lang === 'pl' ? 'Oczekuje na kwalifikację po stronie gry' : 'Awaiting Game qualification',
  );
  await expect(page.locator('main form')).toHaveCount(0);
});
