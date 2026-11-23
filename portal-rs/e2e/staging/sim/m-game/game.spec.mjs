import { check, expect, labels, staging, test } from './helpers.mjs';

const id = '018f0000-0000-7000-8000-000000000001';
const otherId = '018f0000-0000-7000-8000-000000000002';
const name = encodeURIComponent('Absent Sim Character');

async function statuses(request, info, status, routes) {
  for (const entry of routes) {
    const [path, method = 'GET'] = typeof entry === 'string' ? [entry] : entry;
    await test.step(`${method} ${path}`, async () => {
      const response = await staging(request, path, { method });
      try {
        await check(response, status, info);
      } finally {
        await response.dispose();
      }
    });
  }
}

async function redirect(request, info, path, status, location) {
  const response = await staging(request, path);
  try {
    await check(response, status, info);
    expect(response.headers().location, path).toBe(location);
  } finally {
    await response.dispose();
  }
}

test('Public worlds and character directory', async ({ request, screen }, info) => {
  const { lang } = info.project.metadata;
  const t = labels(lang);
  const prefix = `/${lang}`;
  const other = lang === 'pl' ? 'en' : 'pl';
  const worlds = `${prefix}/worlds`;

  await check(await screen.goto(`${prefix}/`), 200, info, screen);
  await expect(screen.locator('html')).toHaveAttribute('lang', lang);
  for (const path of [worlds, `${prefix}/online`, `${prefix}/rankings/level`]) {
    await expect(screen.locator(`#portal-navigation a[href="${path}"]`)).toHaveCount(0);
  }
  await check(await screen.goto(worlds), 404, info, screen);
  await expect(screen.locator('main h1')).toHaveText(t('Nie znaleziono strony', 'Page not found'));
  await check(await screen.reload(), 404, info, screen);
  await expect(screen).toHaveURL(new RegExp(`${worlds}$`));
  await check(await screen.goBack(), 200, info, screen);
  await expect(screen.locator('html')).toHaveAttribute('lang', lang);
  const languageLink = screen.locator(`footer a[hreflang="${other}"]`);
  await expect(languageLink).toHaveAttribute('href', new RegExp(`^/${other}/?$`));
  await languageLink.click();
  await expect(screen.locator('html')).toHaveAttribute('lang', other);
  await check(await screen.goto(`/${other}/worlds`), 404, info, screen);
  await expect(screen.locator('html')).toHaveAttribute('lang', other);

  await statuses(request, info, 404, [
    '/worlds', '/api/v1/worlds', '/online', `${prefix}/online`, `${prefix}/online?page=0`,
    `${prefix}/characters`, `${prefix}/characters?name=${name}`,
    `${prefix}/characters?name=${'a'.repeat(65)}`, `${prefix}/characters/${name}`,
    `${prefix}/characters/${name}/skills`,
  ]);
  for (const [alias, location] of [
    ['/servers', '/pl/worlds'],
    ['/characters', '/pl/characters'],
    ['/characters/Absent', '/pl/characters/Absent'],
  ]) await redirect(request, info, alias, 301, location);
});

test('Highscores and statistics', async ({ request, screen }, info) => {
  const { lang } = info.project.metadata;
  const t = labels(lang);
  const stats = `/${lang}/stats`;
  const rankings = `/${lang}/rankings`;
  await redirect(request, info, '/highscores?page=2&vocation=1', 301, '/pl/rankings/level?page=2&vocation=1');
  await check(await screen.goto(`${rankings}/level`), 404, info, screen);
  await expect(screen.locator('main')).toContainText(t('Nie znaleziono strony.', 'Page not found.'));
  await expect(screen.locator('main table')).toHaveCount(0);

  await statuses(request, info, 404, [
    `${rankings}/experience`, `${rankings}/experience?page=0`, `${rankings}/unknown`,
    '/stats/world', `${stats}/world`, `${stats}/world?world=absent&start=2026-02-29`,
    '/stats/guilds', `${stats}/guilds`, `${stats}/guilds?days=32`,
    `${stats}/rankings/yesterday`, `${stats}/rankings/yesterday?page=0`, `${stats}/rankings/unknown`,
    `${stats}/compare`, `${stats}/compare?ids=${id},${otherId}&metric=level`, `${stats}/compare?ids=invalid`,
    ...['history', 'experience', 'rankings', 'deaths', 'progress', 'online', 'unknown']
      .map(tab => `${stats}/characters/${id}/${tab}`),
    `${stats}/characters/invalid/history`,
    ...['GET', 'POST', 'DELETE'].map(method => ['/api/v1/stats/watches', method]),
  ]);
});

