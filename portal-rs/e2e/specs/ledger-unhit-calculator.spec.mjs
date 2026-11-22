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
