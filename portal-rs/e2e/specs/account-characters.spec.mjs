import {test, expect} from '../fixtures.mjs';
import {csrfRejected, pool, submit} from '../helpers.mjs';
import {post, privatePage, refusesAnonymous, seedPlayer, signIn, strings, switchPlayer, sudo} from '../account-helpers.mjs';

test('character creation filters worlds, suggests names, validates input, and protects request status', async ({page}, info) => {
  test.setTimeout(240_000);
  const {locale: lang} = info.project.metadata;
  const root = `/${lang}/account/characters`;
  const target = `${root}/new`;
  const catalog = `crates/characters/src/create_web/${lang}.ftl`;
  await refusesAnonymous(page, target, 303, `/${lang}/login`);
  await refusesAnonymous(page, `${root}/suggest`, 303, `/${lang}/login`);
  const player = seedPlayer(lang);
  const db = pool();
  try {
    const region = `Qualification ${player.id}`;
    const {rows: [world]} = await db.query(
      "INSERT INTO world_status(id,name,location) VALUES(uuidv7(),'Qualification World',$1) RETURNING id",
      [region],
    );
    await signIn(page, lang, player);
    await privatePage(page, target);
    const main = page.locator('main');
    await strings(main, catalog, [
      'title', 'rules', 'reserved-label', 'supported', 'pvp', 'location', 'any', 'apply',
      'name', 'suggest', 'suggestion-note', 'world', 'choose', 'sex', 'sex-female', 'sex-male',
      'vocation', 'vocation-knight', 'vocation-paladin', 'vocation-sorcerer', 'vocation-druid',
      'vocation-monk', 'submit',
    ]);
    await main.locator('summary').click();
    await strings(main.locator('details'), catalog, ['reserved-words']);
    await expect(main.locator('details p')).toBeVisible();
    await main.locator('summary').click();

    const filter = main.locator('form[method=get]');
    const creation = main.locator('form[method=post]');
    await filter.locator('[name=location]').selectOption({label: region});
    await filter.getByRole('button').click();
    await expect(creation.locator('[name=world] option')).toHaveCount(2);
    await expect(creation.locator('[name=world] option').last()).toHaveAttribute('value', world.id);
    await privatePage(page, `${target}?location=Unknown`, 400);
    await expect(main.locator('h1')).toHaveText(lang === 'pl' ? 'Nieprawidłowe żądanie' : 'Invalid request');
    await privatePage(page, target);

    await main.locator(`a[href="${root}/suggest"]`).click();
    await expect(page).toHaveURL(`${root}/suggest`);
    await expect(creation.locator('[name=name]')).toHaveValue(/^[a-zA-Z ]{3,29}$/);
    await strings(main.locator('[role=status]'), catalog, ['suggestion-note']);
    await creation.locator('[name=world]').selectOption(world.id);
    await creation.locator('[name=sex]').first().check();
    await creation.locator('[name=vocation]').first().check();
    await submit(page, 'main form[method=post]', {name: 'Admin'});
    await expect(page).toHaveURL(target);
    await strings(main.locator('[role=status]'), catalog, ['reserved']);
    await expect(creation.locator('[name=name]')).toHaveValue('Admin');

    await submit(page, 'main form[method=post]', {name: 'Browser Adventurer'});
    await expect(page).toHaveURL(/\/account\/characters\/requests\/[a-f0-9-]+$/);
    const request = new URL(page.url()).pathname;
    await strings(main, catalog, ['pending', 'refresh']);
    await main.locator('a[href=""]').click();
    await strings(main.locator('[role=status]'), catalog, ['pending']);
    const operation = request.split('/').at(-1);
    expect((await db.query('SELECT account_id,state,receipt_json FROM character_commands WHERE id=$1', [operation])).rows)
      .toEqual([{account_id: player.id, state: 'PENDING', receipt_json: null}]);
    await switchPlayer(page, lang, seedPlayer(lang));
    await privatePage(page, request, 404);
    await expect(main.locator('h1')).toHaveText(lang === 'pl' ? 'Nie znaleziono strony' : 'Page not found');
    await privatePage(page, `${root}/requests/invalid`, 404);
  } finally {
    await db.end();
  }
});

