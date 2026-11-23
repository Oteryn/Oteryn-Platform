import {randomUUID} from 'node:crypto';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {expect, test} from '../fixtures.mjs';
import {csrfRejected, login, password} from '../helpers.mjs';
import {query} from '../staff-rbac-helpers.mjs';

function seed(locale) {
  const account = {email: `overview-${randomUUID()}@example.invalid`, password: password(), locale};
  const binary = process.env.PORTAL_E2E_BINARY
    || path.resolve(process.env.CARGO_TARGET_DIR || '../target', 'debug/oteryn-portal');
  const result = spawnSync(binary, ['seed', 'account', JSON.stringify(account)], {encoding: 'utf8'});
  expect(result.status, 'disposable account seed succeeded').toBe(0);
  return {...account, id: JSON.parse(result.stdout).accountId};
}

async function publish(account, world) {
  const [{id}] = await query('SELECT uuidv7() AS id');
  const response = await fetch(`${process.env.PORTAL_E2E_GAME_CLOSURE_CONTROL_URL}/portfolio`, {
    method: 'POST',
    headers: {'content-type': 'application/json'},
    body: JSON.stringify({account_id: account.id, character_id: id, world_id: world}),
  });
  expect(response.status).toBe(200);
  return (await response.json()).name;
}

test('account owner reads a verified roster and saves private display preferences', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const owner = seed(lang);
  const other = seed(lang);
  const [world] = await query("SELECT id FROM world_status WHERE name='Local Bazaar World'");
  const ownedName = await publish(owner, world.id);
  const foreignName = await publish(other, world.id);

  await login(page, lang, owner);
  expect((await page.goto(`/${lang}/`)).status()).toBe(200);
  if (info.project.use.viewport.width <= 960) {
    await page.locator('#portal-menu-toggle').click();
  }
  await page.locator('[data-account-link]').click();
  const menu = page.getByRole('dialog');
  await expect(menu.locator(`a[href="/${lang}/account/display"]`)).toBeVisible();
  await menu.locator('a[href="/account"]').click();

  const overview = page.locator('[data-account-overview]');
  await expect(overview.locator('[data-roster-verified]')).toHaveText(/\d{4}-\d\d-\d\d \d\d:\d\d:\d\d UTC/);
  await expect(overview.getByText(ownedName, {exact: true})).toBeVisible();
  await expect(overview.getByText(foreignName, {exact: true})).toHaveCount(0);
  await expect(overview.getByText('10 / 20', {exact: true})).toBeVisible();
  const websiteLogin = lang === 'pl' ? 'Ostatnie logowanie na stronie' : 'Last website login';
  await expect(overview.locator('dt', {hasText: websiteLogin}).locator('xpath=following-sibling::dd[1]'))
    .toHaveText(/\d{4}-\d\d-\d\d \d\d:\d\d:\d\d UTC/);
  const profession = lang === 'pl' ? 'Profesja' : 'Profession';
  await expect(overview.locator('dt', {hasText: profession}).locator('xpath=following-sibling::dd[1]'))
    .toHaveText(lang === 'pl' ? 'Niedostępne' : 'Unavailable');

  const target = `/${lang}/account/display`;
  await overview.locator(`a[href="${target}"]`).click();
  const form = page.locator(`main form[action="${target}"]`);
  const csrf = await form.locator('input[name=_csrf]').inputValue();
  await csrfRejected(page, target, {display_name: 'Alias', locale: lang});
  const refused = await page.request.post(target, {
    form: {_csrf: csrf, display_name: 'x'.repeat(81), locale: lang},
    headers: {origin: new URL(page.url()).origin},
    maxRedirects: 0,
  });
  expect(refused.status()).toBe(422);
  const preferred = lang === 'pl' ? 'en' : 'pl';
  await form.locator('[name=display_name]').fill('<b>Żółty smok</b>');
  await form.locator('[name=locale]').selectOption(preferred);
  await form.locator('button[type=submit]').click();
  await expect(page.locator('main [role=status]')).toHaveText(
    lang === 'pl' ? 'Zapisano ustawienia wyświetlania.' : 'Display settings saved.',
  );
  await expect(form.locator('[name=display_name]')).toHaveValue('<b>Żółty smok</b>');
  await expect(form.locator('[name=locale]')).toHaveValue(preferred);
  await expect(page.locator('main b')).toHaveCount(0);
  const [row] = await query('SELECT display_name, locale FROM accounts WHERE id=$1', [owner.id]);
  expect(row).toEqual({display_name: '<b>Żółty smok</b>', locale: preferred});
  if (info.project.use.viewport.width === 390) {
    expect(await form.evaluate(element => element.scrollWidth <= element.clientWidth)).toBe(true);
  }
});
