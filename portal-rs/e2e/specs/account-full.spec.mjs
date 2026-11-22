import {createHmac} from 'node:crypto';
import {readFile} from 'node:fs/promises';
import {test, expect} from '../fixtures.mjs';
import {csrfRejected, exportOptions, pool, submit} from '../helpers.mjs';
import {post, privatePage, refusesAnonymous, seedPlayer, signIn, strings, sudo} from '../account-helpers.mjs';

const identity = 'crates/identity/src';
const lifecycle = lang => `${identity}/account_lifecycle_${lang}.ftl`;
const preferences = ['news_email', 'news_portal', 'support_email', 'support_portal'];

test('notification channels persist independently and reject invalid updates', async ({page}, info) => {
  const {locale: lang} = info.project.metadata;
  const target = `/${lang}/account/notifications`;
  await refusesAnonymous(page, target, 303, `/${lang}/login`);

  await signIn(page, lang, seedPlayer(lang));
  await privatePage(page, target);
  const main = page.locator('main');
  await strings(main, lifecycle(lang), ['preferences', 'security', 'news', 'support', 'email', 'portal', 'save']);
  const checkboxes = preferences.map(name => main.locator(`[name=${name}]`));
  await expect(checkboxes[0]).not.toBeChecked();
  await expect(checkboxes[1]).toBeChecked();
  await expect(checkboxes[2]).not.toBeChecked();
  await expect(checkboxes[3]).toBeChecked();

  for (const combination of [[true, false, false, true], [false, true, true, false]]) {
    await submit(page, 'main form', Object.fromEntries(preferences.map((name, i) => [name, combination[i]])));
    await privatePage(page, target);
    for (const [i, checkbox] of checkboxes.entries()) {
      await expect(checkbox).toBeChecked({checked: combination[i]});
    }
  }

  expect((await post(page, target, {news_email: 'invalid'})).status()).toBe(422);
  await privatePage(page, target);
  await expect(checkboxes[0]).not.toBeChecked();
  await expect(checkboxes[2]).toBeChecked();
});

test('privacy controls persist both visibility flags and change the account language', async ({page}, info) => {
  const {locale: lang} = info.project.metadata;
  const other = lang === 'pl' ? 'en' : 'pl';
  const target = `/${lang}/account/privacy`;
  await refusesAnonymous(page, target, 303, `/${lang}/login`);
  await signIn(page, lang, seedPlayer(lang));
  await privatePage(page, target);
  const main = page.locator('main');
  const form = main.locator('form');
  const catalog = `${identity}/privacy_${lang}.ftl`;
  await strings(main, catalog, ['title', 'association', 'status', 'language', 'save']);
  await expect(form.locator('[name=locale] option')).toHaveText(['Polski', 'English']);

  const invalid = await post(page, target, {locale: 'unknown', public_account_association: '1'});
  expect(invalid.status()).toBe(422);
  expect(await invalid.text()).toContain(lang === 'pl'
    ? 'Sprawdź ustawienia prywatności i wybierz język polski lub angielski.'
    : 'Check the privacy settings and select Polish or English.');

  await form.locator('[name=locale]').selectOption(other);
  await submit(page, 'main form', {public_account_association: true, public_status_visible: true});
  await expect(page).toHaveURL(`/${other}/account/privacy`);
  await expect(form.locator('[name=locale]')).toHaveValue(other);
  await expect(form.locator('[name=public_account_association]')).toBeChecked();
  await expect(form.locator('[name=public_status_visible]')).toBeChecked();

  await form.locator('[name=locale]').selectOption(lang);
  await submit(page, 'main form', {public_account_association: false, public_status_visible: false});
  await expect(page).toHaveURL(target);
  await expect(form.locator('[name=public_account_association]')).not.toBeChecked();
  await expect(form.locator('[name=public_status_visible]')).not.toBeChecked();
});

