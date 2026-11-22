const unavailable = ['Usługa chwilowo niedostępna', 'Temporarily unavailable'];
const coldCatalogue = ['Dane świata są obecnie niedostępne.', 'World data is currently unavailable.'];
const coldStats = ['Statystyki są niedostępne. Spróbuj ponownie później.', 'Statistics are unavailable. Please try again later.'];
const bazaar = [
  'Brak zatwierdzonego źródła uprawnień do depozytu i podpisu postaci. Wystawienie, licytacja i przeniesienie są niedostępne.',
  'Accepted character custody and signing authority are unavailable. No listing, bid or transfer can be accepted.',
];
const status = '[role="status"]';

const publicViews = new Map([
  ['atlas', ['h1', unavailable, 'No accepted Game atlas export']],
  ['atlas/delivery', [null, ['Niedostępne', 'Unavailable'], 'No published delivery capabilities']],
  ...['wiki', ...['search', 'items', 'creatures', 'npcs', 'spells', 'quests',
    'achievements', 'loot', 'areas'].map(kind => `wiki/${kind}`)]
    .map(route => [route, ['h1', coldCatalogue, 'No active Game catalogue']]),
  ['wiki/places', ['p', ['Brak zatwierdzonych danych.', 'No qualified data available.'], 'No approved places data']],
  ['houses', [status, ['Informacje o domach są niedostępne. Spróbuj ponownie później.', 'House information is unavailable. Please try again later.'], 'No house projection']],
  ['guilds', [status, ['Informacje o gildiach są niedostępne. Spróbuj ponownie później.', 'Guild information is unavailable. Please try again later.'], 'No guild projection']],
  ['online', [status, ['Tymczasowo niedostępne.', 'Temporarily unavailable.'], 'No online projection']],
  ['deaths', [status, ['Dane o zgonach są obecnie niedostępne.', 'Death records are currently unavailable.'], 'No death projection']],
  ['rankings/experience', [status, ['Tymczasowo niedostępne. Spróbuj ponownie później.', 'Temporarily unavailable. Please try again later.'], 'No ranking projection']],
  ['stats/guilds', [status, ['Niedostępne', 'Unavailable'], 'No guild statistics projection', true]],
  ...['yesterday', 'week', 'month', 'record', 'oldest', 'anniversaries', 'name-changes', 'transfers', 'sold']
    .map(kind => [`stats/rankings/${kind}`, [status, coldStats, 'No growth ranking projection', true]]),
  ['league', [status, ['Autorytatywne wyniki są obecnie niedostępne.', 'Authoritative results are currently unavailable.'], 'No League results publisher']],
  ['bazaar', ['p', bazaar, 'No accepted character custody/signing authority']],
  ...['bazaar/items', 'bazaar/items/new'].map(route => [route, ['h1', [
    'Handel przedmiotami jest tymczasowo niedostępny. Depozyty i zarezerwowane środki pozostają zabezpieczone.',
    'Item trading is temporarily unavailable. Deposits and reserved funds remain protected.',
  ], 'No accepted item custody and receipt authority']]),
]);

const accountViews = new Map([
  ...['account/bazaar', 'account/bazaar/sell']
    .map(route => [route, ['p', bazaar, 'No accepted character custody/signing authority']]),
  ['account/game-logins', ['h1', unavailable, 'No accepted Game login ownership and baseline authority']],
  ['account/dashboard', ['h1', unavailable, 'No closed-session and timer producers']],
  ['account/journal/reports', ['h1', unavailable, 'No accepted private Journal source']],
  ['atlas/places', ['h1', unavailable, 'No accepted places aggregate']],
  ['pass', [status, [
    'Przepustka sezonowa jest obecnie niedostępna. Nie pobrano coinów ani nie wydano nagród.',
    'The season pass is currently unavailable. No coins were charged and no rewards were issued.',
  ], 'No accepted season source or cosmetic authority']],
]);

export function localeOf(url, fallback) {
  return url.pathname.match(/^\/(pl|en)(?:\/|$)/)?.[1] ?? fallback;
}

export function routeOf(url) {
  return url.pathname.replace(/^\/(?:pl|en)(?=\/|$)/, '').replace(/^\//, '');
}

export function isPrivate(route) {
  return /^(?:account(?:\/|$)|panel\/|support\/(?:tickets|reports)(?:\/|$)|atlas\/places$|pass$)/.test(route)
    && route !== 'account/passkeys';
}

export function projectionPaths(actor) {
  return [...publicViews.keys(), ...(actor === 'anonymous' ? [] : accountViews.keys())];
}

export function unavailableView(url, actor, locale) {
  const route = routeOf(url);
  if (url.search && !(route === 'account/journal/reports' && [...url.searchParams.keys()].join() === 'week')) return;
  const entry = publicViews.get(route) ?? (actor !== 'anonymous' && accountViews.get(route));
  if (!entry) return;
  const [selector, messages, reason, filters = false] = entry;
  return { selector, text: messages[localeOf(url, locale) === 'en' ? 1 : 0], reason, filters };
}
