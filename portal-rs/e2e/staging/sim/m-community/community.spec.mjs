import {
  expect, mailAvailability, newPlayer, origin, status, submit, test, userRequest, verifyPlayer,
} from './helpers.mjs';

const missing = 'Sim Missing';
const name = encodeURIComponent(missing);
const uuid = '01900000-0000-7000-8000-000000000001';
const labels = {
  en: {
    missing: 'Page not found',
    login: 'Sign in',
    inbox: 'Check your inbox for an e-mail with the next steps.',
    unavailable: 'Registration is currently unavailable. Please try again later.',
    registerError: 'Passwords must match.',
    workError: 'Verification failed or expired.',
    todayUnavailable: 'World daily data is currently unavailable.',
    league: 'World events and guild seasons',
    leagueUnavailable: 'Authoritative results are currently unavailable.',
    pvpError: 'This operation could not be completed. Check the input and current state, then try again.',
    badges: 'Profile badges and achievements',
    puzzles: 'Daily puzzles',
    puzzlesUnavailable: "Today's puzzles are unavailable.",
    collectionUnavailable: 'Collection is unavailable. Current private progress and ownership must be verified.',
    pvpUnavailable: 'Collection battles and purchases are currently unavailable.',
  },
  pl: {
    missing: 'Nie znaleziono strony',
    login: 'Zaloguj się',
    inbox: 'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.',
    unavailable: 'Rejestracja jest obecnie niedostępna. Spróbuj ponownie później.',
    registerError: 'Hasła muszą być takie same.',
    workError: 'Weryfikacja nie powiodła się lub wygasła.',
    todayUnavailable: 'Dzienne dane świata są teraz niedostępne.',
    league: 'Wydarzenia świata i sezony gildii',
    leagueUnavailable: 'Autorytatywne wyniki są obecnie niedostępne.',
    pvpError: 'Nie udało się wykonać operacji. Sprawdź dane i aktualny stan, a następnie spróbuj ponownie.',
    badges: 'Odznaki i osiągnięcia profilu',
    puzzles: 'Codzienne zagadki',
    puzzlesUnavailable: 'Dzisiejsze zagadki są niedostępne.',
    collectionUnavailable: 'Kolekcja jest niedostępna. Wymagana jest weryfikacja aktualnego prywatnego postępu i właściciela postaci.',
    pvpUnavailable: 'Walki kolekcji i zakupy są obecnie niedostępne.',
  },
};

async function assertStatus(client, method, path, code, options) {
  const response = await status(client, method, path, code, options);
  await response.dispose();
}

async function assertLogin(client, method, path, lang, options) {
  const response = await status(client, method, path, 303, options);
  expect.soft(response.headers().location, `${method} ${path} sign-in destination`).toBe(`/${lang}/login`);
  await response.dispose();
}

async function authorizedForm(page, fields, selector = 'main form') {
  return {
    form: { _csrf: await page.locator(`${selector} [name="_csrf"]`).first().inputValue(), ...fields },
    headers: { origin },
  };
}

test('community navigation and unavailable public projections', async ({ page, request }, info) => {
  const { lang } = info.project.metadata;
  const root = `/${lang}`;
  const other = lang === 'en' ? 'pl' : 'en';
  expect((await page.goto(`${root}/`)).status()).toBe(200);
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  const menu = page.locator('#portal-menu-toggle');
  if (await menu.isVisible()) await menu.click();

  for (const route of ['worlds', 'online', 'rankings/experience', 'guilds', 'houses', 'deaths', 'party']) {
    await expect.soft(page.locator(`header a[href="${root}/${route}"]`)).toHaveCount(0);
  }
  const language = page.locator(`footer a[hreflang="${other}"][href="/${other}/"]`);
  await expect(language).toHaveAttribute('href', `/${other}/`);
  await language.click();
  await expect(page).toHaveURL(`${origin}/${other}/`);
  await expect(page.locator('html')).toHaveAttribute('lang', other);
  await page.goBack();
  await expect(page).toHaveURL(`${origin}${root}/`);

  const unavailable = await page.goto(`${root}/party`);
  expect(unavailable.status()).toBe(404);
  await expect(page.getByRole('heading', { name: labels[lang].missing })).toBeVisible();
  expect((await page.reload()).status()).toBe(404);
  await expect(page.getByRole('heading', { name: labels[lang].missing })).toBeVisible();

  for (const prefix of ['', root]) {
    for (const route of [
      '/worlds', '/online', '/guilds', `/guilds/${name}`, '/houses', `/houses/${name}`,
      '/deaths', `/characters/${name}/deaths`, `/characters/${name}/skills`,
    ]) {
      await assertStatus(request, 'GET', prefix + route, 404);
    }
    const data = await status(request, 'GET', `${prefix}/today/data`, 503);
    expect.soft(await data.json()).toEqual({ status: 'unavailable' });
    await data.dispose();
    const widget = await status(request, 'GET', `${prefix}/today/widget`, 503);
    expect.soft(await widget.text()).toContain(labels[prefix ? lang : 'pl'].todayUnavailable);
    await widget.dispose();
  }
  for (const path of [
    '/api/v1/worlds', `${root}/rankings/experience`, `${root}/rankings/invalid`,
    `${root}/characters`, `${root}/characters/${name}`,
  ]) {
    await assertStatus(request, 'GET', path, 404);
  }
});