test('each optional consent can be granted and revoked independently', async ({page}, info) => {
  const {locale: lang} = info.project.metadata;
  const target = `/${lang}/account/consents`;
  const anonymous = await page.request.get(target, {maxRedirects: 0});
  expect(anonymous.status()).toBe(303);
  expect(anonymous.headers().location).toBe(`/${lang}/login`);
  await signIn(page, lang, seedPlayer(lang));
  await privatePage(page, target);

  const main = page.locator('main');
  const catalog = `${identity}/consents_${lang}.ftl`;
  await strings(main, catalog, ['title', 'hint', 'save', 'refused', 'no-decision']);
  const play = main.locator('form').filter({has: page.locator('#play_session')});
  const ads = main.locator('form').filter({has: page.locator('#personalised_ads')});
  await strings(play, catalog, ['play_session', 'play_session-purpose']);
  await strings(ads, catalog, ['personalised_ads', 'personalised_ads-purpose']);

  const form = await play.evaluate(element => Object.fromEntries(new FormData(element)));
  expect((await post(page, target, {...form, kind: 'unknown'}, play)).status()).toBe(400);
  const outdated = await post(page, target, {...form, version: 'outdated', granted: 'true'}, play);
  expect(outdated.status()).toBe(409);
  expect(await outdated.text()).toContain(lang === 'pl'
    ? 'Treść zgody uległa zmianie.'
    : 'The consent text has changed.');

  await submit(page, 'main form:has(#play_session)', {granted: true});
  await expect(play.getByRole('checkbox')).toBeChecked();
  await expect(ads.getByRole('checkbox')).not.toBeChecked();
  await strings(play, catalog, ['granted', 'decided']);
  await expect(play.locator('time')).toBeVisible();

  await submit(page, 'main form:has(#personalised_ads)', {granted: true});
  await expect(ads.getByRole('checkbox')).toBeChecked();
  await expect(play.getByRole('checkbox')).toBeChecked();
  await submit(page, 'main form:has(#play_session)', {granted: false});
  await expect(play.getByRole('checkbox')).not.toBeChecked();
  await expect(ads.getByRole('checkbox')).toBeChecked();
  await submit(page, 'main form:has(#personalised_ads)', {granted: false});
  await expect(ads.getByRole('checkbox')).not.toBeChecked();
  await strings(ads, catalog, ['refused', 'decided']);
});

test('sudo unlocks the owner JSON download with an explicit public field set', async ({page}, info) => {
  const {locale: lang} = info.project.metadata;
  const target = `/${lang}/account/privacy/export`;
  const owner = seedPlayer(lang);
  const other = seedPlayer(lang);
  await refusesAnonymous(page, target, 303, `/${lang}/login`);
  await signIn(page, lang, owner);
  const beforeSudo = await page.request.get(target, {maxRedirects: 0});
  expect(beforeSudo.status()).toBe(303);
  expect(beforeSudo.headers().location).toBe(`/${lang}/account/sudo?return_to=export`);

  await page.goto(`/${lang}/account/notifications`);
  await submit(page, 'main form', {news_email: true, news_portal: false});
  await privatePage(page, `/${lang}/account/consents`);
  await submit(page, 'main form:has(#play_session)', {granted: true});
  const db = pool();
  let fixture;
  try {
    const {rows} = await db.query(
      "INSERT INTO notifications(account_id,category,subject,body) VALUES($1,'news','Local export fixture','Synthetic notice'),($2,'news','Foreign export fixture','Foreign notice') RETURNING account_id, jsonb_build_object('id',id,'category',category,'subject',subject,'body',body,'read_at',read_at,'created_at',created_at) AS exported",
      [owner.id, other.id],
    );
    fixture = rows.find(row => row.account_id === owner.id).exported;
  } finally {
    await db.end();
  }
  await sudo(page, lang, owner);
  const response = await page.request.post(target, await exportOptions(page, lang));
  expect(response.status()).toBe(200);
  expect(response.headers()).toMatchObject({
    'content-disposition': 'attachment; filename="account-data.json"',
    'content-type': 'application/json',
    'cache-control': 'no-store, private',
    'x-content-type-options': 'nosniff',
  });

  const confirmation = await page.goto(`${target}?account_id=${other.id}`);
  expect(confirmation.status()).toBe(200);
  expect(confirmation.headers()['content-disposition']).toBeUndefined();
  const [download] = await Promise.all([
    page.waitForEvent('download'),
    page.locator('main form[method=post] button[type=submit]').click(),
  ]);
  expect(download.suggestedFilename()).toBe('account-data.json');
  expect(await download.failure()).toBeNull();
  const data = JSON.parse(await readFile(await download.path(), 'utf8'));
  expect(Object.keys(data).sort()).toEqual([
    'account', 'consents', 'notification_preferences', 'notifications', 'schema_version', 'sessions',
  ]);
  expect(data.schema_version).toBe(1);
  expect(data.account).toMatchObject({id: owner.id, email: owner.email, locale: lang, status: 'active'});
  expect(Object.keys(data.account).sort()).toEqual([
    'created_at', 'display_name', 'email', 'email_verified_at', 'id', 'locale', 'public_account_association',
    'public_status_visible', 'status', 'updated_at',
  ]);
  expect(data.notification_preferences).toContainEqual({category: 'news', email_enabled: true, portal_enabled: false});
  expect(data.consents).toContainEqual(expect.objectContaining({document_kind: 'play_session', granted: true}));
  expect(data.notifications).toContainEqual(fixture);
  expect(data.notifications.map(item => item.subject)).not.toContain('Foreign export fixture');
  expect(data.sessions).toHaveLength(1);
  expect(Object.keys(data.sessions[0]).sort()).toEqual(['created_at', 'device', 'expires_at', 'ip', 'last_seen_at']);
  expect(JSON.stringify(data)).not.toContain(other.email);
});

