import { harness } from '../api/transport.mjs';
import { expect, test } from '../fixtures.mjs';
import { csrfRejected, login } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';

async function openAccountMenu(page, info) {
  const lang = info.project.metadata.locale;
  await login(page, lang, {
    email: harness().seed.coinAccount1.email,
    password: process.env.PORTAL_API_LOGIN_PASSWORD,
  });
  expect((await page.goto(`/${lang}/`)).status()).toBe(200);
  if (info.project.use.viewport.width <= 960) {
    await page.locator('#portal-menu-toggle').click();
  }
  await page.locator('[data-account-link]').click();
  await expect(page.getByRole('dialog')).toBeVisible();
}

test('the localized account link opens the creation form @route-enabled', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const target = `/${lang}/account/characters/new`;
  const worldName = 'Character Navigation World';
  const [world] = await query(
    'INSERT INTO world_status (id, name) VALUES (uuidv7(), $1) RETURNING id',
    [worldName],
  );
  await openAccountMenu(page, info);

  const link = page.getByRole('dialog').getByRole('link', {
    name: lang === 'pl' ? 'Utwórz postać' : 'Create a character',
    exact: true,
  });
  await expect(link).toHaveAttribute('href', target);
  const destination = new URL(target, page.url()).href;
  const [response] = await Promise.all([
    page.waitForResponse(response => response.request().isNavigationRequest() && response.url() === destination),
    link.click(),
  ]);
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toBe('no-store, max-age=0');
  await expect(page).toHaveURL(destination);
  const form = page.locator(`main form[method="post"][action="${target}"]`);
  await expect(form).toBeVisible();
  await expect(form.locator(`select[name="world"] option[value="${world.id}"]`)).toHaveText(worldName);
  await csrfRejected(page, target);
});

test('an unmounted route is absent from account navigation @route-disabled', async ({ page }, info) => {
  const target = `/${info.project.metadata.locale}/account/characters/new`;
  await openAccountMenu(page, info);

  const response = await page.request.get(target, { maxRedirects: 0 });
  expect(response.status()).toBe(404);
  await expect(page.locator(`a[href="${target}"]`)).toHaveCount(0);
});