test('party posting, reporting, removal and staff queue are unavailable', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const root = `/${lang}`;
  await page.goto(`${root}/login`);
  const fields = await authorizedForm(page, {});
  const staff = `/admin${root}/party`;
  await assertStatus(page.request, 'GET', staff, 404);
  const starts = Math.floor(Date.now() / 1000) + 1800;
  for (const [path, form] of [
    [`${root}/party`, { character: uuid, purpose: 'hunt', min_level: 8, max_level: 20,
      profession: 'any', starts_at: starts, expires_at: starts + 3600, note: 'Staging QA' }],
    [`${root}/party/${uuid}/report`, { reason: 'Staging QA' }],
    [`${root}/party/${uuid}/remove`, {}],
    [`${staff}/${uuid}/moderate`, { report_ids: uuid, action: 'dismiss', reason: 'Staging QA' }],
  ]) {
    await assertStatus(page.request, 'POST', path, 404, { ...fields, form: { ...fields.form, ...form } });
    await assertStatus(page.request, 'POST', path, 404, { headers: { origin }, form });
  }
  for (const [method, path] of [
    ['POST', `${root}/party/invalid/report`], ['POST', `${root}/party/invalid/remove`],
    ['POST', `${staff}/invalid/moderate`],
  ]) {
    await assertStatus(page.request, method, path, 404, fields);
  }
});

test('account routes direct visitors to sign in', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const root = `/${lang}`;
  const referral = `${root}/account/referrals`;
  for (const prefix of ['', root]) {
    const routeLang = prefix ? lang : 'pl';
    for (const route of [
      '/account/badges', ...['achievements', 'equipment', 'outfit', 'profile']
        .map(tab => `/account/characters/${name}/${tab}`),
    ]) {
      await assertLogin(page.request, 'GET', prefix + route, routeLang);
    }
  }
  await page.goto(referral);
  await expect(page).toHaveURL(`${origin}${root}/login`);
  await expect(page.getByRole('heading', { name: labels[lang].login })).toBeVisible();
  const fields = await authorizedForm(page, {});
  // Anonymous GET is gated by the shell even when the feature handler is off.
  await assertStatus(page.request, 'POST', referral, 404, {
    ...fields, form: { ...fields.form, kind: 'link' },
  });
  for (const prefix of ['', root]) {
    const routeLang = prefix ? lang : 'pl';
    const options = { ...fields, headers: { ...fields.headers, 'accept-language': routeLang } };
    await assertLogin(page.request, 'POST', `${prefix}/account/badges`, routeLang, options);
    for (const method of ['POST', 'PUT']) {
      await assertLogin(page.request, method, `${prefix}/account/characters/${name}/profile`, routeLang, options);
    }
  }

  await assertStatus(page.request, 'GET', `${root}/referrals/${uuid}`, 404);
  await assertStatus(page.request, 'GET', `${root}/referrals/not-a-code`, 404);
  await assertStatus(page.request, 'POST', `${root}/referrals/${uuid}`, 404, fields);
  await assertStatus(page.request, 'POST', `${root}/referrals/not-a-code`, 404, fields);
});