test('account closure requires TERMINATE, revokes the session, and can be cancelled in grace', async ({page}, info) => {
  test.setTimeout(240_000);
  const {locale: lang} = info.project.metadata;
  const target = `/${lang}/account/privacy/delete`;
  const player = seedPlayer(lang);
  await refusesAnonymous(page, target, 303, `/${lang}/login`);
  await signIn(page, lang, player);
  await sudo(page, lang, player);
  await privatePage(page, target);
  const main = page.locator('main');
  await strings(main, lifecycle(lang), ['delete', 'explanation', 'password', 'confirmation']);

  const invalid = await post(page, target, {current_password: player.password, confirmation: 'DELETE'});
  expect(invalid.status()).toBe(422);
  expect((await post(page, target, {current_password: 'Incorrect local password', confirmation: 'TERMINATE'})).status())
    .toBe(403);
  await submit(page, 'main form', {current_password: player.password, confirmation: 'TERMINATE'});
  await expect(page).toHaveURL(`/${lang}/login`);
  await refusesAnonymous(page, target, 303, `/${lang}/login`);
  const db = pool();
  try {
    const termination = async () => (await db.query(
      'SELECT scheduled_for > now() AS in_grace, cancelled_at, finalized_at FROM account_terminations WHERE account_id=$1',
      [player.id],
    )).rows[0];
    expect(await termination()).toEqual({in_grace: true, cancelled_at: null, finalized_at: null});
    await signIn(page, lang, player);
    await sudo(page, lang, player);
    await privatePage(page, target);
    await strings(main, lifecycle(lang), ['pending', 'password', 'cancel']);
    await submit(page, 'main form', {current_password: player.password});
    await expect(page).toHaveURL(target);
    await strings(main, lifecycle(lang), ['explanation', 'confirmation']);
    expect(await termination()).toMatchObject({cancelled_at: expect.any(Date), finalized_at: null});
  } finally {
    await db.end();
  }
});

test('signed unsubscribe confirmation changes only the addressed optional email channel', async ({page, browser}, info) => {
  const {locale: lang} = info.project.metadata;
  const player = seedPlayer(lang);
  await signIn(page, lang, player);
  const preferencesPage = `/${lang}/account/notifications`;
  await privatePage(page, preferencesPage);
  await submit(page, 'main form', {news_email: true, support_email: true});

  // Match the local notification sender's signed link format.
  const payload = `unsubscribe:${player.id}:news:${Math.floor(Date.now() / 1000) + 3600}`;
  const signature = createHmac('sha256', process.env.OTERYN_NOTIFY_UNSUBSCRIBE_KEY)
    .update(payload).digest('hex');
  const target = `/${lang}/unsubscribe`;
  const link = `${target}?token=${encodeURIComponent(`${payload}:${signature}`)}`;
  const visitor = await browser.newContext({
    baseURL: new URL(page.url()).origin,
    ignoreHTTPSErrors: true,
    viewport: info.project.use.viewport,
  });
  try {
    const anonymous = await visitor.newPage();
    expect((await anonymous.request.get(`${target}?token=${'a'.repeat(513)}`)).status()).toBe(422);
    expect((await anonymous.request.post(`${target}?token=invalid`)).status()).toBe(422);
    await privatePage(anonymous, link);
    const main = anonymous.locator('main');
    await strings(main, lifecycle(lang), ['unsubscribe', 'unsubscribe-info']);
    await csrfRejected(anonymous, link);
    await main.getByRole('button').click();
    await strings(main, lifecycle(lang), ['unsubscribed']);
    const replay = await anonymous.request.post(link, {form: {'List-Unsubscribe': 'One-Click'}});
    expect(replay.status()).toBe(200);
  } finally {
    await visitor.close();
  }

  await privatePage(page, preferencesPage);
  await expect(page.locator('main [name=news_email]')).not.toBeChecked();
  await expect(page.locator('main [name=news_portal]')).toBeChecked();
  await expect(page.locator('main [name=support_email]')).toBeChecked();
  await expect(page.locator('main [name=support_portal]')).toBeChecked();
});
