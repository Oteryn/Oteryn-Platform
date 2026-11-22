import AxeBuilder from '@axe-core/playwright';
import {test, expect} from '../fixtures.mjs';

test('calculator passes contrast checks and splits loot on form submission', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  expect((await page.goto(`/${lang}/wiki/calculators`)).status()).toBe(200);
  await expect(page.locator('a[href="#main"]')).toHaveCount(1);
  const audit = await new AxeBuilder({page}).withRules(['color-contrast']).analyze();
  expect(audit.violations).toEqual([]);

  const form = page.locator('main form:has([name=calculator][value=loot])');
  await form.locator('[name=total]').fill('10');
  await form.locator('[name=members]').fill('3');
  const [response] = await Promise.all([
    page.waitForResponse(r => new URL(r.url()).searchParams.get('calculator') === 'loot'),
    form.getByRole('button').click(),
  ]);
  expect(response.status()).toBe(200);
  await expect(page.locator('output')).toHaveText('3; 1');
});

for (const [operation, fields, status] of [
  ['experience', {current: '0', target: '2'}, 503],
  ['stamina', {current: '0', offline: '780'}, 200],
  ['training', {remaining: '601', family: '0', dummy: 'oteryn:item.tibia.i28558'}, 503],
]) {
  test(`calculator submits the ${operation} form`, async ({page}, info) => {
    const lang = info.project.metadata.locale;
    const path = `/${lang}/wiki/calculators`;
    expect((await page.goto(path)).status()).toBe(200);
    const form = page.locator(`main form:has([name=calculator][value=${operation}])`);
    for (const [name, value] of Object.entries(fields)) {
      await form.locator(`[name=${name}]`).fill(value);
    }
    const [response] = await Promise.all([
      page.waitForResponse(r => r.request().isNavigationRequest() && new URL(r.url()).pathname === path),
      form.getByRole('button').click(),
    ]);
    expect(response.status()).toBe(status);
    expect(Object.fromEntries(new URL(response.url()).searchParams)).toEqual({calculator: operation, ...fields});
    if (operation === 'stamina') {
      await expect(page.locator('output')).toHaveText('1');
    } else {
      await expect(page.getByRole('alert')).toHaveText(lang === 'pl' ? 'Dane świata są niedostępne' : 'World data is unavailable');
      await expect(page.locator('output')).toHaveCount(0);
    }
  });
}
