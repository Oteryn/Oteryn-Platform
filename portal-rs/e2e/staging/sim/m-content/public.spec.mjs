import { expect } from '@playwright/test';
import { follow, status, test, visit } from './helpers.mjs';

const copy = {
  pl: {
    news: 'Aktualności', newsEmpty: 'Brak opublikowanych aktualności.',
    events: 'Wydarzenia', eventsEmpty: 'Brak opublikowanych wydarzeń.',
    today: 'Dziś', announcements: 'Ogłoszenia',
    unpublished: 'Ta strona nie została jeszcze opublikowana.',
    unavailable: 'Ta strona jest obecnie niedostępna.',
    support: 'Pomoc dla graczy', report: 'Zgłoś błąd',
    terms: 'Regulamin Oteryn', privacy: 'Polityka prywatności', cookies: 'Polityka plików cookie',
    wikiEmpty: 'Nie znaleziono opublikowanych poradników.',
    wikiMissing: 'Nie znaleziono artykułu.', entityMissing: 'Nie znaleziono strony.',
    wikiInvalid: 'Sprawdź dane. Wyszukiwanie wymaga 2–80 znaków i strony od 1 do 1000.',
    places: 'Miejsca polowań, bestiariusza i bossów', placesEmpty: 'Brak zatwierdzonych danych.',
  },
  en: {
    news: 'News', newsEmpty: 'No news has been published yet.',
    events: 'Events', eventsEmpty: 'No published events.',
    today: 'Today', announcements: 'Announcements',
    unpublished: 'This page has not been published yet.',
    unavailable: 'This page is currently unavailable.',
    support: 'Player support', report: 'Report a bug',
    terms: 'Terms of Service', privacy: 'Privacy Policy', cookies: 'Cookie Policy',
    wikiEmpty: 'No published guides found.',
    wikiMissing: 'Article not found.', entityMissing: 'Page not found.',
    wikiInvalid: 'Check the input. Search requires 2–80 characters and a page from 1 to 1000.',
    places: 'Hunting, bestiary and boss places', placesEmpty: 'No qualified data available.',
  },
};
const missing = 'sim-m-content-unpublished-example';
const kinds = ['items', 'creatures', 'npcs', 'spells', 'quests', 'loot', 'achievements', 'areas'];

test.afterEach(async ({ page }, info) => {
  if (info.status !== info.expectedStatus) {
    await page.screenshot({ path: info.outputPath('public-page.png'), timeout: 5000 }).catch(() => {});
  }
});

async function listOutcome(page, path, empty) {
  const entries = page.locator(`main a[href^="${path}/"]`);
  if (await entries.count()) {
    const first = entries.first();
    await follow(page, await first.getAttribute('href'), (await first.innerText()).trim());
    await page.goBack({ waitUntil: 'domcontentloaded' });
    await expect(page).toHaveURL(url => url.pathname === path);
  } else {
    await expect(page.locator('main')).toContainText(empty);
  }
}

test('News and events navigation, aliases and publication visibility', async ({ page, request }, info) => {
  const lang = info.project.metadata.locale;
  const t = copy[lang];
  await visit(page, `/${lang}/`, 200);
  await follow(page, `/${lang}/news`, t.news);
  await listOutcome(page, `/${lang}/news`, t.newsEmpty);
  await visit(page, `/news?lang=${lang}&page=1`, 200, t.news);
  await follow(page, `/${lang}/events`, t.events);
  await listOutcome(page, `/${lang}/events`, t.eventsEmpty);

  for (const path of ['/events', `/events/${missing}`]) {
    const response = await status(request, 'GET', path, 303);
    expect.soft(response.headers().location).toBe('/pl' + path);
  }
  for (const [path, code] of [
    ['/news?page=0', 422], [`/${lang}/news?page=100001`, 422], [`/${lang}/news?page=text`, 400],
    [`/news/${missing}`, 404], [`/${lang}/news/${missing}`, 404],
    [`/${lang}/news/${'a'.repeat(161)}`, 404], ['/xx/news', 404], [`/xx/news/${missing}`, 404],
    ['/events/INVALID', 404], [`/${lang}/events/${missing}`, 404],
    [`/${lang}/events/INVALID`, 404], [`/${lang}/events?page=-1`, 400], ['/xx/events', 404],
  ]) {
    await status(request, 'GET', path, code);
  }
});