test('account overview opens owner-only profile preferences and persists every control', async ({page}, info) => {
  test.setTimeout(240_000);
  const {locale: lang} = info.project.metadata;
  const player = seedPlayer(lang);
  const foreign = seedPlayer(lang);
  const anonymous = await page.request.get('/account', {
    headers: {'accept-language': lang},
    maxRedirects: 0,
  });
  expect(anonymous.status()).toBe(303);
  expect(anonymous.headers().location).toBe(`/${lang}/login`);
  const db = pool();
  let ids;
  try {
    ({rows: [ids]} = await db.query(
      'SELECT uuidv7() AS asset, uuidv7() AS seller, uuidv7() AS character',
    ));
  } finally {
    await db.end();
  }
  const fixture = await page.request.post('/__bazaar_game/seed', {data: {
    kind: 'item', seller: foreign.id, buyer: player.id, asset_id: ids.asset,
    seller_character_id: ids.seller, buyer_character_id: ids.character,
  }});
  expect(fixture.status()).toBe(201);
  const name = `Fixture ${ids.character.slice(-12)}`;
  const target = `/${lang}/account/characters/${encodeURIComponent(name)}/profile`;
  await refusesAnonymous(page, target, 303, `/${lang}/login`);
  await signIn(page, lang, player);
  await privatePage(page, '/account');
  const main = page.locator('main');
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await strings(main, `templates/account/overview.${lang}.ftl`, [
    'overview', 'created', 'status', 'active', 'characters', 'world', 'level', 'profession',
    'unavailable', 'linked', 'security', 'premium', 'balance', 'loyalty', 'title', 'limit',
    'web-login', 'game-login', 'termination', 'availability', 'available', 'profile-preferences',
  ]);
  await expect(main.getByRole('link', {name, exact: true})).toHaveAttribute(
    'href', `/${lang}/characters/${encodeURIComponent(name)}`,
  );
  await main.locator(`a[href="${target}"]`).click();
  await expect(page).toHaveURL(target);
  const catalog = `templates/profile/${lang}.ftl`;
  await strings(main, catalog, [
    'title', 'guild', 'main', 'save', 'hidden', 'hide-all', 'account', 'status', 'house',
    'skills', 'deaths', 'kills', 'online-time', 'comment',
  ]);
  await expect(main.locator('[name=hidden]')).toBeChecked();
  const flags = ['show_guild', 'is_main_character', 'hide_all_characters', 'show_account', 'show_status',
    'show_house', 'show_skills', 'show_deaths', 'show_kills', 'show_online_time'];
  for (const flag of flags) await expect(main.locator(`[name=${flag}]`)).not.toBeChecked();

  const bad = await post(page, target, {comment: 'x'.repeat(501)});
  expect(bad.status()).toBe(422);
  expect(await bad.text()).toContain(lang === 'pl' ? 'Sprawdź wprowadzone dane' : 'Please check your input');
  const comment = '<script>Local profile comment</script>';
  await submit(page, 'main form', {
    hidden: false, comment, ...Object.fromEntries(flags.map(flag => [flag, true])),
  });
  await expect(main.locator('[name=comment]')).toHaveValue(comment);
  for (const flag of flags) await expect(main.locator(`[name=${flag}]`)).toBeChecked();
  await expect(main.locator('[name=hidden]')).not.toBeChecked();
  await privatePage(page, '/account');
  await strings(main.locator('[data-main-character]'), `templates/account/overview.${lang}.ftl`, ['main-character']);
  const unavailableProfile = page.waitForResponse(response =>
    response.request().isNavigationRequest()
    && new URL(response.url()).pathname === `/${lang}/characters/${encodeURIComponent(name)}`);
  await main.getByRole('link', {name, exact: true}).click();
  expect((await unavailableProfile).status()).toBe(503);
  await strings(main, `templates/profiles/${lang}.ftl`, ['unavailable']);

  for (const [link, destination] of [
    ['/account/security', `/${lang}/password/change`],
    [`/${lang}/panel/premium`, `/${lang}/panel/premium`],
    [`/${lang}/shop/history`, `/${lang}/shop/history`],
  ]) {
    await privatePage(page, '/account');
    const response = page.waitForResponse(r => r.request().isNavigationRequest() && new URL(r.url()).pathname === destination);
    await main.locator(`a[href="${link}"]`).click();
    expect((await response).status()).toBe(200);
  }

  await switchPlayer(page, lang, foreign);
  await privatePage(page, `/${lang}/account/notifications`);
  expect((await post(page, target, {comment: 'Foreign change'})).status()).toBe(404);
  await privatePage(page, target, 404);
});

test('public character search validates names and follows the localized result route', async ({page}, info) => {
  const {locale: lang} = info.project.metadata;
  const target = `/${lang}/characters`;
  const catalog = `templates/profiles/${lang}.ftl`;
  await privatePage(page, target);
  const main = page.locator('main');
  await strings(main, catalog, ['title', 'name', 'search']);
  const form = main.locator('form');
  const name = form.locator('[name=name]');
  await name.fill('invalid/name');
  const invalid = page.waitForResponse(r => r.request().isNavigationRequest() && r.url().includes('name=invalid%2Fname'));
  await form.getByRole('button').click();
  expect((await invalid).status()).toBe(400);
  await strings(main, catalog, ['invalid']);

  await name.fill('Unpublished Character');
  const unavailable = page.waitForResponse(r => r.request().isNavigationRequest() && r.url().endsWith('/Unpublished%20Character'));
  await form.getByRole('button').click();
  expect((await unavailable).status()).toBe(503);
  await expect(page).toHaveURL(`${target}/Unpublished%20Character`);
  await strings(main, catalog, ['unavailable']);
});

test('character services validate form actions and refuse without their Game authority', async ({page}, info) => {
  const {locale: lang} = info.project.metadata;
  const character = '019a541b-a000-7000-8000-000000000001';
  const request = '019a541b-a000-7000-8000-000000000002';
  const target = `/${lang}/account/characters/${character}/services`;
  await refusesAnonymous(page, target, 303, `/${lang}/login`);
  const player = seedPlayer(lang);
  await signIn(page, lang, player);
  await privatePage(page, target, 503);
  await expect(page.locator('main h1')).toHaveText(lang === 'pl'
    ? 'Usługa chwilowo niedostępna' : 'Temporarily unavailable');

  const tokenPage = `/${lang}/account/notifications`;
  await privatePage(page, tokenPage);
  const schedule = {action: 'schedule', confirm: 'yes', request_id: request};
  await csrfRejected(page, target, schedule);
  expect((await post(page, target, schedule)).status()).toBe(403);

  await sudo(page, lang, player);
  await privatePage(page, tokenPage);
  for (const fields of [
    schedule,
    {...schedule, action: 'cancel', deletion_operation_id: request},
    {...schedule, action: 'rename', accepted_price: '0', requested_name: 'Local Hero'},
  ]) {
    expect((await post(page, target, {...fields, confirm: 'no'})).status()).toBe(400);
    expect((await post(page, target, fields)).status()).toBe(503);
  }
});
