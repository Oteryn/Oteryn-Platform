const unavailable = ['Usługa chwilowo niedostępna', 'Temporarily unavailable'];
const coldCatalogue = ['Dane świata są obecnie niedostępne.', 'World data is currently unavailable.'];
const coldStats = ['Statystyki są niedostępne. Spróbuj ponownie później.', 'Statistics are unavailable. Please try again later.'];
const bazaar = [
  'Brak zatwierdzonego źródła uprawnień do depozytu i podpisu postaci. Wystawienie, licytacja i przeniesienie są niedostępne.',
  'Accepted character custody and signing authority are unavailable. No listing, bid or transfer can be accepted.',
];
const status = '[role="status"]';
const heading = ['h1'];
const notice = ['h1', 'p'];
const statusNotice = ['h1', `p${status}`];
const shellRefusal = ['section.op-shell__hero:has(> p.op-eyebrow + h1 + p + div > a)'];

const publicViews = new Map([
  ['atlas', ['h1', unavailable, 'No accepted Game atlas export', shellRefusal]],
  ['atlas/delivery', [null, ['Niedostępne', 'Unavailable'], 'No published delivery capabilities', heading]],
  ...['wiki', ...['search', 'items', 'creatures', 'npcs', 'spells', 'quests',
    'achievements', 'loot', 'areas'].map(kind => `wiki/${kind}`)]
    .map(route => [route, ['h1', coldCatalogue, 'No active Game catalogue', heading]]),
  ['wiki/places', ['p', ['Brak zatwierdzonych danych.', 'No qualified data available.'], 'No approved places data', ['h1', 'p', 'p']]],
  ['houses', [status, ['Informacje o domach są niedostępne. Spróbuj ponownie później.', 'House information is unavailable. Please try again later.'], 'No house projection', statusNotice]],
  ['guilds', [status, ['Informacje o gildiach są niedostępne.', 'Guild information is unavailable.'], 'No guild projection', statusNotice]],
  ['online', [status, ['Tymczasowo niedostępne.', 'Temporarily unavailable.'], 'No online projection', statusNotice]],
  ['deaths', [status, ['Dane o zgonach są obecnie niedostępne.', 'Death records are currently unavailable.'], 'No death projection', statusNotice]],
  ['rankings/experience', [status, ['Tymczasowo niedostępne. Spróbuj ponownie później.', 'Temporarily unavailable. Please try again later.'], 'No ranking projection', statusNotice]],
  ['stats/guilds', [status, ['Niedostępne', 'Unavailable'], 'No guild statistics projection', ['h1', 'p', 'form[method="get"]', `p${status}`, 'p'], true]],
  ...['yesterday', 'week', 'month', 'record', 'oldest', 'anniversaries', 'name-changes', 'transfers', 'sold']
    .map(kind => [`stats/rankings/${kind}`, [status, coldStats, 'No growth ranking projection', ['h1', 'nav', 'p', `p${status}`, 'form[method="get"]'], true]]),
  ['league', [status, ['Autorytatywne wyniki są obecnie niedostępne.', 'Authoritative results are currently unavailable.'], 'No League results publisher', [...statusNotice, 'h2', 'section:empty', 'h2', 'section:empty']]],
  ['bazaar', ['p', bazaar, 'No accepted character custody/signing authority', notice]],
  ...['bazaar/items', 'bazaar/items/new'].map(route => [route, ['h1', [
    'Handel przedmiotami jest tymczasowo niedostępny. Depozyty i zarezerwowane środki pozostają zabezpieczone.',
    'Item trading is temporarily unavailable. Deposits and reserved funds remain protected.',
  ], 'No accepted item custody and receipt authority', heading]]),
]);

const accountViews = new Map([
  ...['account/bazaar', 'account/bazaar/sell']
    .map(route => [route, ['p', bazaar, 'No accepted character custody/signing authority', notice]]),
  ['account/game-logins', ['h1', unavailable, 'No accepted Game login ownership and baseline authority', shellRefusal]],
  ['account/dashboard', ['h1', unavailable, 'No closed-session and timer producers', shellRefusal]],
  ['account/journal/reports', ['h1', unavailable, 'No accepted private Journal source', shellRefusal]],
  ['atlas/places', ['h1', unavailable, 'No accepted places aggregate', shellRefusal]],
  ['pass', [status, [
    'Przepustka sezonowa jest obecnie niedostępna.',
    'The season pass is currently unavailable.',
  ], 'No accepted season source or cosmetic authority', statusNotice]],
]);

export function localeOf(url, fallback) {
  return url.pathname.match(/^\/(pl|en)(?:\/|$)/)?.[1] ?? fallback;
}

export function routeOf(url) {
  return url.pathname.replace(/^\/(?:pl|en)(?=\/|$)/, '').replace(/^\//, '');
}

export const staffRoute = route => /^(?:admin|console)(?:\/|$)/.test(route);
const staffAnalytics = route => /^admin\/analytics(?:\/|$)/.test(route);

export function staffDenied(route, actor) {
  return staffRoute(route) && (actor === 'player' || (actor === 'anonymous' && staffAnalytics(route)));
}

function isPrivate(route) {
  return /^(?:account(?:\/|$)|panel\/|support\/(?:tickets|reports)(?:\/|$)|atlas\/places$|pass$)/.test(route)
    && route !== 'account/passkeys';
}

export function redirectDestination(url, actor, locale) {
  const route = routeOf(url);
  const lang = localeOf(url, locale);
  const selectedLocale = route.match(/^language\/(pl|en)$/)?.[1];
  if (url.pathname === '/') return `/${locale}/`;
  if (selectedLocale) return `/${selectedLocale}/`;
  if (actor === 'anonymous') {
    if ((staffRoute(route) && !staffAnalytics(route)) || route === 'password/change') return '/login';
    if (isPrivate(route) || /^(?:shop|puzzles|collection)(?:\/|$)/.test(route)) return `/${lang}/login`;
  } else {
    if (route === 'account/security') return `/${lang}/password/change`;
    if (route === 'account/privacy/export') return `/${lang}/account/sudo?return_to=export`;
    if (route === 'register') return '/account';
  }
}

export function projectionPaths(actor) {
  return [...publicViews.keys(), ...(actor === 'anonymous' ? [] : accountViews.keys())];
}

export function unavailableView(url, actor, locale) {
  const route = routeOf(url);
  if (url.search && !(route === 'account/journal/reports' && [...url.searchParams.keys()].join() === 'week')) return;
  const entry = publicViews.get(route) ?? (actor !== 'anonymous' && accountViews.get(route));
  if (!entry) return;
  const [selector, messages, reason, layout, filters = false] = entry;
  return { selector, text: messages[localeOf(url, locale) === 'en' ? 1 : 0], reason, layout, filters };
}