test('minigame sign-in, unavailable league and disabled season pass', async ({ page, request }, info) => {
  const { lang } = info.project.metadata;
  const root = `/${lang}`;
  await page.goto(`${root}/`);
  const menu = page.locator('#portal-menu-toggle');
  if (await menu.isVisible()) await menu.click();
  const world = page.locator('#portal-world-button');
  await world.click();
  for (const path of ['/puzzles', '/collection', '/league']) {
    await expect.soft(page.locator(`header a[href="${root}${path}"]`)).toBeVisible();
  }
  await expect.soft(page.locator(`header a[href="${root}/pass"]`)).toHaveCount(0);
  await page.locator(`header a[href="${root}/puzzles"]`).click();
  await expect(page).toHaveURL(`${origin}${root}/login`);
  await expect(page.getByRole('heading', { name: labels[lang].login })).toBeVisible();

  for (const path of [
    '/puzzles', `/puzzles/${uuid}/image`, '/puzzles/invalid/image', '/collection',
    `/collection/${uuid}`, `/collection/${uuid}/cards/${uuid}/image`,
    `/collection/battles/${uuid}`, '/collection-pvp', '/pass',
  ]) {
    await assertLogin(request, 'GET', root + path, lang);
  }
  const fields = await authorizedForm(page, {});
  for (const [path, form] of [
    [`/puzzles/${uuid}/answer`, { choice: 0 }],
    ['/puzzles/invalid/answer', { choice: 0 }],
    [`/collection/${uuid}/decks`, { name: 'Staging QA', first: uuid }],
    [`/collection/decks/${uuid}/battle`, {}],
    [`/collection/battles/${uuid}`, { expected: 0, action: 'strike' }],
    [`/collection/battles/${uuid}/forfeit`, {}],
    [`/collection/decks/${uuid}/delete`, {}],
  ]) {
    await assertLogin(page.request, 'POST', root + path, lang, {
      ...fields, form: { ...fields.form, ...form },
    });
    await assertStatus(page.request, 'POST', root + path, 403, { headers: { origin }, form });
  }
  const pvp = await status(page.request, 'POST', `${root}/collection-pvp`, 403, {
    ...fields, form: { ...fields.form, op: 'challenge' },
  });
  expect.soft((await pvp.text()).includes(labels[lang].pvpError), 'Localized PvP refusal').toBe(true);
  await pvp.dispose();

  await assertStatus(page.request, 'POST', `${root}/pass/${uuid}/buy`, 404, {
    ...fields, form: { ...fields.form, operation: uuid },
  });
  await assertStatus(page.request, 'POST', `${root}/pass/${uuid}/claim`, 404, {
    ...fields, form: { ...fields.form, operation: uuid, reward: uuid },
  });
  const league = await page.goto(`${root}/league`);
  expect(league.status()).toBe(503);
  await expect(page.getByRole('heading', { name: labels[lang].league, exact: true })).toBeVisible();
  await expect(page.locator('main')).toContainText(labels[lang].leagueUnavailable);
  await expect(page.locator('main table tbody tr')).toHaveCount(0);
});

test('downloads origin returns absent documents and rejects publication', async ({ request }, info) => {
  const { lang } = info.project.metadata;
  for (const suffix of ['/metadata/timestamp.json', '/policy-v1.json', '/targets/sim-missing.json']) {
    const response = await status(request, 'GET', `/updates/stable${suffix}`, 404);
    expect.soft(response.headers()['cache-control'], suffix).toBe('no-store');
    await response.dispose();
    await assertStatus(request, 'GET', `/updates/not-a-channel${suffix}`, 400);
    const refusal = await status(request, 'POST', `/updates/stable${suffix}`, 405);
    expect.soft(refusal.headers().allow).toBe('GET,HEAD');
    await refusal.dispose();
  }
  for (const suffix of ['', '/sim-missing']) {
    const legacy = await status(request, 'GET', `/download${suffix}`, 301);
    expect.soft(legacy.headers().location).toBe(`/pl/download${suffix}`);
    await legacy.dispose();
    await assertStatus(request, 'GET', `/${lang}/download${suffix}`, 404);
  }
});

