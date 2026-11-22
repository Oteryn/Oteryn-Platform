// These fallback contracts belong to the browser crawl's empty Game fixture.
const signedIn = ['player', 'platform_admin'];
const allActors = ['anonymous', ...signedIn];
const unavailableTitle = ['Usługa chwilowo niedostępna', 'Temporarily unavailable'];
const unavailable = ['Niedostępne', 'Unavailable'];
const bazaar = [
  'Brak zatwierdzonego źródła uprawnień do depozytu i podpisu postaci. Wystawienie, licytacja i przeniesienie są niedostępne.',
  'Accepted character custody and signing authority are unavailable. No listing, bid or transfer can be accepted.',
];
const projectedRoutes = new Map([
  ['atlas', [allActors, 'h1', unavailableTitle]],
  ['atlas/delivery', [allActors, null, unavailable]],
  ['wiki', [allActors, 'h1', ['Dane świata są obecnie niedostępne.', 'World data is currently unavailable.']]],
  ['wiki/creatures', [allActors, 'h1', ['Dane świata są obecnie niedostępne.', 'World data is currently unavailable.']]],
  ['houses', [allActors, '[role="status"]', ['Informacje o domach są niedostępne. Spróbuj ponownie później.', 'House information is unavailable. Please try again later.']]],
  ['guilds', [allActors, '[role="status"]', ['Informacje o gildiach są niedostępne.', 'Guild information is unavailable.']]],
  ['online', [allActors, '[role="status"]', ['Tymczasowo niedostępne.', 'Temporarily unavailable.']]],
  ['deaths', [allActors, '[role="status"]', ['Dane o zgonach są obecnie niedostępne.', 'Death records are currently unavailable.']]],
  ['rankings/experience', [allActors, '[role="status"]', ['Tymczasowo niedostępne. Spróbuj ponownie później.', 'Temporarily unavailable. Please try again later.']]],
  ['stats/guilds', [allActors, '[role="status"]', unavailable]],
  ...['yesterday', 'week', 'month', 'record', 'oldest', 'anniversaries', 'name-changes', 'transfers', 'sold']
    .map(kind => [`stats/rankings/${kind}`, [allActors, '[role="status"]', ['Statystyki są niedostępne. Spróbuj ponownie później.', 'Statistics are unavailable. Please try again later.']]]),
  ['league', [allActors, '[role="status"]', ['Autorytatywne wyniki są obecnie niedostępne.', 'Authoritative results are currently unavailable.']]],
  ['bazaar', [allActors, 'p', bazaar]],
  ['account/bazaar', [signedIn, 'p', bazaar]],
  ['account/journal/reports', [signedIn, 'h1', unavailableTitle]],
  ['account/game-logins', [signedIn, 'h1', unavailableTitle]],
  ['atlas/places', [signedIn, 'h1', unavailableTitle]],
  ['pass', [signedIn, '[role="status"]', ['Przepustka sezonowa jest obecnie niedostępna. Nie pobrano coinów ani nie wydano nagród.', 'The season pass is currently unavailable. No coins were charged and no rewards were issued.']]],
]);

export function emptyGameFixture(env) {
  return env.PORTAL_E2E_CRAWL_FIXTURE === 'empty-game'
    && !env.OTERYN_WORLD_CATALOG_FILE
    && !env.OTERYN_WORLD_CATALOG_URL
    && !env.PORTAL_E2E_LEAGUE_CONTROL
    && !env.OTERYN_LEAGUE_SOURCE_AUTHORITY
    && env.OTERYN_BAZAAR_ENABLED !== 'true';
}

export function coldView(url, actor, emptyGame) {
  if (!emptyGame || url.search) return;
  const match = url.pathname.match(/^\/(pl|en)\/(.+)$/);
  if (!match) return;
  const [, lang, path] = match;
  const entry = projectedRoutes.get(path);
  if (!entry || !entry[0].includes(actor)) return;
  return {selector: entry[1], text: entry[2][lang === 'en' ? 1 : 0], filters: path === 'stats/guilds' || path.startsWith('stats/rankings/')};
}

export function protectedDestination(url, locale, actor, emptyGame) {
  const match = url.pathname.match(/^\/(?:(pl|en)\/)?(.+)$/);
  if (!match) return;
  const [, lang = locale, path] = match;
  if (/^(admin|console)(\/|$)/.test(path)) {
    return {path: url.pathname, status: actor === 'platform_admin' ? 200 : 403};
  }
  const account = /^(account|panel|support\/(tickets|reports)|atlas\/places|pass)(\/|$)/.test(path);
  const security = /^(mfa|password\/change)(\/|$)/.test(path);
  if (!account && !(security && signedIn.includes(actor))) return;
  if (actor === 'anonymous') {
    return {path: path === 'account/passkeys' ? url.pathname : `/${lang}/login`, status: 200};
  }
  if (path === 'account/security') return {path: `/${lang}/password/change`, status: 200};
  const view = coldView(url, actor, emptyGame);
  return {path: url.pathname, status: view ? 503 : 200};
}
