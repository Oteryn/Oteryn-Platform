import { afterSharedQuota, expect, get, messages, navigate, origin, test, ui } from './helpers.mjs';

const otherLanguage = lang => lang === 'pl' ? 'en' : 'pl';
const localized = (path, key) => lang => messages(path, lang)[key];
const shell = key => lang => ui(lang)[key];
const pages = key => localized('templates/pages/{lang}.ftl', key);
const wiki = key => localized('crates/wiki/src/i18n/{lang}.ftl', key);
const terms = lang => lang === 'pl' ? 'Regulamin Oteryn' : 'Terms of Service';
const privacy = lang => lang === 'pl' ? 'Polityka prywatności' : 'Privacy Policy';
const missing = shell('error-404-title');

const publicPages = {
  '/': [200, shell('shell-news')],
  '/wiki': [200, wiki('wiki')],
  '/news': [200, shell('shell-news')],
  '/events': [200, shell('shell-events')],
  '/today': [200, lang => lang === 'pl' ? 'Dziś' : 'Today'],
  '/help': [200, pages('support')],
  '/support/report-a-bug': [200, pages('report-a-bug')],
  '/rules': [200, pages('not-published')],
  '/server-information': [200, pages('not-published')],
  '/legal/terms': [200, terms],
  '/legal/privacy': [200, privacy],
  '/legal/cookies': [200, lang => lang === 'pl' ? 'Polityka plików cookie' : 'Cookie Policy'],
  '/login': [200, localized('templates/account/mfa/{lang}.ftl', 'mfa-login')],
  '/register': [200, shell('register-title')],
  '/bazaar': [503, localized('templates/bazaar/{lang}.ftl', 'authority-title')],
};

const gamePages = {
  '/wiki/creatures': [404, localized('crates/wiki/src/entities/{lang}.ftl', 'missing')],
  '/wiki/calculators': [404, missing],
  '/atlas': [404, missing],
  '/atlas/delivery': [404, localized('crates/atlas/src/delivery/{lang}.ftl', 'title')],
  '/worlds': [404, missing],
  '/online': [404, missing],
  '/rankings/experience': [404, shell('rankings-title')],
  '/stats/world': [404, missing],
  '/stats/guilds': [404, missing],
  '/stats/rankings/yesterday': [404, localized('templates/stats/rankings/{lang}.ftl', 'title')],
  '/stats/compare': [404, localized('templates/stats/character/{lang}.ftl', 'title')],
  '/guilds': [404, missing],
  '/houses': [404, missing],
  '/deaths': [404, missing],
  '/party': [404, missing],
  '/league': [404, missing],
  '/puzzles': [404, missing],
  '/collection': [404, missing],
  '/download': [404, missing],
};

async function checkPage(page, response, path, lang, [status, heading]) {
  expect.soft(response.status(), `${lang}${path} status`).toBe(status);
  await expect.soft(page, `${lang}${path} destination`).toHaveURL(`${origin}/${lang}${path}`);
  await expect.soft(page.locator('html')).toHaveAttribute('lang', lang);
  await expect.soft(page.locator('main h1'), `${lang}${path} localized heading`).toHaveText(heading(lang));
}

async function switchPage(page, path, lang, expected) {
  const alternate = otherLanguage(lang);
  const link = page.locator(`footer a[hreflang=${alternate}]`);
  await expect(link).toHaveAttribute('href', `/${alternate}${path}`);
  const [response] = await Promise.all([
    page.waitForResponse(reply => reply.request().isNavigationRequest()
      && new URL(reply.url()).pathname === `/${alternate}${path}`, { timeout: 25_000 }),
    link.click(),
  ]);
  if (response.status() === 429) {
    await afterSharedQuota(response);
    await checkPage(page, await navigate(page, `/${alternate}${path}`), path, alternate, expected);
  } else {
    await checkPage(page, response, path, alternate, expected);
  }
}

async function openMenu(page) {
  const mobile = page.locator('#portal-menu-toggle');
  if (await mobile.isVisible()) await mobile.click();
  const menu = page.locator('#portal-navigation');
  await expect(menu).toBeVisible();
  return menu;
}