test('fresh player signup and verified community entry points', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const root = `/${lang}`;
  const account = newPlayer();
  await page.goto(`${root}/`);
  const menu = page.locator('#portal-menu-toggle');
  if (await menu.isVisible()) await menu.click();
  await page.locator(`header a[href="${root}/login"]`).click();
  const [opened] = await Promise.all([
    page.waitForResponse(r => new URL(r.url()).pathname === `${root}/register` && r.request().method() === 'GET'),
    page.locator(`main a[href="${root}/register"]`).click(),
  ]);
  if (opened.status() === 503) {
    await expect(page.locator('main')).toContainText(labels[lang].unavailable);
    await page.screenshot({ path: info.outputPath('registration-form-unavailable.png') });
  }
  expect(opened.status(), 'open the registration form').toBe(200);

  const since = Date.now() - 5000;
  const response = await submit(page, `${root}/register`, {
    ...account, password_confirmation: account.password, terms: true, privacy: true,
  });
  if (response.status() === 503) {
    await expect(page.locator('main')).toContainText(labels[lang].unavailable);
    await page.screenshot({ path: info.outputPath('registration-unavailable.png') });
  } else if (response.status() === 422) {
    await expect(page.locator('main')).toContainText(labels[lang].workError);
    await page.screenshot({ path: info.outputPath('registration-verification-refused.png') });
  }
  const challenge = info.annotations.find(annotation => annotation.type === 'registration challenge status');
  expect(response.status(), `registration form status (challenge: ${challenge?.description})`).toBe(303);
  await expect(page).toHaveURL(`${origin}${root}/register/check-inbox`);
  await expect(page.getByRole('status')).toHaveText(labels[lang].inbox);
  await page.reload();
  await expect(page.getByRole('status')).toHaveText(labels[lang].inbox);

  const mail = await mailAvailability(page.request);
  test.skip([404, 405].includes(mail), 'Staging Mailpit unavailable for player verification');
  expect(mail).toBe(200);
  await verifyPlayer(page, account, since);
  await page.goto(`${root}/login`);
  await submit(page, `${root}/login`, account);
  await expect(page).toHaveURL(`${origin}${root}/`);

  const tab = await page.context().newPage();
  try {
    expect((await tab.goto(`${root}/account/badges`)).status()).toBe(200);
    await expect(tab.getByRole('heading', { name: labels[lang].badges })).toBeVisible();
    const age = tab.locator('[name=badge][value=account-age]');
    await expect(age).toBeEnabled();
    await age.check();
    await tab.locator('main form button[type=submit]').click();
    await expect(age).toBeChecked();
    expect((await tab.reload()).status()).toBe(200);
    await expect(age).toBeChecked();
    const malformed = await authorizedForm(tab, { profile: 'not-a-character-id' });
    await assertStatus(tab.request, 'POST', `${root}/account/badges`, 422, malformed);
    await assertStatus(tab.request, 'POST', `${root}/account/badges`, 403, {
      headers: { origin }, form: { badge: 'account-age' },
    });

    expect((await tab.goto(`${root}/puzzles`)).status()).toBe(200);
    await expect(tab.getByRole('heading', { name: labels[lang].puzzles })).toBeVisible();
    await expect(tab.locator('main')).toContainText(labels[lang].puzzlesUnavailable);
    await assertStatus(tab.request, 'GET', `${root}/puzzles/${uuid}/image`, 404);

    for (const [path, message] of [
      ['/collection', labels[lang].collectionUnavailable],
      ['/collection-pvp', labels[lang].pvpUnavailable],
    ]) {
      expect((await tab.goto(root + path)).status()).toBe(200);
      await expect(tab.locator('main')).toContainText(message);
    }
    for (const path of [
      '/account/referrals', '/pass',
      ...['achievements', 'equipment', 'outfit', 'profile'].map(
        segment => `/account/characters/${name}/${segment}`,
      ),
    ]) {
      const response = await tab.goto(root + path);
      expect.soft(response.status(), root + path).toBe(404);
      await expect.soft(tab).toHaveURL(origin + root + path);
    }
    await page.reload();
    await expect(page.locator('header [data-account-link]')).toHaveCount(1);
  } finally {
    await tab.close();
  }
});

test('registration keeps mismatched passwords on the form', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const path = `/${lang}/register`;
  const account = newPlayer();
  expect((await page.goto(path)).status()).toBe(200);
  const form = page.locator('main form');
  const options = await authorizedForm(page, {
    ...account,
    password_confirmation: `${account.password}a`,
    terms: await form.locator('[name=terms]').inputValue(),
    privacy: await form.locator('[name=privacy]').inputValue(),
  });
  // Field validation precedes the work challenge and can be checked directly.
  const response = await userRequest(page.request, path, { ...options, method: 'POST' });
  expect(response.status()).toBe(422);
  const html = await response.text();
  expect(html.includes(`<html lang="${lang}"`), 'Response language').toBe(true);
  expect(html.includes('id="confirmation-error"'), 'Confirmation field error').toBe(true);
  expect(html.includes(labels[lang].registerError), 'Localized mismatch explanation').toBe(true);
  expect(html.includes(account.password), 'Password field cleared').toBe(false);
  await response.dispose();
});

test('community and league producer routes are unavailable to browser actors', async ({ request }) => {
  const families = [
    ['v1', 503, ['highscores', 'character-profile', 'character-deaths', 'guild', 'house', 'world-online', 'watermark']],
    ['v1', 404, ['world-event-outcome', 'guild-season', 'league-watermark']],
    ['v2', 503, ['character-deaths', 'guild', 'house', 'world-daily', 'watermark']],
    ['v2', 404, ['character-skills', 'character-achievements', 'character-equipment', 'character-outfit']],
  ];
  for (const [version, code, names] of families) {
    for (const name of names) {
      await assertStatus(request, 'POST', `/internal/${version}/game-public/${name}`, code);
    }
  }
});
