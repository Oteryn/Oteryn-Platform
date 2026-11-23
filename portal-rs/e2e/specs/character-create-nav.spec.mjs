import { run as withPortal } from '../api/harness.mjs';
import { expect, test as browserTest } from '../fixtures.mjs';
import { csrfRejected, login } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';

const routeFlags = [
  'OTERYN_CHARACTER_CREATE_WEB_ENABLED',
  'OTERYN_CHARACTER_COMMANDS_ENABLED',
  'OTERYN_CHARACTER_COMMAND_CREATE_ENABLED',
];
const loginFlag = 'OTERYN_ACCOUNT_LOGIN';

const test = browserTest.extend({
  disabledFlag: [null, { option: true }],
  portal: async ({ disabledFlag }, use, info) => {
    // Each worker gets ports outside the main and outbox harnesses.
    const port = 18640 + info.parallelIndex * 10;
    await withPortal({
      browser: true,
      harnessEnv: {
        PORTAL_E2E_HTTP_PORT: String(port),
        PORTAL_E2E_INTERNAL_PORT: String(port + 1),
        PORTAL_E2E_ADMISSION_PORT: String(port + 2),
        PORTAL_E2E_GATEWAY_PORT: String(port + 3),
        PORTAL_API_NATIVE_PORT: String(port + 4),
        PORTAL_API_SMTP_PORT: String(port + 5),
        PORTAL_E2E_KEEP_DATABASE: '0',
      },
      portalOptions: {
        ...Object.fromEntries([...routeFlags, loginFlag].map(flag => [flag, String(flag !== disabledFlag)])),
        OTERYN_CHARACTER_COMMAND_SOURCE_AUTHORITY: 'oteryn-game-e2e',
      },
    }, use);
  },
  baseURL: async ({ portal }, use) => use(portal.origin),
});

test.use({ trace: 'off', screenshot: 'off' });

async function openAccountMenu(page, info, portal) {
  const lang = info.project.metadata.locale;
  await login(page, lang, portal.account, portal.databaseURL);
  expect((await page.goto(`/${lang}/`)).status()).toBe(200);
  if (info.project.use.viewport.width <= 960) {
    await page.locator('#portal-menu-toggle').click();
  }
  await page.locator('[data-account-link]').click();
  await expect(page.getByRole('dialog')).toBeVisible();
}

test('the localized account link opens the creation form', async ({ page, portal }, info) => {
  const lang = info.project.metadata.locale;
  const target = `/${lang}/account/characters/new`;
  const worldName = 'Character Navigation World';
  const [world] = await query(
    'INSERT INTO world_status (id, name) VALUES (uuidv7(), $1) RETURNING id',
    [worldName],
    portal.databaseURL,
  );
  expect((await page.goto(`/${lang}/`)).status()).toBe(200);
  await expect(page.locator(`#jak-zaczac a[href="${target}"]`)).toBeVisible();
  await openAccountMenu(page, info, portal);

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

for (const flag of routeFlags) {
  test.describe(`with ${flag}=false`, () => {
    test.use({ disabledFlag: flag });

    test('the unmounted route is absent from account navigation', async ({ page, portal }, info) => {
      const target = `/${info.project.metadata.locale}/account/characters/new`;
      await openAccountMenu(page, info, portal);

      const response = await page.request.get(target, { maxRedirects: 0 });
      expect(response.status()).toBe(404);
      await expect(page.locator(`a[href="${target}"]`)).toHaveCount(0);
    });
  });
}

test.describe('with login disabled', () => {
  test.use({ disabledFlag: loginFlag });

  test('the signed-out homepage omits the creation link', async ({ page }, info) => {
    const lang = info.project.metadata.locale;
    const target = `/${lang}/account/characters/new`;
    expect((await page.goto(`/${lang}/`)).status()).toBe(200);
    const creation = await page.request.get(target, { maxRedirects: 0 });
    expect(creation.status()).toBe(303);
    expect(creation.headers().location).toBe(`/${lang}/login`);
    expect((await page.request.get(`/${lang}/login`, { maxRedirects: 0 })).status()).toBe(404);
    await expect(page.locator(`a[href="${target}"]`)).toHaveCount(0);
    await expect(page.locator('#jak-zaczac').getByText(lang === 'pl'
      ? 'Tworzenie postaci jest obecnie niedostępne.'
      : 'Character creation is currently unavailable.', { exact: true })).toBeVisible();
  });
});