test('Editorial, help and legal pages remain readable across language and reload', async ({ page, request }, info) => {
  const lang = info.project.metadata.locale;
  const t = copy[lang];
  await visit(page, `/${lang}/`, 200);
  for (const [path, title] of [
    ['rules', t.unpublished], ['server-information', t.unpublished],
    ['help', t.support], ['support/report-a-bug', t.report],
    ['legal/terms', t.terms], ['legal/privacy', t.privacy],
  ]) await follow(page, `/${lang}/${path}`, title);

  await visit(page, `/${lang}/support`, 200, t.support);
  await visit(page, `/${lang}/legal/cookies`, 200, t.cookies);
  expect((await page.reload({ waitUntil: 'domcontentloaded' })).status()).toBe(200);
  await expect(page.locator('main h1')).toHaveText(t.cookies);
  await expect(page.locator('main article')).toBeVisible();
  const other = lang === 'en' ? 'pl' : 'en';
  const opposite = page.locator(`a[hreflang="${other}"][href="/${other}/legal/cookies"]`);
  expect(await opposite.count()).toBeGreaterThan(0);
  await follow(page, `/${other}/legal/cookies`, copy[other].cookies);
  await expect(page.locator('html')).toHaveAttribute('lang', other);
  const secondTab = await page.context().newPage();
  try {
    await visit(secondTab, `/${lang}/legal/cookies`, 200, t.cookies);
    await expect(page.locator('main h1')).toHaveText(copy[other].cookies);
  } finally {
    await secondTab.close();
  }
  await page.goBack({ waitUntil: 'domcontentloaded' });
  await expect(page.locator('main h1')).toHaveText(t.cookies);

  await visit(page, `/${lang}/pages/${missing}`, 404, t.unavailable);
  for (const path of [`/${lang}/pages/Invalid_Slug`, `/${lang}/pages/${'a'.repeat(161)}`,
    `/${lang}/pages/terms-of-service`, '/xx/pages/example', '/xx/legal/terms']) {
    await status(request, 'GET', path, 404);
  }
});

test('Today and the announcement ticker show localized publication state', async ({ page, request }, info) => {
  const lang = info.project.metadata.locale;
  const t = copy[lang];
  await visit(page, `/${lang}/`, 200);
  await follow(page, `/${lang}/today`, t.today);
  const announcements = page.getByRole('region', { name: t.announcements, exact: true });
  await expect(announcements).toBeVisible();
  const firstAnnouncement = await announcements.locator('article h3').first().textContent({ timeout: 1000 })
    .catch(() => null);
  await visit(page, `/today?lang=${lang}`, 200, t.today);
  await expect(page.getByRole('region', { name: t.announcements, exact: true })).toBeVisible();

  for (const path of [`/${lang}/announcements/ticker`, `/announcements/ticker?lang=${lang}`]) {
    const response = await status(request, 'GET', path, 200);
    expect.soft(response.headers()['cache-control']).toContain('no-store');
    const content = await response.text();
    if (firstAnnouncement) expect.soft(content).toContain(firstAnnouncement);
    if (content) expect.soft(content).toContain(t.announcements);
  }
  for (const path of ['/today?lang=xx', '/xx/today', '/announcements/ticker?lang=xx', '/xx/announcements/ticker']) {
    await status(request, 'GET', path, 404);
  }
});

test('Wiki search, history navigation and unavailable world data', async ({ page, request }, info) => {
  const lang = info.project.metadata.locale;
  const t = copy[lang];
  const base = `/${lang}/wiki`;
  const unprefixed = { headers: { 'Accept-Language': lang } };
  await visit(page, `/${lang}/`, 200);
  await follow(page, base, 'Wiki');
  const guides = page.locator('main article h2 a');
  if (await guides.count()) {
    const guide = guides.first();
    await follow(page, await guide.getAttribute('href'), (await guide.innerText()).trim());
    await page.goBack({ waitUntil: 'domcontentloaded' });
  } else {
    await expect(page.locator('main')).toContainText(t.wikiEmpty);
  }

  const form = page.locator(`main form[action="${base}/search"]`);
  const query = form.locator('input[name=q]');
  await query.fill(missing);
  await form.getByRole('button').click();
  await expect(page).toHaveURL(url => url.pathname === `${base}/search` && url.searchParams.get('q') === missing);
  await expect(page.locator('main')).toContainText(t.wikiEmpty);
  await page.goBack({ waitUntil: 'domcontentloaded' });
  await expect(page).toHaveURL(url => url.pathname === base);
  for (const path of [base, `${base}/search`]) {
    await visit(page, `${path}?q=x`, 422, t.wikiInvalid);
  }
  await visit(page, `${base}/search?q=${'a'.repeat(81)}`, 422, t.wikiInvalid);
  for (const path of ['/wiki', '/wiki/search']) {
    const response = await status(request, 'GET', `${path}?q=${missing}`, 200, unprefixed);
    expect.soft(await response.text()).toContain(t.wikiEmpty);
    await status(request, 'GET', `${path}?page=0`, 422, unprefixed);
  }
  await visit(page, `${base}/articles/${missing}`, 404, t.wikiMissing);
  const article = await status(request, 'GET', `/wiki/articles/${missing}`, 404, unprefixed);
  expect.soft(await article.text()).toContain(t.wikiMissing);

  for (const path of [`${base}/calculators`, `${base}/calculators?calculator=loot&total=101&members=4`]) {
    await status(request, 'GET', path, 404);
  }
  await visit(page, `${base}/places?purpose=hunt&sort=experience&party=1`, 503, t.places);
  await expect(page.locator('main')).toContainText(t.placesEmpty);
  await status(request, 'GET', `${base}/places?sort=invalid`, 400);
  await status(request, 'GET', `${base}/places?party=text`, 400);

  for (const prefix of [base, '/wiki']) {
    for (const kind of kinds) {
      for (const suffix of [kind, `${kind}/${missing}`]) {
        const response = await status(request, 'GET', `${prefix}/${suffix}`, 404, unprefixed);
        expect.soft(await response.text(), `${prefix}/${suffix}`).toContain(t.entityMissing);
      }
    }
    await status(request, 'GET', `${prefix}/items?page=text`, 422, unprefixed);
    await status(request, 'GET', `${prefix}/unknown-kind`, 404, unprefixed);
  }
});