test('anonymous homepage, navigation, footer, and language switches', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  await checkPage(page, await navigate(page, `/${lang}/`), '/', lang, publicPages['/']);
  await expect(page.locator('#jak-zaczac h2')).toHaveText(ui(lang)['thais-home-start']);
  await expect(page.locator('#jak-zaczac')).toContainText(ui(lang)['stage-btn-download']);
  await expect(page.locator('#jak-zaczac a[href$="/server-information"]')).toBeVisible();
  const nav = await openMenu(page);
  const world = nav.locator('#portal-world-button');
  const community = nav.locator('#portal-community-button');
  const panel = nav.locator('#portal-community-panel');
  if (info.project.use.hasTouch) {
    await world.tap();
    await expect(nav.locator('#portal-world-panel')).toBeVisible();
    await community.tap();
    await expect.soft(panel, 'A tap switches from Game World to Community').toBeVisible();
    if (!await panel.isVisible()) {
      await page.screenshot({ path: info.outputPath('mobile-menu-switch.png') });
      if (!await nav.isVisible()) await openMenu(page);
      await community.press('Enter');
      await expect(panel).toBeVisible();
    }
  } else {
    await world.click();
    await expect(nav.locator('#portal-world-panel')).toBeVisible();
    await community.click();
    await expect(panel).toBeVisible();
  }

  const links = await page.locator('header#portal-header a[href], footer.foot a[href]')
    .evaluateAll(nodes => Array.from(new Set(nodes.map(node => node.getAttribute('href')))));
  await expect(page.locator('footer .foot-social a')).toHaveAttribute('href',
    'https://www.twitch.tv/directory/category/tibia');
  await expect(page.locator('footer .foot-community-status')).toHaveText(ui(lang)['thais-foot-discord-unavailable']);

  for (const fragment of links.filter(href => href.startsWith('#'))) {
    await expect(page.locator(fragment), `Home fragment ${fragment}`).toHaveCount(1);
  }
  const getStarted = nav.locator('a[href="#jak-zaczac"]');
  const guide = `${origin}/${lang}/#jak-zaczac`;
  if (info.project.use.hasTouch) {
    await getStarted.tap();
    await expect.soft(page, 'Getting started opens from the expanded mobile menu').toHaveURL(guide);
    if (page.url() !== guide) {
      await page.screenshot({ path: info.outputPath('mobile-getting-started.png') });
      await page.locator('footer a[href="#jak-zaczac"]').tap();
    }
  } else {
    await getStarted.click();
  }
  await expect(page).toHaveURL(guide);
  await page.goBack({ waitUntil: 'domcontentloaded' });
  await expect(page).toHaveURL(`${origin}/${lang}/`);
  await switchPage(page, '/', lang, publicPages['/']);

  const publishedPaths = links.filter(href => href.startsWith(`/${lang}/`))
    .map(href => href.slice(3));
  expect(publishedPaths).toEqual(expect.arrayContaining([
    '/wiki', '/news', '/events', '/today', '/help', '/support/report-a-bug',
    '/rules', '/server-information', '/legal/terms', '/legal/privacy', '/login',
  ]));

  for (const path of new Set([...publishedPaths, '/register', '/legal/cookies'])) {
    if (path === '/') continue;
    await test.step(path, async () => {
      const expected = publicPages[path] ?? gamePages[path];
      expect(expected, `Published route ${path} has an expectation`).toBeDefined();
      await checkPage(page, await navigate(page, `/${lang}${path}`), path, lang, expected);
      await switchPage(page, path, lang, expected);
    });
  }
  for (const href of links.filter(href => href.startsWith('/static/'))) {
    expect(href).toBe(`/static/thais/shop-${lang}.html`);
    const response = await navigate(page, href);
    expect(response.status(), 'Shop preview fallback').toBe(200);
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    await expect(page.locator('h1').first()).toBeVisible();
    const shopTitle = locale => locale === 'pl' ? 'Sklep Oterynu' : 'Oteryn Shop';
    await expect(page.locator('main h1')).toHaveText(shopTitle(lang));
    const alternate = otherLanguage(lang);
    const previewLanguage = page.locator(`footer a[hreflang=${alternate}]`);
    await expect(previewLanguage).toHaveAttribute('href', `shop-${alternate}.html`);
    await previewLanguage.click();
    await expect(page).toHaveURL(`${origin}/static/thais/shop-${alternate}.html`);
    await expect(page.locator('html')).toHaveAttribute('lang', alternate);
    await expect(page.locator('main h1')).toHaveText(shopTitle(alternate));
  }

  const badNews = '/news/Bad--Slug';
  await checkPage(page, await navigate(page, `/${lang}${badNews}`), badNews, lang, [404, missing]);
  const badEvents = await navigate(page, `/${lang}/events?page=-1`);
  expect(badEvents.status()).toBe(400);
  await expect(page.locator('main h1')).toHaveText(ui(lang)['error-400-title']);
});

test('catalogue, game availability, and wiki search validation', async ({ page, request }, info) => {
  const { lang } = info.project.metadata;
  for (const [path, expected] of Object.entries(gamePages)) {
    await test.step(path, async () => {
      await checkPage(page, await navigate(page, `/${lang}${path}`), path, lang, expected);
      await switchPage(page, path, lang, expected);
    });
  }
  const bounds = 'version=1&floor=7&min_x=1&min_y=1&max_x=3&max_y=3';
  expect((await get(request, `/atlas/spawns?${bounds}`)).status(), 'Disabled spawns endpoint').toBe(404);

  await navigate(page, `/${lang}/wiki`);
  const search = page.locator('main form[action$="/wiki/search"]');
  await search.locator('[name=q]').fill('a');
  const [invalid] = await Promise.all([
    page.waitForResponse(reply => reply.request().isNavigationRequest()
      && new URL(reply.url()).searchParams.get('q') === 'a', { timeout: 25_000 }),
    search.getByRole('button').click(),
  ]);
  expect(invalid.status()).toBe(422);
  await expect(page.locator('main h1')).toHaveText(wiki('invalid')(lang));
  const missingArticle = await navigate(page, `/${lang}/wiki/articles/sim-newcomer-missing-guide`);
  expect(missingArticle.status()).toBe(404);
  await expect(page.locator('main h1')).toHaveText(wiki('missing')(lang));
});
