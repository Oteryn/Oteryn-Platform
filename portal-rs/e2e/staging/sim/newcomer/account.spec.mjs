import {
  expect, get, login, messages, navigate, newcomer, origin, postForm, requireVerifiedPlayer,
  routeStagingAccess, submit, test, ui,
} from './helpers.mjs';

const loginText = lang => messages('templates/account/mfa/{lang}.ftl', lang);
const overviewText = lang => messages('templates/account/overview.{lang}.ftl', lang);
const privatePaths = [
  '/account', '/account/privacy', '/account/notifications', '/account/sessions',
  '/account/security/wizard', '/account/recovery-key', '/account/characters/new',
  '/support/tickets/create', '/support/reports', '/atlas/places', '/pass',
];

test('anonymous visitors are sent to sign in before opening private pages', async ({ page, request }, info) => {
  const { lang } = info.project.metadata;
  for (const path of privatePaths) {
    const response = await get(request, `/${lang}${path}`);
    expect(response.status(), path).toBe(303);
    expect(response.headers().location, path).toBe(`/${lang}/login`);
  }
  const response = await navigate(page, `/${lang}/account/privacy`);
  expect(response.status()).toBe(200);
  await expect(page).toHaveURL(`${origin}/${lang}/login`);
  await expect(page.locator('main h1')).toHaveText(loginText(lang)['mfa-login']);
  const form = page.locator('main form[action$="/login"]');
  await form.locator('[name=email]').fill('missing-at-sign');
  await form.locator('[name=password]').fill(newcomer().password);
  await form.getByRole('button', { name: loginText(lang)['mfa-login'] }).click();
  await expect(form.locator('[name=email]:invalid')).toBeFocused();

  const refusal = await login(page, lang, newcomer(), null);
  expect(refusal.status()).toBe(302);
  await expect(page).toHaveURL(`${origin}/${lang}/login`);
  await expect(page.locator('main [role=alert]')).toHaveText(loginText(lang)['mfa-error']);
  await page.goBack({ waitUntil: 'domcontentloaded' });
  await expect(page.locator('main h1')).toHaveText(loginText(lang)['mfa-login']);
});

async function checkOverview(page, lang) {
  const response = await navigate(page, '/account');
  expect(response.status(), 'Authenticated overview').toBe(200);
  const actualLanguage = await page.locator('html').getAttribute('lang');
  expect(['pl', 'en']).toContain(actualLanguage);
  await expect(page.locator('main h1')).toHaveText(overviewText(actualLanguage).overview);
  await expect(page.locator('main dl')).toContainText(overviewText(actualLanguage).active);
  await expect(page.locator(`footer a[hreflang=${lang}]`)).toBeVisible();
}

async function exploreAccount(page, lang) {
  const published = await page.locator('#podglad-konta a[href], header .om-fallback-account a[href], main nav a[href]')
    .evaluateAll(links => Array.from(new Set(links.map(link => link.getAttribute('href')))));
  expect(published).toContain(`/${lang}/account/privacy`);
  expect(published).toContain(`/${lang}/account/notifications`);
  const extras = [
    '/account/security/email', '/account/sudo', '/mfa', '/account/privacy/export',
    '/account/privacy/delete', '/account/consents',
  ].map(path => `/${lang}${path}`);
  // These surfaces have explicit no-data refusals when Game or commerce is off.
  const optional = /\/(?:panel|shop|pass|account\/(?:game-logins|journal|bazaar|hunts|characters\/new))(?:\/|$)/;
  for (const path of new Set([...published, ...extras])) {
    if (path === '/account') continue;
    await test.step(path, async () => {
      const response = await navigate(page, path);
      const heading = page.locator('main h1');
      if (optional.test(path) && response.status() === 503) {
        await expect(heading).toBeVisible();
        await expect(page.locator('main form[method=post]')).toHaveCount(0);
      } else {
        expect.soft(response.status(), path).toBe(200);
        await expect.soft(heading, path).toBeVisible();
      }
      await expect.soft(page.locator('html')).toHaveAttribute('lang', lang);
      await expect.soft(page.locator('main form[action$="/login"]')).toHaveCount(0);
    });
  }
}

async function characterProfile(page, lang, player) {
  const paths = `/${lang}/account/characters`;
  const text = messages('crates/characters/src/create_web/{lang}.ftl', lang);
  const response = await navigate(page, `${paths}/new`);
  const form = page.locator('main form[method=post]:has([name=name])');
  if (await form.count() === 0) {
    expect([200, 503], 'Documented Game unavailability').toContain(response.status());
    await expect(page.locator('main')).toContainText(text.unavailable);
    test.info().annotations.push({ type: 'staging limitation', description: 'Game character creation is unavailable' });
    return;
  }
  expect(response.status()).toBe(200);
  const name = form.locator('[name=name]');
  const world = form.locator('[name=world]');
  await world.selectOption({ index: 1 });
  await form.locator('[name=sex]').first().check();
  await form.locator('[name=vocation]').first().check();
  for (const invalid of ['Łucja', 'Character'.repeat(4)]) {
    await name.fill(invalid);
    expect((await submit(form)).status()).toBe(303);
    await expect(page.locator('main')).toContainText(text['name-invalid']);
  }
  await name.fill(`  ${player.name}  `);
  expect((await submit(form)).status()).toBe(303);
  await expect(page).toHaveURL(/\/account\/characters\/requests\/[\da-f-]+$/);
  const statusUrl = page.url();
  await expect(page.locator('main')).toContainText(text.pending);
  await expect.poll(async () => {
    await navigate(page, statusUrl);
    return (await page.locator('main').textContent()).includes(text.created);
  }, { timeout: 30_000, intervals: [3_000, 6_000] }).toBe(true);
  await checkOverview(page, lang);
  const preferences = page.locator('main a[href$="/profile"]').first();
  await expect(preferences).toBeVisible();
  await preferences.click();
  const profile = page.locator('main form[method=post]');
  const comment = lang === 'pl' ? 'Pierwsza wyprawa' : 'First adventure';
  await profile.locator('[name=comment]').fill(comment);
  await profile.locator('[name=is_main_character]').check();
  expect((await submit(profile)).status()).toBe(303);
  expect((await page.reload({ waitUntil: 'domcontentloaded' })).status()).toBe(200);
  await expect(profile.locator('[name=comment]')).toHaveValue(comment);
  await expect(profile.locator('[name=is_main_character]')).toBeChecked();
}