test('Atlas and map services', async ({ request, screen }, info) => {
  const { lang } = info.project.metadata;
  const t = labels(lang);
  const atlas = `/${lang}/atlas`;
  const version = '0'.repeat(40);
  const viewport = `version=${version}&floor=7&min_x=1&min_y=2&max_x=3&max_y=4`;

  await check(await screen.goto(`${atlas}/delivery`), 404, info, screen);
  await expect(screen.locator('main h1')).toHaveText(t('Mapa dostaw zadań i gdzie wypada', 'Quest delivery and item sources'));
  await expect(screen.locator('main')).toContainText(t('Niedostępne', 'Unavailable'));
  await expect(screen.locator('main form')).toHaveCount(0);
  await statuses(request, info, 404, [
    atlas, `${atlas}?x=-1&z=7`, `/atlas/tiles/${version}/7/1/0/0.webp`, '/atlas/tiles/invalid/7/1/0/0.webp',
    `${atlas}/delivery?item=rope&count=2`, `${atlas}/delivery?count=0`, '/atlas/delivery.css',
    `/atlas/spawns-text/${lang}`, '/atlas/spawns-text/xx',
    `/atlas/spawns?${viewport}`, `/atlas/spawns?${viewport.replace('max_x=3', 'max_x=0')}`,
    '/atlas/private-markers', ['/atlas/private-markers', 'POST'],
    [`/atlas/private-markers/${id}`, 'DELETE'], ['/atlas/map-reports', 'POST'],
    `${atlas}/bestiary/${id}`, `${atlas}/bestiary/invalid?stars=6`,
  ]);
  await statuses(request, info, 503, [`${atlas}/places/participation`, [`${atlas}/places/participation`, 'POST']]);
});

test('Player navigation and signed-out account views', async ({ request, screen }, info) => {
  const { lang } = info.project.metadata;
  const t = labels(lang);
  const prefix = `/${lang}`;
  const character = `account/characters/${name}`;
  const paths = [
    'account', 'account/badges', `${character}/profile`, `${character}/services`,
    `${character}/achievements`, `${character}/equipment`, `${character}/outfit`,
    'account/characters/new', `account/characters/requests/${id}`, 'account/characters/suggest',
    'account/dashboard', 'account/game-logins', 'panel/hirelings', 'account/hunts',
    'account/hunts/compare', `account/hunts/${id}`, `account/hunts/${id}/share`,
    `panel/items/${id}/${otherId}`, `panel/history/${id}/${otherId}`, `panel/history/${id}/${otherId}/${id}`,
    'account/journal/reports', 'account/journal/reports/notes', 'account/journal/reports/notes/export', 'atlas/places',
  ];
  await check(await screen.goto(`${prefix}/account/characters/new`), 200, info, screen);
  await expect(screen).toHaveURL(new RegExp(`${prefix}/login$`));
  const heading = screen.locator('main h1');
  const expectedHeading = t('Logowanie', 'Log in');
  if ((await heading.innerText()).trim() !== expectedHeading) {
    await screen.screenshot({ path: info.outputPath('login-heading.png'), timeout: 5_000 }).catch(() => {});
  }
  await expect.soft(heading).toHaveText(expectedHeading);
  await expect(screen.locator('main input[name=password]')).toBeVisible();
  for (const path of paths) await redirect(request, info, `${prefix}/${path}`, 303, `${prefix}/login`);
  for (const path of ['/account', '/account/badges', '/account/game-logins', '/account/characters/Absent/profile']) {
    await redirect(request, info, path, 303, '/pl/login');
  }
  await statuses(request, info, 404, ['/static/playerdata/hunts.css']);
});

test('Staff game surfaces on the public staging host', async ({ request }, info) => {
  const { lang } = info.project.metadata;
  const read = ['/admin', '/admin/console/audit', '/admin/console/access', '/admin/console/approvals', '/admin/players'];
  const forms = [
    '/admin/world', '/console/dashboard', '/console/worlds', '/console/nodes', '/console/database',
    '/admin/console/players', `/admin/players/${id}`, '/admin/control/step-up',
    `/admin/control/approvals/${id}`, `/admin/${lang}/bazaar`,
  ];
  await statuses(request, info, 404, [...read, ...forms.flatMap(path => [path, [path, 'POST']])]);
});
