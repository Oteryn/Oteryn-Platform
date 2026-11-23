const status = (message) => `<p role="status">${message}</p>`;

// Match the known public read views and their localized refusal copy.
const views = {
  wiki: [
    ['Dane świata są obecnie niedostępne.'],
    ['World data is currently unavailable.'],
  ],
  bazaar: [
    ['Aukcje postaci są niedostępne', '<p>Brak zatwierdzonego źródła uprawnień do depozytu i podpisu postaci. Wystawienie, licytacja i przeniesienie są niedostępne.</p>'],
    ['Character auctions unavailable', '<p>Accepted character custody and signing authority are unavailable. No listing, bid or transfer can be accepted.</p>'],
  ],
  league: [
    ['Wydarzenia świata i sezony gildii', status('Autorytatywne wyniki są obecnie niedostępne.')],
    ['World events and guild seasons', status('Authoritative results are currently unavailable.')],
  ],
  online: [
    ['Postacie online', status('Tymczasowo niedostępne.')],
    ['Online characters', status('Temporarily unavailable.')],
  ],
  'rankings/experience': [
    ['Rankingi', status('Tymczasowo niedostępne. Spróbuj ponownie później.')],
    ['Rankings', status('Temporarily unavailable. Please try again later.')],
  ],
  'stats/guilds': [
    ['Statystyki gildii', status('Niedostępne')],
    ['Guild statistics', status('Unavailable')],
  ],
  'stats/rankings/yesterday': [
    ['Rankingi przyrostu', status('Statystyki są niedostępne. Spróbuj ponownie później.')],
    ['Growth rankings', status('Statistics are unavailable. Please try again later.')],
  ],
  'stats/world': [
    ['Statystyki świata', status('Niedostępne')],
    ['World statistics', status('Unavailable')],
  ],
};

export function isGameUnavailable(url, { status, headers, body }) {
  if (status !== 503 || headers.get('x-oteryn-game-data') !== 'unavailable' ||
      !/^text\/html(?:\s*;|$)/i.test(headers.get('content-type') || '') ||
      !headers.get('cache-control')?.split(',').some(value => value.trim().toLowerCase() === 'no-store')) {
    return false;
  }

  const [, lang, path] = url.pathname.match(/^\/(pl|en)\/(.+?)\/?$/) || [];
  const view = path === 'wiki/creatures' ? views.wiki : Object.hasOwn(views, path) ? views[path] : undefined;
  if (!view) return false;

  const contentLanguage = headers.get('content-language');
  if (contentLanguage && contentLanguage !== lang) return false;
  const nonContent = /<!--[\s\S]*?(?:-->|$)|<(script|style|template)\b[^<>]*>[\s\S]*?(?:<\/\1\s*>|$)/gi;
  let html = body.toString('utf8');
  let previous;
  // Removing one span can join fragments into another non-content span.
  do {
    previous = html;
    html = previous.replace(nonContent, '');
  } while (html !== previous);
  const documentTag = html.match(/<html\b[^<>]*>/i)?.[0];
  const language = documentTag?.match(/\slang="([^"]+)"/i)?.[1];
  const mainTag = /<main\b[^<>]*>/i.exec(html);
  if (language !== lang || !mainTag || !/\sid="main"/i.test(mainTag[0])) return false;
  const content = html.slice(mainTag.index + mainTag[0].length);
  const close = content.search(/<\/main>/i);
  if (close === -1) return false;
  const main = content.slice(0, close);

  const [heading, ...markers] = view[lang === 'pl' ? 0 : 1];
  return main.includes(`<h1>${heading}</h1>`) && markers.every(marker => main.includes(marker));
}