async function openLogout(page, lang) {
  await navigate(page, `/${lang}/`);
  const mobile = page.locator('#portal-menu-toggle');
  if (await mobile.isVisible()) await mobile.click();
  await page.locator('header a[data-account-link]').click();
  const form = page.locator('#podglad-konta form[action$="/logout"]');
  await expect(form).toBeVisible();
  return form;
}

test('verified newcomer explores account settings and uses two independent devices', async ({ page, browser }, info) => {
  const { lang } = info.project.metadata;
  const player = newcomer();
  const mail = await get(page.request, '/__mailpit/api/v1/messages?limit=100');
  test.skip([404, 405].includes(mail.status()), `Mailpit GET returned HTTP ${mail.status()}`);
  expect(mail.status(), 'Mailpit availability').toBe(200);
  await requireVerifiedPlayer(page, lang, player);
  expect((await login(page, lang, player)).status()).toBe(302);
  await checkOverview(page, lang);
  await exploreAccount(page, lang);

  const privacy = `/${lang}/account/privacy`;
  const notices = `/${lang}/account/notifications`;
  expect((await navigate(page, privacy)).status()).toBe(200);
  const settings = page.locator('main form');
  const publicStatus = settings.locator('[name=public_status_visible]');
  const showStatus = !await publicStatus.isChecked();
  await publicStatus.setChecked(showStatus);
  await settings.locator('[name=locale]').selectOption(lang);
  expect((await submit(settings)).status()).toBe(303);
  await expect(page).toHaveURL(origin + privacy);
  expect((await page.reload({ waitUntil: 'domcontentloaded' })).status()).toBe(200);
  await expect(publicStatus).toBeChecked({ checked: showStatus });

  expect((await navigate(page, notices)).status()).toBe(200);
  const email = page.locator('main form [name=news_email]');
  const receiveNews = !await email.isChecked();
  await email.setChecked(receiveNews);
  expect((await submit(page.locator('main form'))).status()).toBe(303);
  await expect(email).toBeChecked({ checked: receiveNews });
  await characterProfile(page, lang, player);

  const width = info.project.use.viewport.width === 390 ? 1440 : 390;
  const device = await browser.newContext({
    baseURL: origin,
    locale: lang,
    viewport: { width, height: width === 390 ? 844 : 900 },
    hasTouch: width === 390,
    proxy: info.project.use.proxy,
    ignoreHTTPSErrors: info.project.use.ignoreHTTPSErrors,
    serviceWorkers: 'block',
  });
  try {
    await routeStagingAccess(device);
    const second = await device.newPage();
    const desktop = width === 1440 ? second : page;
    const phone = width === 390 ? second : page;
    expect((await login(second, lang, player)).status()).toBe(302);
    await checkOverview(second, lang);
    await navigate(second, notices);
    await expect(second.locator('main [name=news_email]')).toBeChecked({ checked: receiveNews });
    await navigate(second, privacy);
    await expect(second.locator('main [name=public_status_visible]')).toBeChecked({ checked: showStatus });

    expect((await submit(await openLogout(phone, lang))).status()).toBe(302);
    await expect(phone).toHaveURL(`${origin}/${lang}/`);
    await checkOverview(desktop, lang);
    await navigate(phone, notices);
    await expect(phone).toHaveURL(`${origin}/${lang}/login`);
    expect((await login(phone, lang, player, null)).status()).toBe(302);
    await expect.soft(phone, 'Sign-in resumes the original private page').toHaveURL(origin + notices);
    await checkOverview(phone, lang);
    const sessions = `/${lang}/account/sessions`;
    expect((await navigate(desktop, sessions)).status()).toBe(200);
    await expect(desktop.locator(`main li form[action^="${sessions}/"]`)).toHaveCount(2);

    expect((await submit(await openLogout(phone, lang))).status()).toBe(302);
    const destination = 'https://example.invalid/';
    const query = new URLSearchParams({ next: destination, return: destination });
    await navigate(phone, `/${lang}/login?${query}`);
    const fields = {
      _csrf: await phone.locator('main form [name=_csrf]').inputValue(),
      email: player.email,
      password: player.password,
      next: destination,
      return: destination,
    };
    const result = await postForm(phone.request, `/${lang}/login?${query}`, fields);
    expect(result.status()).toBe(302);
    expect(Boolean(result.headers().location)).toBe(true);
    expect(new URL(result.headers().location, origin).origin).toBe(origin);
    await checkOverview(phone, lang);
  } finally {
    await device.unrouteAll({ behavior: 'ignoreErrors' });
    await device.close();
  }
});
