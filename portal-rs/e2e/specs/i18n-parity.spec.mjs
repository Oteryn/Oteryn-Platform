import fs from 'node:fs';
import https from 'node:https';
import path from 'node:path';
import * as OTPAuth from 'otpauth';
import { test, expect } from '../fixtures.mjs';
import { login, pool, register, seedPasswordRange, submit, waitForWikiBudget } from '../helpers.mjs';
import { catalogCommand, importGameCatalog, uuid7 } from '../staff-commerce-helpers.mjs';
import { action, isolatedAdmin, loginWithRecoveryCode, player, query, unique } from '../staff-rbac-helpers.mjs';

const read = (path) => fs.readFileSync(new URL(path, import.meta.url), 'utf8');
const inventory = JSON.parse(read('../inventory.json'));
const expectations = JSON.parse(read('../route-expectations.json'));
const portalOrigin = new URL(process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443');
const workspace = new URL('../../', import.meta.url);
const htmlDocument = /<html(?:\s|>)/iu;
const announcementsPath = '/admin/announcements';
const announcementEditor = `${announcementsPath}/create`;
const announcementEdit = /^\/admin\/announcements\/[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}\/edit$/u;
const normalize = (value) => value.normalize('NFC').replace(/\s+/gu, ' ').trim();
const catalogs = Object.fromEntries(['pl', 'en'].map((locale) => [locale, new Map()]));
const catalogKeys = new Set();

function readCatalogs(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const url = new URL(entry.name + (entry.isDirectory() ? '/' : ''), directory);
    if (entry.isDirectory()) {
      readCatalogs(url);
      continue;
    }
    const locale = /(?:^|[._])(pl|en)\.ftl$/u.exec(entry.name)?.[1];
    if (!locale) continue;
    for (const [, key, text] of fs.readFileSync(url, 'utf8').matchAll(/^([a-z][\w-]*)\s*=[ \t]*([^\r\n]*)/gmu)) {
      catalogKeys.add(key);
      const value = normalize(text);
      if (value && !/[{}]/u.test(value)) {
        const names = catalogs[locale].get(value) ?? [];
        names.push(key);
        catalogs[locale].set(value, names);
      }
    }
  }
}
for (const path of ['crates/', 'templates/']) readCatalogs(new URL(path, workspace));

// A translation can legitimately contain a word that is a key in another
// catalog. Approve the exact rendered prose on its own route.
const signInProse = new Map([
  ['/en/account/passkeys', 'Adding or removing a passkey signs out all browser sessions. Your password remains available for sign-in.'],
  ['/en/account/passkeys/manage', 'Adding or removing a passkey signs out all browser sessions. Your password remains available for sign-in.'],
  ['/en/legal/cookies', 'You can delete or block cookies in your browser settings. Blocking the session cookie may prevent you from signing in and submitting forms. Deleting the other cookies clears the saved language or means you have to verify your sign-in again.'],
  ['/en/legal/terms', "Keep your details current, protect your sign-in credentials and report suspected account compromise. Do not share, sell or buy accounts. Minors may use the service subject to the rules on legal capacity that apply to them. Obtain a legal representative's consent before entering into an agreement or making a purchase whenever the law requires it."],
  ['/oauth/authorize', 'Authorize game sign-in'],
]);

const searchRoute = /^\/(?:(?:pl|en)\/)?search$/u;
const searchCopy = { pl: 'Szukaj', en: 'Search' };

function rawKeysIn(text, pathname) {
  if (text === 'search' && searchRoute.test(pathname) && catalogKeys.has(text)) return [text];
  const tokens = text.match(/(?<![\w/-])[a-z][\w]*(?:-[\w]+)+(?![\w/-])/gu) ?? [];
  return tokens.filter((token) =>
    catalogKeys.has(token) && !(token === 'sign-in' && text === signInProse.get(pathname)));
}

// Exact labels and longer diagnostic phrases are deliberately chosen to avoid
// flagging words shared by both languages, proper names or editorial content.
const markerKeys = new Set([
  'skip', 'shell-register', 'shell-login', 'shell-logout',
  'login-error', 'reset-error', 'mfa-error', 'error-403-description',
  'error-404-description', 'error-422-description', 'error-503-description',
  'announcements-heading', 'announcements-save', 'coin-checkout-title', 'refused', 'error', 'invalid',
]);
const opposite = (locale) => locale === 'en' ? 'pl' : 'en';
const markers = Object.fromEntries(['pl', 'en'].map((locale) => {
  const unique = [...catalogs[opposite(locale)]]
    .filter(([value]) => !catalogs[locale].has(value))
    .flatMap(([value, names]) => names.map((name) => [value, name]));
  return [locale, {
    body: unique.filter(([, key]) => markerKeys.has(key)),
    title: new Map(unique.filter(([, key]) => /(?:^|-)title(?:-|$)/u.test(key) || ['announcements-heading', 'home-link'].includes(key))),
  }];
}));

function wrongLocale(value, locale, title = false) {
  const text = normalize(value);
  if (title) {
    for (const part of text.split(/ [·|–—] /u)) {
      if (markers[locale].title.has(part)) return markers[locale].title.get(part);
    }
  }
  return markers[locale].body.find(([marker]) =>
    text === marker || (/[.!?]$/u.test(marker) && text.includes(marker)))?.[1];
}

function catalogText(path, key) {
  const source = fs.readFileSync(new URL(path, workspace), 'utf8');
  const value = source.split(/\r?\n/u).find((line) => line.startsWith(`${key} = `))?.slice(key.length + 3);
  if (!value) throw new Error(`Missing catalog marker ${key} in ${path}`);
  return value;
}

function assertText(value, locale, scope, pathname, title = false) {
  const text = normalize(value);
  expect.soft(rawKeysIn(text, pathname), `${scope}: rendered Fluent keys`).toEqual([]);
  expect.soft(/\{\{\s*[\w.]|\{\s*\$[\w-]+/u.test(text), `${scope}: unresolved interpolation`).toBe(false);
  expect.soft(wrongLocale(text, locale, title), `${scope}: opposite-locale catalog marker`).toBeUndefined();
}

async function assertDocument(page, locale, target, actor = 'anonymous') {
  expect.soft(await page.locator('html').getAttribute('lang'), `${target}: document language`).toBe(locale);
  const title = await page.title();
  const titleLabel = normalize(title).replace(/ ?[·|–—] ?Oteryn$/u, '').trim();
  expect.soft(titleLabel.length, `${target}: localized document title`).toBeGreaterThan(0);
  const pathname = new URL(page.url()).pathname;
  assertText(titleLabel, locale, `${target}: title`, pathname, true);
  // Text nodes preserve boundaries between labels and include closed menus.
  const values = await page.locator('body').evaluate((body, locale) => {
    const values = [];
    const append = (element, value, label = value) => {
      const declaredLanguage = element.closest('[lang]:not(html), [hreflang]');
      const declaredLocale = declaredLanguage?.getAttribute('lang') ?? declaredLanguage?.getAttribute('hreflang');
      values.push({ value, label, checkLocale: !declaredLocale || declaredLocale === locale });
    };
    const walker = document.createTreeWalker(body, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      if (node.textContent.trim() && !node.parentElement.closest('script, style, template, noscript, code, pre, textarea')) {
        append(node.parentElement, node.textContent, node.parentElement.textContent);
      }
    }
    for (const element of body.querySelectorAll('[aria-label], [alt], [title], [placeholder], input[type=submit]')) {
      for (const attribute of ['aria-label', 'alt', 'title', 'placeholder']) {
        if (element.hasAttribute(attribute)) append(element, element.getAttribute(attribute));
      }
      if (element.matches('input[type=submit]')) append(element, element.value);
    }
    return values;
  }, locale);
  const rawKeys = new Set();
  const leaks = new Set();
  let interpolation = false;
  for (const { value, label, checkLocale } of values) {
    const text = normalize(value);
    const wholeLabel = normalize(label);
    for (const token of [...rawKeysIn(text, pathname), ...rawKeysIn(wholeLabel, pathname)]) rawKeys.add(token);
    const leak = checkLocale && wrongLocale(text, locale);
    if (leak) leaks.add(leak);
    interpolation ||= /\{\{\s*[\w.]|\{\s*\$[\w-]+/u.test(text);
  }
  expect.soft([...rawKeys], `${target}: rendered Fluent keys`).toEqual([]);
  expect.soft([...leaks], `${target}: opposite-locale catalog markers`).toEqual([]);
  expect.soft(interpolation, `${target}: unresolved interpolation`).toBe(false);
  await assertShellCopy(page, locale, target, actor);
}

// The shell and checkout expectations are pinned independently of the catalogs.
const shellCopy = {
  "shell-mfa": ["Uwierzytelnianie dwuskładnikowe", "Two-factor authentication"],
  "shell-sudo": ["Potwierdź hasło", "Confirm your password"],
  "shell-email": ["Zmień adres e-mail", "Change email address"],
  "shell-export": ["Pobierz dane konta", "Download account data"],
  "shell-termination": ["Zamknij konto", "Close account"],
  "shell-appeals": ["Odwołania od sankcji", "Sanction appeals"],
  "home-link": ["Strona główna", "Home"],
  "navigation": ["Nawigacja główna", "Main navigation"],
  "panel-admin": ["Panel administracyjny", "Staff dashboard"],
  "shell-abuse-reports": ["Zgłoszenia naruszeń", "Abuse reports"],
  "shell-account-bazaar": ["Moje aukcje", "My auctions"],
  "shell-account-notifications": ["Powiadomienia", "Notifications"],
  "shell-account-overview": ["Przegląd konta", "Account overview"],
  "shell-account-passkeys": ["Klucze dostępu", "Passkeys"],
  "shell-account-privacy": ["Prywatność konta", "Account privacy"],
  "shell-account-recovery-key": ["Klucz odzyskiwania", "Recovery key"],
  "shell-account-sessions": ["Sesje konta", "Account sessions"],
  "shell-account-wizard": ["Kreator bezpieczeństwa", "Security wizard"],
  "shell-atlas": ["Atlas", "Atlas"],
  "shell-bazaar": ["Bazar postaci", "Character bazaar"],
  "shell-calculators": ["Kalkulatory", "Calculators"],
  "shell-collection": ["Kolekcja", "Collection"],
  "shell-community": ["Społeczność", "Community"],
  "shell-compare": ["Porównaj postacie", "Compare characters"],
  "shell-create-character": ["Utwórz postać", "Create a character"],
  "shell-deaths": ["Ostatnie zgony", "Recent deaths"],
  "shell-delivery": ["Trasy dostaw", "Delivery routes"],
  "shell-events": ["Wydarzenia", "Events"],
  "shell-growth-rankings": ["Rankingi rozwoju", "Growth rankings"],
  "shell-guild-stats": ["Statystyki gildii", "Guild statistics"],
  "shell-guilds": ["Gildie", "Guilds"],
  "shell-hirelings": ["Pomocnicy", "Hirelings"],
  "shell-houses": ["Domy", "Houses"],
  "shell-hunts": ["Polowania", "Hunts"],
  "shell-league": ["Rozgrywki", "Leagues"],
  "shell-logout": ["Wyloguj", "Sign out"],
  "shell-menu": ["Menu", "Menu"],
  "shell-news": ["Aktualności", "News"],
  "shell-online": ["Gracze online", "Online players"],
  "shell-party": ["Znajdź drużynę", "Find a party"],
  "shell-pass": ["Przepustka sezonowa", "Season pass"],
  "shell-payments": ["Płatności", "Payments"],
  "shell-places": ["Miejsca", "Places"],
  "shell-premium": ["Premium", "Premium"],
  "shell-privacy": ["Polityka prywatności", "Privacy policy"],
  "shell-puzzles": ["Codzienne zagadki", "Daily puzzles"],
  "shell-reports": ["Raporty dziennika", "Journal reports"],
  "shell-rules": ["Zasady gry", "Game rules"],
  "shell-server": ["Informacje o serwerze", "Server information"],
  "shell-shop": ["Sklep", "Shop"],
  "shell-support": ["Pomoc", "Support"],
  "shell-terms": ["Regulamin", "Terms of service"],
  "shell-tickets": ["Zgłoszenia pomocy", "Support tickets"],
  "shell-wiki": ["Wiki", "Wiki"],
  "shell-world-stats": ["Statystyki świata", "World statistics"],
  "shell-worlds": ["Światy", "Worlds"],
  "skip": ["Przejdź do treści", "Skip to content"],
  "stage-login": ["Zaloguj się", "Log in"],
  "stage-my-account": ["Moje konto", "My account"],
  "thais-account-close": ["Zamknij menu konta", "Close account menu"],
  "thais-account-game-logins": ["Historia logowań do gry", "Game login history"],
  "thais-account-nav": ["Nawigacja konta", "Account navigation"],
  "thais-account-password": ["Zmień hasło", "Change password"],
  "thais-account-play": ["Postacie i gra", "Characters and gameplay"],
  "thais-account-requests": ["Moje zgłoszenia", "My requests"],
  "thais-account-security": ["Bezpieczeństwo", "Security"],
  "thais-account-settings": ["Ustawienia", "Settings"],
  "thais-foot-discord-unavailable": ["Discord · zaproszenie niedostępne", "Discord · invitation unavailable"],
  "thais-foot-external": ["Serwisy zewnętrzne", "External resources"],
  "thais-foot-language": ["Język portalu", "Portal language"],
  "thais-foot-legal": ["Informacje prawne", "Legal information"],
  "thais-foot-links": ["Nawigacja w stopce", "Footer navigation"],
  "thais-foot-player-help": ["Pomoc dla graczy", "Player support"],
  "thais-foot-report": ["Zgłoś błąd", "Report a bug"],
  "thais-foot-twitch": ["Twitch · kategoria Tibia", "Twitch · Tibia category"],
  "thais-home-today": ["Dzisiaj w Oterynie", "Today in Oteryn"],
  "thais-nav-activities": ["Aktywności", "Activities"],
  "thais-nav-bestiary": ["Bestiariusz", "Bestiary"],
  "thais-nav-experience": ["Ranking doświadczenia", "Experience ranking"],
  "thais-nav-explore": ["Poznaj świat", "Explore the world"],
  "thais-nav-players": ["Gracze", "Players"],
  "thais-nav-professions": ["Profesje", "Vocations"],
  "thais-nav-rankings": ["Rankingi i porównania", "Rankings and comparisons"],
  "thais-nav-start": ["Jak zacząć", "Getting started"],
  "thais-nav-world": ["Świat gry", "Game world"],
  "worlds-title": ["Status światów", "World status"],
};

const shellLinkKeys = new Map(inventory.sections.map(({ path, label }) => ['/' + path.replace(/^\//u, ''), label]));
for (const [path, key] of Object.entries({
  '/mfa': 'shell-mfa',
  '/account/sudo': 'shell-sudo',
  '/account/security/email': 'shell-email',
  '/account/privacy/export': 'shell-export',
  '/account/privacy/delete': 'shell-termination',
  '/account/game-logins': 'thais-account-game-logins',
  '/admin': 'panel-admin',
  '/help': 'shell-support',
  '/password/change': 'thais-account-password',
  '/rankings/experience': 'thais-nav-experience',
  '/support/appeals': 'shell-appeals',
  '/support/report-a-bug': 'thais-foot-report',
  '/today': 'thais-home-today',
  '/wiki/creatures': 'thais-nav-bestiary',
})) shellLinkKeys.set(path, key);
const shellFragmentKeys = new Map([
  ['#bestiariusz', 'thais-nav-bestiary'],
  ['#jak-zaczac', 'thais-nav-start'],
  ['#profesje', 'thais-nav-professions'],
  ['#status', 'worlds-title'],
]);

function shellHeading(selector, routes, heading = 'h2') {
  const anchors = routes.flatMap((route) => ['', '/pl', '/en'].map((prefix) => 'a[href="' + prefix + route + '"]'));
  return selector + ':has(' + anchors.join(', ') + ') ' + heading;
}

// Each entry names a stable element and the authored text or accessible label
// it exposes. Optional controls depend on the mounted modules and session.
const interiorWorld = '.om-section:has(#portal-world-panel) > summary';
const interiorCommunity = '.om-section:has(#portal-community-panel) > summary';
const shellControls = [
  ['a.skip[href="#main"]', 'skip'],
  ['#portal-menu-toggle span, header#portal-header .om-fallback-menu > summary', 'shell-menu'],
  ['#portal-navigation', 'navigation', 'aria-label'],
  [`#portal-world-button .om-label, ${interiorWorld} .om-label, footer.foot .foot-links .foot-group:first-child h2`, 'thais-nav-world'],
  [`#portal-community-button .om-label, ${interiorCommunity} .om-label, footer.foot .foot-links .foot-group:has(a[href$="/news"]) h2`, 'shell-community'],
  ['#portal-world-panel .om-column:nth-child(1) h2', 'thais-nav-explore'],
  ['#portal-world-panel .om-column:nth-child(2) h2', 'thais-nav-activities'],
  [shellHeading('#portal-community-panel .om-column', ['/online', '/party', '/bazaar', '/guilds', '/deaths']), 'thais-nav-players'],
  [shellHeading('#portal-community-panel .om-column', ['/rankings/experience', '/stats/compare', '/stats/guilds', '/stats/rankings/yesterday']), 'thais-nav-rankings'],
  ['#account-heading, header#portal-header .om-fallback-account > summary', 'stage-my-account'],
  ['#podglad-konta .account-close', 'thais-account-close', 'aria-label'],
  ['#podglad-konta .account-grid, header#portal-header .account-grid', 'thais-account-nav', 'aria-label'],
  [shellHeading(':is(#podglad-konta, #portal-header) .account-grid section', ['/account/characters/new', '/panel/hirelings', '/account/hunts', '/account/journal/reports', '/account/bazaar'], 'h3'), 'thais-account-play'],
  [shellHeading(':is(#podglad-konta, #portal-header) .account-grid section', ['/password/change', '/account/security/wizard', '/account/passkeys', '/account/recovery-key', '/account/sessions', '/account/game-logins'], 'h3'), 'thais-account-security'],
  [shellHeading(':is(#podglad-konta, #portal-header) .account-grid section', ['/account/privacy', '/account/notifications', '/panel/premium', '/panel/payments'], 'h3'), 'thais-account-settings'],
  [shellHeading(':is(#podglad-konta, #portal-header) .account-grid section', ['/support/tickets', '/support/reports'], 'h3'), 'thais-account-requests'],
  ['#podglad-konta form[action$="/logout"] button, header#portal-header form[action$="/logout"] button', 'shell-logout'],
  ['footer.foot .foot-links', 'thais-foot-links', 'aria-label'],
  ['footer.foot .foot-social', 'thais-foot-external', 'aria-label'],
  ['footer.foot .foot-community-status', 'thais-foot-discord-unavailable'],
  [shellHeading('footer.foot .foot-links .foot-group', ['/help']), 'thais-foot-player-help'],
  ['footer.foot .foot-legal', 'thais-foot-legal', 'aria-label'],
  ['footer.foot .foot-langs', 'thais-foot-language', 'aria-label'],
];

async function assertShellCopy(page, locale, target, actor) {
  const path = new URL(page.url()).pathname;
  if (path === `/static/thais/shop-${locale}.html`) {
    const labels = locale === 'pl'
      ? ['Nawigacja sklepu', 'Strona główna', 'Aktualności', 'Zaloguj się', 'English', 'Regulamin', 'Polityka prywatności']
      : ['Shop navigation', 'Home', 'News', 'Log in', 'Polski', 'Terms of service', 'Privacy policy'];
    await expect(page.locator('a.skip[href="#main"]')).toHaveCount(1);
    await expect(page.locator('header .brand')).toHaveAttribute('href', `/${locale}/`);
    const navigation = page.getByRole('navigation', { name: labels[0], exact: true });
    await expect(navigation.getByRole('link')).toHaveText(labels.slice(1, 5));
    await expect(navigation.getByRole('link', { name: labels[4], exact: true }))
      .toHaveAttribute('hreflang', opposite(locale));
    await expect(page.locator('footer .foot-legal a')).toHaveText(labels.slice(5));
    await expect(page.locator('footer .foot-legal a').first()).toHaveAttribute('href', `/${locale}/legal/terms`);
    await expect(page.locator('footer .foot-legal a').last()).toHaveAttribute('href', `/${locale}/legal/privacy`);
    return;
  }
  const home = /^\/(?:pl|en)\/?$/u.test(new URL(page.url()).pathname);
  const mobile = page.viewportSize().width <= 960;
  const shell = await page.evaluate(({ controls, home, mobile, signedIn, interiorWorld, interiorCommunity }) => {
    const landmarks = [
      'a.skip[href="#main"]',
      'header#portal-header',
      'header#portal-header a.om-brand',
      'header#portal-header nav#portal-navigation',
      home ? '#portal-navigation #portal-world-button' : interiorWorld,
      home ? '#portal-navigation #portal-community-button' : interiorCommunity,
      '#portal-navigation .om-shop',
      'footer.foot',
      'footer.foot nav.foot-links',
      'footer.foot nav.foot-legal',
      'footer.foot nav.foot-langs a[hreflang="pl"]',
      'footer.foot nav.foot-langs a[hreflang="en"]',
      signedIn ? '#portal-navigation a.om-login[data-account-link]' : '#portal-navigation a.om-login[href$="/login"]',
    ];
    if (signedIn) landmarks.push('dialog#podglad-konta', '#podglad-konta .account-close', '#podglad-konta form[action$="/logout"] button');
    const expectedControl = mobile
      ? (home ? '#portal-menu-toggle[aria-controls="portal-navigation"]' : 'header#portal-header .om-fallback-menu > summary')
      : (home ? '#portal-navigation #portal-world-button' : interiorWorld);
    const missing = landmarks.filter((selector) => document.querySelectorAll(selector).length !== 1);
    if (!document.querySelector(expectedControl)?.checkVisibility()) missing.push(expectedControl + ' (visible)');
    const captions = controls.flatMap(([selector, key, attribute]) =>
      [...document.querySelectorAll(selector)].map((node) =>
        [key, attribute ? node.getAttribute(attribute) : node.textContent]));
    const links = [...document.querySelectorAll('header#portal-header a[href], #podglad-konta a[href], footer.foot a[href]')].map((link) => {
      const { pathname, hash } = new URL(link.href);
      let key;
      let text = link.textContent;
      if (link.matches('.brand')) {
        key = 'home-link';
        text = link.getAttribute('aria-label');
      } else if (link.matches('.om-shop')) key = 'shell-shop';
      else if (link.matches('.om-login')) key = link.hasAttribute('data-account-link') ? 'stage-my-account' : 'stage-login';
      else if (link.matches('.om-mobile-only')) key = 'worlds-title';
      else if (link.closest('.foot-social')) key = 'thais-foot-twitch';
      return { pathname, hash, key, text, language: link.getAttribute('hreflang'), legal: Boolean(link.closest('footer.foot .foot-legal')) };
    });
    return { missing, captions, links };
  }, { controls: shellControls, home, mobile, signedIn: actor !== 'anonymous', interiorWorld, interiorCommunity });
  expect.soft(shell.missing, target + ': shared shell controls for ' + actor).toEqual([]);
  const caption = (key) => shellCopy[key]?.[locale === 'pl' ? 0 : 1];
  const errors = [];
  const compare = (where, actual, expected) => {
    if (expected === undefined || normalize(actual ?? '') !== expected) errors.push({ where, actual, expected });
  };
  for (const [key, text] of shell.captions) compare(key, text, caption(key));
  const legalLinks = new Set();
  for (const { pathname, hash, key, text, language, legal } of shell.links) {
    const route = pathname.replace(/^\/(?:pl|en)(?=\/|$)/u, '');
    const label = key ?? (hash ? shellFragmentKeys.get(hash) : shellLinkKeys.get(route));
    const expected = language?.toUpperCase() ?? (label === 'home-link' ? 'Oteryn · ' + caption(label) : caption(label));
    compare(pathname + hash, text, expected);
    if (legal) legalLinks.add(route);
  }
  expect.soft([...legalLinks].sort(), target + ': shared legal links').toEqual(['/legal/privacy', '/legal/terms']);
  expect.soft(errors, target + ': shared navigation and footer copy').toEqual([]);
}

async function assertRouteResponse(response, target, contract = {}) {
  const { status = 200, pathname = target, mediaType = 'text/html', document = true } = contract;
  expect(response?.status(), `${target}: response status`).toBe(status);
  const final = new URL(response.url());
  expect(final.origin, `${target}: final origin`).toBe(portalOrigin.origin);
  expect(final.pathname, `${target}: final path`).toBe(pathname);
  expect(response.headers()['content-type']?.split(';')[0] ?? '', `${target}: content type`).toBe(mediaType);
  if (mediaType === 'text/html') {
    expect(htmlDocument.test(await response.text()), `${target}: HTML ${document ? 'document' : 'fragment'}`).toBe(document);
  }
}

async function assertProfileControls(page, locale) {
  const language = locale === 'pl' ? 0 : 1;
  const form = page.locator('main form');
  const labels = {
    show_guild: ['Pokaż gildię, jeśli gra na to pozwala', 'Show guild when the game permits it'],
    is_main_character: ['Wybierz jako postać główną', 'Select as main character'],
    hidden: ['Ukryj postać', 'Hide character'],
    hide_all_characters: ['Ukryj wszystkie moje postacie', 'Hide all my characters'],
    show_account: ['Pokaż powiązanie z kontem', 'Show account association'],
    show_status: ['Pokaż status', 'Show status'],
    show_house: ['Pokaż dom', 'Show house'],
    show_skills: ['Pokaż umiejętności', 'Show skills'],
    show_deaths: ['Pokaż zgony', 'Show deaths'],
    show_kills: ['Pokaż zabójstwa', 'Show kills'],
    show_online_time: ['Pokaż czas online', 'Show online time'],
    comment: ['Komentarz publiczny (500 znaków)', 'Public comment (500 characters)'],
  };
  for (const [field, copy] of Object.entries(labels)) {
    await expect(form.locator(`label[for="${field}"]`)).toHaveText(copy[language]);
  }
  await expect(form.locator('button[type=submit]')).toHaveText(locale === 'pl' ? 'Zapisz ustawienia' : 'Save settings');
}

const signInCopy = {
  pl: {
    email: 'Adres e-mail',
    password: 'Obecne hasło',
    submit: 'Zaloguj się',
    error: 'Nie udało się zweryfikować żądania. Sprawdź dane i spróbuj ponownie.',
  },
  en: {
    email: 'Email address',
    password: 'Current password',
    submit: 'Sign in',
    error: 'The request could not be verified. Check your details and try again.',
  },
};

async function assertSignInControls(page, locale) {
  const copy = signInCopy[locale];
  const form = page.locator('main form[action$="/login"]');
  await expect(form.locator('label[for=email]')).toHaveText(copy.email);
  await expect(form.locator('label[for=password]')).toHaveText(copy.password);
  await expect(form.getByRole('button')).toHaveText(copy.submit);
  for (const field of ['email', 'password']) {
    await expect(form.locator(`input[name="${field}"]`), `sign-in requires ${field}`).toHaveJSProperty('required', true);
  }
  const blankSubmissions = await form.evaluate((form) => {
    const email = form.elements.namedItem('email');
    const password = form.elements.namedItem('password');
    const button = form.querySelector('button');
    const original = [email.value, password.value];
    try {
      return [email, password].map((input) => {
        email.value = 'parity@example.invalid';
        password.value = 'Local validation fixture';
        input.value = '';
        const other = input === email ? password : email;
        let invalid = false;
        const onInvalid = () => { invalid = true; };
        // Only exercise the browser submission when the blank field blocks it.
        if (!form.noValidate && !button.formNoValidate && input.validity.valueMissing && other.validity.valid) {
          input.addEventListener('invalid', onInvalid);
          try {
            HTMLFormElement.prototype.requestSubmit.call(form, button);
          } finally {
            input.removeEventListener('invalid', onInvalid);
          }
        }
        return { field: input.name, invalid, message: input.validationMessage };
      });
    } finally {
      [email.value, password.value] = original;
    }
  });
  for (const { field, invalid, message } of blankSubmissions) {
    expect(invalid, `blank sign-in ${field} blocks submission`).toBe(true);
    expect(message, `blank sign-in ${field} has a native validation message`).not.toBe('');
  }
}

function sample(route, locale) {
  return route
    .replace(/\{(?:lang|locale|language)\}/gu, locale)
    .replaceAll('{kind}', route.includes('stats/rankings') ? 'yesterday' : route.includes('rankings') ? 'experience' : 'items')
    .replaceAll('{tab}', 'experience')
    .replaceAll('{slug}', route.startsWith('/admin/pages/') ? 'rules' : 'missing-e2e')
    .replace(/\{(?:floor|zoom|x|y)\}/gu, '0')
    .replace(/\{(?:name|key|family|channel|filename|\*target|\*rest)\}/gu, 'missing-e2e')
    .replace(/\{[^{}]+\}/gu, '019a541b-a000-7000-8000-000000000001');
}

// Inventory statuses describe the authenticated actors. Login redirects and
// public protocol refusals have their own expectations for the anonymous crawl.
const signInRequired = /^\/(?:account(?:$|\/(?!passkeys$))|mfa(?:\/challenge)?$|atlas\/places$|(?:panel|collection|collection-pvp|pass|puzzles|shop)(?:\/|$)|support\/(?:reports|tickets)(?:\/|$))/u;
const fragments = new Set(['/announcements/ticker', '/today/widget']);
// Requests for unavailable assets that negotiate an HTML refusal keep the
// document contract. This list covers resources served outside the HTML shell.
const resources = new Map([
  ['/admin/analytics/data', 'application/json'],
  ['/admin/operations/rate-limits.csv', 'text/csv'],
  ['/admin/analytics/creatures/data', 'application/json'],
  ['/admin/analytics/activity/data', 'application/json'],
  ['/admin/analytics/levels/data', 'application/json'],
  ['/admin/analytics/health/data', 'application/json'],
  ['/admin/analytics/economy/data', 'application/json'],
  ['/api/v1/account/summary', ''],
  ['/api/v1/feed', ''],
  ['/api/v1/game-auth/native-characters', 'application/json'],
  ['/api/v1/stats/watches', ''],
  ['/api/v1/worlds', 'application/json'],
  ['/atlas/delivery.css', 'text/css'],
  ['/favicon.ico', 'image/svg+xml'],
  ['/healthz', 'text/plain'],
  ['/oauth/authorize', 'application/json'],
  ['/readyz', ''],
  ['/register/abuse.js', 'text/javascript'],
  ['/robots.txt', 'text/plain'],
  ['/sitemap.xml', 'application/xml'],
  ['/static/playerdata/hunts.css', 'text/css'],
]);
const playerJson = new Set(['/account/journal/reports/notes/export', '/atlas/places/participation', '/atlas/private-markers']);

function routeContract(target, inventoryStatus, actor, locale) {
  const [, prefix] = /^\/(en|pl)(?=\/|$)/u.exec(target) ?? [];
  const lang = prefix ?? locale;
  const route = prefix ? target.slice(prefix.length + 1) : target;
  const coldProjection = route === '/wiki/creatures' || /^\/characters\/[^/]+\/skills$/u.test(route);
  const missingFixture = /^\/(?:shop\/test-checkout\/[^/]+|account\/characters\/[^/]+\/(?:achievements|equipment|outfit))$/u.test(route);
  const missingInvite = /^\/referrals\/[^/]+$/u.test(route);
  let status = inventoryStatus ?? (coldProjection ? 503 : missingFixture ? 404 : missingInvite ? 403 : 200);
  let pathname = target;
  if (target === '/') pathname = `/${locale}/`;
  if (target === '/forgot-password' || /^\/events(?:\/|$)/u.test(target)) pathname = `/${locale}${target}`;
  if (actor === 'anonymous') {
    if (signInRequired.test(route) || route === '/password/change') {
      return { status: 200, pathname: route === '/password/change' ? '/login' : `/${lang}/login`, document: true };
    }
    if (route === '/atlas/private-markers' || route.startsWith('/atlas/bestiary/')) status = 401;
    if (route === '/atlas/places/participation' || route === '/bazaar/items/new' || route.startsWith('/support/appeals')) status = 403;
  }
  if (actor === 'player') {
    if (route === '/mfa/challenge') pathname = `/${lang}/`;
    if (route === '/register') pathname = '/account';
    if (route === '/account/security') pathname = `/${lang}/password/change`;
    if (route === '/account/privacy/export') pathname = `/${lang}/account/sudo`;
  }
  if (fragments.has(route)) return { status, pathname, document: false };
  const json = /^\/atlas\/spawns-text\/(?:pl|en)$/u.test(route) || (actor === 'player' && playerJson.has(route));
  const mediaType = json ? 'application/json' : resources.get(route);
  if (mediaType !== undefined) return { status, pathname, mediaType, document: false };
  return { status, pathname, document: true };
}

function destinations(actor, locale) {
  const targets = new Map();
  for (const row of expectations) {
    if (['gm', 'game_master_senior'].includes(row.actor)) continue;
    if (row.method !== 'GET' || (row.actor === 'staff') !== (actor === 'staff')) continue;
    targets.set(sample(row.route, locale), row.status);
  }
  for (const row of inventory.routes) {
    const staff = /^\/(?:\{lang\}\/)?(?:admin|console)(?:\/|$)/u.test(row.route);
    if (row.methods.includes('GET') && staff === (actor === 'staff')) {
      const path = sample(row.route, locale);
      if (!targets.has(path)) targets.set(path, undefined);
    }
  }
  if (actor === 'staff') {
    for (const path of [announcementsPath, announcementEditor, `${announcementsPath}/{id}/edit`]) {
      targets.set(sample(path, locale), path.endsWith('/edit') ? 404 : 200);
    }
  } else {
    targets.set(`/${locale}/wiki/creatures`, 503);
  }
  return targets;
}

async function addLinkedPages(page, targets, knownRoutes, locale, actor) {
  const paths = await page.locator('a[href]:not([hreflang], [download])').evaluateAll((links) =>
    links.flatMap((link) => {
      const href = link.getAttribute('href');
      if (!URL.canParse(href, link.baseURI)) return [];
      const url = new URL(href, link.baseURI);
      // Do not replay links carrying query parameters, fragments or signed tokens.
      return url.origin === location.origin && !url.search && !url.hash ? [url.pathname] : [];
    }));
  for (const path of paths) {
    if (new RegExp(`^/(?:admin/)?${opposite(locale)}(?:/|$)`, 'u').test(path)) continue;
    if (/^\/language\/|\/(?:confirm|revert|reset-password)\/[^/]+$/u.test(path)) continue;
    const staff = /^\/(?:pl\/|en\/)?(?:admin|console)(?:\/|$)/u.test(path);
    // Another group owns known routes. Newly discovered destinations stay
    // with the group that found them so rendered links still extend coverage.
    if (staff !== (actor === 'staff') || knownRoutes.has(path) || targets.has(path)) continue;
    targets.set(path, undefined);
  }
}

const localeQueryRoute = /^\/(?:news(?:\/[^/]+)?|today(?:\/(?:data|widget))?|announcements\/ticker|admin\/(?:news(?:\/.*)?|media|console\/(?:access|approvals|audit|players)))$/u;

const validationMessages = 'main [role=alert], main .op-error, main [aria-live]:not([aria-live=off])';

async function nativeValidation(page, locale, checked, uncovered) {
  const pathname = new URL(page.url()).pathname;
  const forms = page.locator('form');
  for (let index = 0; index < await forms.count(); index++) {
    const form = forms.nth(index);
    const { method, action, result } = await form.evaluate((form) => {
      const field = Array.from(form.elements).find((element) =>
        element.willValidate && (element.required || element.type === 'email') &&
        element.matches('input:not([type=hidden], [type=radio]), textarea, select') && element.getClientRects().length);
      const button = Array.from(form.elements).find((element) =>
        element.type === 'submit' && !element.disabled && !element.formNoValidate && element.getClientRects().length);
      // Named controls such as "action" shadow HTMLFormElement properties.
      const action = button?.getAttribute('formaction') ?? form.getAttribute('action') ?? '';
      const method = button?.getAttribute('formmethod') ?? form.getAttribute('method') ?? 'get';
      const details = { action: new URL(action || location.href, form.baseURI).pathname, method, result: null };
      if (!field || !button || form.noValidate) return details;
      const original = { value: field.value, checked: field.checked, selectedIndex: field.selectedIndex };
      if (field.type === 'checkbox') field.checked = false;
      else if (field.tagName === 'SELECT') field.selectedIndex = -1;
      else field.value = field.required ? '' : 'invalid-email';

      try {
        // Submit only if the real browser constraint already rejects the input.
        if (field.validity.valid) return details;
        let invalidEvent = false;
        field.addEventListener('invalid', () => { invalidEvent = true; }, { once: true });
        HTMLFormElement.prototype.requestSubmit.call(form, button);
        details.result = { invalidEvent, message: field.validationMessage, custom: field.validity.customError };
        return details;
      } finally {
        if (field.type === 'checkbox') field.checked = original.checked;
        else if (field.tagName === 'SELECT') field.selectedIndex = original.selectedIndex;
        else field.value = original.value;
      }
    });
    const name = `${method.toUpperCase()} ${action} (form ${index + 1})`;
    if (!result) {
      uncovered.add(name);
      continue;
    }
    expect.soft(result.invalidEvent, name).toBe(true);
    expect.soft(result.message.length, name).toBeGreaterThan(0);
    // The browser supplies native UI strings. Check authored custom messages
    // and DOM feedback against the portal's catalogs.
    if (result.custom) assertText(result.message, locale, name, pathname);
    for (const message of await page.locator(validationMessages).allInnerTexts()) {
      assertText(message, locale, name, pathname);
    }
    checked.add(name);
  }
}

function posted(page, path) {
  return page.waitForResponse((response) =>
    response.request().method() === 'POST' && new URL(response.url()).pathname === path);
}

// Keep the editorial expectations independent of the Fluent resources.
const announcementLabels = {
  heading: ["Ogłoszenia", "Announcements"],
  list: ["Lista", "List"],
  create: ["Utwórz ogłoszenie", "Create announcement"],
  instructions: ["Szkice są prywatne. Opublikowane ogłoszenia są publiczne. Daty wpisuj w formacie RFC3339 (UTC). Edytuj jednocześnie tekst angielski i polskie tłumaczenie. Aby zatwierdzić bieżące tłumaczenie, ustaw nową datę zatwierdzenia.", "Drafts are private. Published announcements are public. Enter dates in RFC3339 (UTC). Edit the English source and Polish translation together. Set a new approval timestamp to approve the current translation."],
  draft: ["Szkic", "Draft"],
  published: ["Opublikowane", "Published"],
  info: ["Informacja", "Information"],
  maintenance: ["Prace techniczne", "Maintenance"],
  warning: ["Ostrzeżenie", "Warning"],
  save: ["Zapisz ogłoszenie", "Save announcement"],
  publication_state: ["Publikacja", "Publication"],
  severity: ["Ważność", "Severity"],
  starts_at: ["Początek (UTC)", "Starts at (UTC)"],
  ends_at: ["Koniec (UTC, opcjonalny)", "Ends at (UTC, optional)"],
  title: ["Tytuł (angielski)", "Title (English)"],
  body: ["Treść (angielska)", "Body (English)"],
  action_label: ["Etykieta akcji (angielska)", "Action label (English)"],
  action_url: ["Ścieżka akcji", "Action path"],
  title_pl: ["Tytuł (polski)", "Title (Polish)"],
  body_pl: ["Treść (polska)", "Body (Polish)"],
  action_label_pl: ["Etykieta akcji (polska)", "Action label (Polish)"],
  translation_published_at: ["Data zatwierdzenia tłumaczenia (UTC)", "Translation approved at (UTC)"],
};
const announcementCopy = (locale, key) => announcementLabels[key][locale === 'pl' ? 0 : 1];

async function assertAnnouncements(page, locale, target) {
  const copy = (key) => announcementCopy(locale, key);
  const main = page.locator('main');
  await expect(page).toHaveTitle(copy('heading'));
  await expect(main.locator('h1')).toHaveText(copy('heading'));
  await expect(main.locator(`a[href="${announcementsPath}"]`)).toHaveText(copy('list'));
  await expect(main.locator(`a[href="${announcementEditor}"]`)).toHaveText(copy('create'));
  if (target !== announcementsPath) {
    const action = target === announcementEditor ? announcementsPath : target;
    const form = main.locator(`form[action="${action}"]`);
    await expect(form.locator('p')).toHaveText(copy('instructions'));
    for (const field of [
      'publication_state', 'severity', 'starts_at', 'ends_at', 'title', 'body', 'action_label',
      'action_url', 'title_pl', 'body_pl', 'action_label_pl', 'translation_published_at',
    ]) {
      await expect(form.locator(`label[for="${field}"]`)).toHaveText(copy(field));
    }
    await expect(form.locator('select[name=publication_state] option')).toHaveText(['draft', 'published'].map(copy));
    await expect(form.locator('select[name=severity] option')).toHaveText(['info', 'maintenance', 'warning'].map(copy));
    await expect(form.getByRole('button')).toHaveText(copy('save'));
  }
}

async function refusedForm(page, locale, form, fields, status, message) {
  const { url, data } = await form.evaluate(form => ({
    url: new URL(form.getAttribute('action') || document.location.href, form.baseURI).href,
    data: Object.fromEntries(new FormData(form)),
  }));
  expect(data._csrf, 'the rendered form supplies its own CSRF proof').toBeTruthy();
  const response = await page.request.post(url, {
    form: { ...data, ...fields },
    headers: { origin: new URL(url).origin, accept: 'text/html', 'accept-language': locale },
    maxRedirects: 0,
  });
  try {
    expect(response.status(), new URL(response.url()).pathname).toBe(status);
    expect(response.headers()['content-type']).toContain('text/html');
    const html = await response.text();
    expect(html).toContain(`<html lang="${locale}"`);
    expect(html).toContain(message ?? catalogText(`crates/ui/i18n/${locale}.ftl`, `error-${status}-description`));
  } finally {
    await response.dispose();
  }
}

async function assertPasswordRefusal(page, locale, actor) {
  await submit(page, 'main form', { current_password: 'Incorrect local password!' });
  await expect(page.locator('main [role=alert]')).toHaveText(
    catalogText(`crates/identity/src/credentials_${locale}.ftl`, 'refused'),
  );
  await assertDocument(page, locale, 'password confirmation refusal', actor);
}

async function serverValidation(page, locale, actor, target, targets) {
  const pathname = new URL(page.url()).pathname;
  const ui = `crates/ui/i18n/${locale}.ftl`;
  if ((actor === 'player' && pathname === `/${locale}/account/sudo`)
      || (actor === 'staff' && pathname === `/${locale}/admin/roles/confirm`)) {
    await assertPasswordRefusal(page, locale, actor);
  }
  if (actor === 'player' && pathname === `/${locale}/account/privacy/delete`) {
    await refusedForm(page, locale, page.locator('main form'), {
      current_password: 'Incorrect local password!', confirmation: 'NOT_TERMINATE',
    }, 422);
  }
  if (actor === 'staff' && pathname === '/admin/roles') {
    const roleForm = action => page.locator(`main form:has(button[value="${action}"])`);
    for (const [action, fields, status, description] of [
      ['create', { permissions: 'unknown.permission' }, 400],
      ['create-grant-role', { key: 'invalid-grant', reason: 'unknown_reason' }, 422, 'staff-action-approved-reason-required'],
      ['set-permissions', { permissions: 'unknown.permission' }, 400],
      ['grant', { account: 'not-an-account-id', reason: 'role_assignment' }, 422],
      ['revoke', { account: 'not-an-account-id' }, 422],
    ]) {
      await test.step(`refuse invalid role ${action}`, () => refusedForm(
        page, locale, roleForm(action), { action, ...fields }, status,
        description && catalogText(ui, description),
      ));
    }
  }
  if (actor === 'staff' && pathname === '/admin/commerce') {
    const invalid = catalogText(`crates/admin/src/commerce/${locale}.ftl`, 'invalid');
    for (const [action, fields] of [
      ['grant', { days: '400' }],
      ['revoke', { target: 'not-an-account-id' }],
      ['adjust', { amount: '0' }],
      ['refund', { target: 'not-a-payment-id' }],
    ]) {
      const form = page.locator(`main form[action="/admin/commerce/${action}"]`);
      await refusedForm(page, locale, form, {
        target: '019a541b-a000-7000-8000-000000000001',
        reason: 'Localized validation fixture',
        ...fields,
      }, 422, invalid);
    }
  }
  if (actor === 'staff' && announcementEdit.test(pathname)) {
    await refusedForm(page, locale, page.locator('main form'), { starts_at: 'invalid-date' }, 422);
  }
  if (actor === 'player' && target === `/${locale}/password/change`) {
    await submit(page, 'main form', {
      current_password: 'Incorrect local password!',
      password: 'Synthetic replacement password!',
      password_confirmation: 'Mismatched replacement password!',
    });
    await expect(page.locator('main [role=alert]')).toHaveText(
      catalogText(`crates/identity/src/credentials_${locale}.ftl`, 'refused'),
    );
    await assertDocument(page, locale, 'password change refusal', actor);
  }
  if (actor === 'player' && target === `/${locale}/support/reports/create`) {
    await submit(page, 'main form', { target_reference: ' ', evidence_summary: 'Parity fixture' });
    await expect(page.locator('main [role=alert]')).toHaveText(
      catalogText(`crates/support/src/reports/${locale}.ftl`, 'invalid'),
    );
    await assertDocument(page, locale, 'report validation refusal', actor);
  }
  if (actor === 'player' && target === `/${locale}/support/tickets/create`) {
    const response = posted(page, `/${locale}/support/tickets`);
    await submit(page, 'main form', { subject: ' ', body: 'Parity fixture' });
    expect((await response).status()).toBe(422);
    await expect(page.locator('main h1')).toHaveText(catalogText(ui, 'error-422-title'));
    await expect(page.locator('main h1 + p')).toHaveText(catalogText(ui, 'error-422-description'));
    await assertDocument(page, locale, 'ticket validation refusal', actor);
    expect((await page.goto(target)).status()).toBe(200);
    await submit(page, 'main form', { subject: 'Parity fixture', body: 'Synthetic support inquiry' });
    await expect(page).toHaveURL(/support\/tickets\/[a-f0-9-]+$/u);
    targets.set(new URL(page.url()).pathname, 200);
  }
  if (actor === 'staff' && target === announcementEditor) {
    const valid = { starts_at: new Date().toISOString(), title: 'Parity fixture', body: 'Parity fixture' };
    for (const [name, fields] of [
      ['all required fields blank', { starts_at: '', title: '', body: '' }],
      ['malformed start date', { ...valid, starts_at: 'invalid-date' }],
      ['blank start date with valid title and body', { ...valid, starts_at: '' }],
      ['blank title', { ...valid, title: '' }],
      ['blank body', { ...valid, body: '' }],
    ]) {
      await test.step(name, async () => {
        await assertRouteResponse(await page.goto(target), target);
        const [response] = await Promise.all([
          posted(page, announcementsPath),
          submit(page, 'main form', fields),
        ]);
        expect(response.status()).toBe(422);
        await expect(page.locator('main')).toContainText(catalogText(`crates/ui/i18n/${locale}.ftl`, 'error-422-description'));
        await assertDocument(page, locale, 'announcement validation refusal', actor);
      });
    }
    await test.step('save and reopen a maintenance draft', async () => {
      await assertRouteResponse(await page.goto(target), target);
      const selector = `main form[action="${announcementsPath}"]`;
      const form = page.locator(selector);
      await form.locator('[name=publication_state]').selectOption('draft');
      await form.locator('[name=severity]').selectOption('maintenance');
      const [response, navigation] = await Promise.all([
        posted(page, announcementsPath),
        page.waitForNavigation(),
        submit(page, selector, valid),
      ]);
      expect(response.status()).toBe(303);
      const editPath = response.headers().location;
      expect(editPath).toMatch(announcementEdit);
      await assertRouteResponse(navigation, editPath);
      await assertDocument(page, locale, editPath, actor);
      await assertAnnouncements(page, locale, editPath);
      const editor = page.locator(`main form[action="${editPath}"]`);
      for (const [field, value] of [['publication_state', 'draft'], ['severity', 'maintenance']]) {
        const select = editor.locator(`select[name="${field}"]`);
        await expect(select).toHaveValue(value);
        await expect(select.locator('option:checked')).toHaveText(announcementCopy(locale, value));
      }
      await expect(editor.locator('[name=title]')).toHaveValue(valid.title);
      await expect(editor.locator('[name=body]')).toHaveValue(valid.body);
      expect(Date.parse(await editor.locator('[name=starts_at]').inputValue())).toBe(Date.parse(valid.starts_at));
      targets.set(editPath, 200);
    });
  }
}

const ownedCharacterName = 'Parity Fixture';
const missingProfile = '/account/characters/missing-e2e/profile';
const projectionEpoch = '3';
const checkoutPrefix = '/native-account/coin-checkout/';

async function publishOwnershipFixture(route, fields) {
  const base = new URL(process.env.PORTAL_ACCOUNT_CLOSURE_BASE_URL);
  if (!['localhost', '127.0.0.1', '[::1]'].includes(base.hostname)) {
    throw new Error('Ownership fixtures require a loopback authority');
  }
  const directory = path.dirname(process.env.OTERYN_INTERNAL_TLS_CLIENT_CA_FILE);
  const payload = JSON.stringify({
    contract_version: 1,
    source_authority: process.env.OTERYN_LCFA_SOURCE_AUTHORITY,
    projection_epoch: projectionEpoch,
    ...fields,
  });
  const { status, body } = await new Promise((resolve, reject) => {
    const request = https.request(new URL(`/internal/v1/game-auth/native-account-characters${route}`, base), {
      method: 'POST',
      ca: fs.readFileSync(process.env.OTERYN_INTERNAL_TLS_CLIENT_CA_FILE),
      cert: fs.readFileSync(path.join(directory, 'closure-portfolio.pem')),
      key: fs.readFileSync(path.join(directory, 'closure-portfolio-key.pem')),
      minVersion: 'TLSv1.3',
      ALPNProtocols: ['http/1.1'],
      headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(payload) },
      timeout: 10_000,
    }, (response) => {
      let body = '';
      response.setEncoding('utf8');
      response.on('data', (chunk) => { body += chunk; });
      response.on('end', () => resolve({ status: response.statusCode, body }));
      response.on('error', reject);
    });
    request.on('error', reject);
    request.on('timeout', () => request.destroy(new Error('Ownership fixture publication timed out')));
    request.end(payload);
  });
  expect(status, fields.operation).toBe(200);
  return JSON.parse(body);
}

const unixSeconds = () => String(Math.floor(Date.now() / 1000));

async function publishOwnership(email) {
  const [character] = await query(`
    SELECT a.id AS account_id, uuidv7() AS character_id, w.id AS world_id
    FROM accounts a CROSS JOIN world_status w
    WHERE a.email = $1 AND w.name = 'Local Bazaar World'
    ORDER BY w.id LIMIT 1
  `, [email]);
  expect(character, 'local harness world for the ownership fixture').toBeDefined();
  const reply = await publishOwnershipFixture('', {
    operation: 'PublishAccountCharactersV1',
    account_id: character.account_id,
    projection_revision: '1',
    source_observed_at: unixSeconds(),
    characters: [{
      character_id: character.character_id,
      world_id: character.world_id,
      name: ownedCharacterName,
      availability: 'AVAILABLE',
    }],
  });
  expect(reply, 'owned character snapshot').toEqual({ contract_version: 1, result: 'accepted' });
  // The listing also requires a current, accepted ownership baseline.
  await query(`INSERT INTO pd_ownership_baselines
    (character_id,account_id,world_id,character_revision,from_projection_revision,to_projection_revision,projection_epoch,recovery_generation)
    VALUES ($1,$2,$3,1,1,1,$4,3)`,
    [character.character_id, character.account_id, character.world_id, projectionEpoch]);
}

async function ownershipWatermark() {
  const [watermark] = await query(`
    SELECT stamp, COALESCE(
      highest_epoch = $1 AND watermark_epoch = $1 AND source_authority = $2
      AND complete_through BETWEEN stamp - 5 AND stamp,
      false
    ) AS fresh
    FROM account_character_feed
    CROSS JOIN (SELECT floor(extract(epoch FROM clock_timestamp()))::bigint AS stamp) clock
    WHERE id = 1
  `, [projectionEpoch, process.env.OTERYN_LCFA_SOURCE_AUTHORITY]);
  expect(watermark, 'local ownership feed').toBeDefined();
  return watermark;
}

async function ensureFreshOwnership() {
  const watermark = await ownershipWatermark();
  // Reuse the verified watermark while it is recent. Publishing the same
  // second twice is superseded by the authority.
  if (watermark.fresh) return;
  const reply = await publishOwnershipFixture('/watermark', {
    operation: 'PublishProjectionWatermarkV1',
    complete_through: watermark.stamp,
    observed_at: watermark.stamp,
  });
  expect(reply, 'ownership watermark publication').toEqual({
    contract_version: 1,
    result: expect.stringMatching(/^(?:accepted|superseded)$/u),
  });
  expect((await ownershipWatermark()).fresh, `ownership watermark after ${reply.result}`).toBe(true);
}

async function seedCheckout(page, email) {
  const db = pool();
  try {
    // Persist the same quote and awaiting-payment order shape as the native API
    // in the disposable database. The browser GET binds the real sandbox intent.
    const { rows: [order] } = await db.query(`
      WITH quote AS (
        INSERT INTO coin_purchase_quotes
          (id, payer_account_id, package_id, coins, minor_units, currency, exponent, environment, expires_at, consumed_at)
        SELECT uuidv7(), id, uuidv7(), 12, 1000, 'PLN', 2, 'testing', now() + interval '5 minutes', now()
        FROM accounts WHERE email = $1
        RETURNING id, payer_account_id, package_id, coins, minor_units, currency, exponent, environment
      ), fixture AS (SELECT *, uuidv7() AS order_id FROM quote)
      INSERT INTO coin_purchase_orders
        (id, payer_account_id, request_id, quote_id, package_id, coins, minor_units, currency, exponent,
         environment, payment_intent_id, state, checkout_url, code)
      SELECT order_id, payer_account_id, uuidv7(), id, package_id, coins, minor_units, currency, exponent,
        environment, uuidv7(), 'awaiting_payment', $2 || order_id, 'COIN_ORDER_AWAITING_PAYMENT'
      FROM fixture
      RETURNING id, payer_account_id
    `, [email, new URL(checkoutPrefix, page.url()).href]);
    expect(order, 'disposable order belongs to the registered player').toBeDefined();
    return order;
  } finally {
    await db.end();
  }
}

const checkoutCopy = {
  pl: {
    title: "Zakup monet w trybie testowym",
    description: "To płatność testowa. Żadne pieniądze nie zostaną pobrane. Monety zostaną przyznane po weryfikacji płatności.",
    order: "Zamówienie",
    coins: "Monety",
    price: "Cena",
    status: "Status",
    pay: "Zakończ płatność testową",
    fail: "Odrzuć płatność testową",
    return: "Możesz wrócić do gry. Gra pobierze status zamówienia i saldo monet z Oteryn.",
    awaiting_payment: "Oczekuje na płatność",
    failed: "Niepowodzenie",
  },
  en: {
    title: "Sandbox coin checkout",
    description: "Test payment only. No real money is charged. Coins are credited only after the payment is verified.",
    order: "Order",
    coins: "Coins",
    price: "Price",
    status: "Status",
    pay: "Complete test payment",
    fail: "Fail test payment",
    return: "You can return to the game. It reads the order status and your coin balance from Oteryn.",
    awaiting_payment: "Awaiting payment",
    failed: "Failed",
  },
};

async function assertCheckout(page, locale, state) {
  const copy = (key) => checkoutCopy[locale][key];
  await expect(page).toHaveTitle(`${copy('title')} · Oteryn`);
  await expect(page.locator('main h1')).toHaveText(copy('title'));
  await expect(page.locator('main dt')).toHaveText(['order', 'coins', 'price', 'status'].map(copy));
  await expect(page.locator('main dt').filter({ hasText: new RegExp(`^${copy('price')}$`, 'u') }).locator('+ dd'))
    .toHaveText(locale === 'pl' ? '10,00 PLN' : '10.00 PLN');
  await expect(page.locator('main dt').filter({ hasText: new RegExp(`^${copy('status')}$`, 'u') }).locator('+ dd'))
    .toHaveText(copy(state));
  if (state === 'awaiting_payment') {
    await expect(page.locator('main > p')).toHaveText(copy('description'));
    await expect(page.locator('main form').filter({ has: page.locator('input[name=action][value=pay]') }).getByRole('button'))
      .toHaveText(copy('pay'));
    await expect(page.locator('main form').filter({ has: page.locator('input[name=action][value=fail]') }).getByRole('button'))
      .toHaveText(copy('fail'));
  } else {
    await expect(page.locator('main > p')).toHaveText(['description', 'return'].map(copy));
    await expect(page.locator('main form')).toHaveCount(0);
  }
}

async function assertAnonymousCheckoutRedirects(page, locale) {
  const target = sample(`${checkoutPrefix}{id}`, locale);
  const languagePath = `/language/${locale}`;
  await assertRouteResponse(await page.goto(languagePath), languagePath, { pathname: `/${locale}/` });
  expect((await page.goto(`/${locale}/login`)).status()).toBe(200);
  const csrf = await page.locator('main form input[name=_csrf]').first().inputValue();
  const options = { maxRedirects: 0, headers: { origin: new URL(page.url()).origin, 'accept-language': opposite(locale) } };
  for (const response of [
    await page.request.get(target, options),
    await page.request.post(target, { ...options, form: { _csrf: csrf, action: 'pay' } }),
  ]) {
    expect(response.status()).toBe(303);
    expect(response.headers().location).toBe(`/${locale}/login`);
  }
}

async function signInStaff(page, locale) {
  const account = await register(page, locale);
  await query(
    "INSERT INTO account_roles(account_id,role_id) SELECT a.id,r.id FROM accounts a CROSS JOIN roles r WHERE a.email=$1 AND r.key='platform_admin'",
    [account.email],
  );
  await login(page, locale, account);
  expect((await page.goto(`/${locale}/mfa`)).status()).toBe(200);
  await submit(page, 'main form[action$="/mfa/enroll"]', { password: account.password });
  const provisioning = await page.locator('main .op-prose').innerText();
  const totp = provisioning.startsWith('otpauth:')
    ? OTPAuth.URI.parse(provisioning)
    : new OTPAuth.TOTP({ secret: provisioning, algorithm: 'SHA1', digits: 6, period: 30 });
  await submit(page, 'main form[action$="/mfa/confirm"]', { password: account.password, code: totp.generate() });
  // The confirmation replaces the document. Wait for the recovery-code view
  // before reading it, since a count on the old DOM can return zero.
  const recovery = page.locator('textarea[readonly]').first();
  await expect(recovery).toBeVisible();
  await loginWithRecoveryCode(page, locale, account, await recovery.inputValue());
  await expect(page).toHaveURL(new RegExp(`/${locale}/$`, 'u'));
  expect((await page.goto(`/admin?lang=${locale}`)).status()).toBe(200);
}

test('panel detail forms return localized server validation', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const locale = info.project.metadata.locale;
  const owner = await player(page, locale);
  const staff = await isolatedAdmin(browser, info);
  try {
    const [{ id: staffId }] = await query('SELECT id FROM accounts WHERE email=$1', [staff.account.email]);
    await test.step('catalog price staging and activation', async () => {
      const offer = `parity.${unique()}`;
      const current = importGameCatalog(staffId, offer, 21);
      const candidate = importGameCatalog(staffId, offer, 41);
      const [previous] = await query('SELECT catalog_revision, activated_by, updated_at::text FROM store_active_catalog WHERE id=1');
      catalogCommand(['activate', current], staffId);
      try {
        const staged = `/admin/store-catalog?lang=${locale}`;
        await assertRouteResponse(await staff.page.goto(staged), '/admin/store-catalog');
        const stageForm = staff.page.locator('main article').filter({ hasText: offer }).locator('form');
        await refusedForm(staff.page, locale, stageForm, { price: '0', reason: 'price_correction' }, 422);

        const activation = `/admin/store-activation?lang=${locale}`;
        await assertRouteResponse(await staff.page.goto(activation), '/admin/store-activation');
        const activationForm = staff.page.locator('main article').filter({ hasText: candidate }).locator('form');
        await refusedForm(staff.page, locale, activationForm, { revision: 'invalid-revision', reason: 'game_catalog_verified' }, 422);
        expect(await query('SELECT catalog_revision FROM store_active_catalog WHERE id=1')).toEqual([{ catalog_revision: current }]);
        expect(await query('SELECT 1 FROM store_price_proposals WHERE base_revision=$1 AND offer_key=$2', [current, offer])).toEqual([]);
      } finally {
        const restored = await query(`
          UPDATE store_active_catalog SET catalog_revision=$1, activated_by=$2, updated_at=$3
          WHERE id=1 AND catalog_revision=$4 RETURNING id
        `, [previous.catalog_revision, previous.activated_by, previous.updated_at, current]);
        expect(restored).toHaveLength(1);
      }
    });

    await test.step('payment review and reconciliation alias', async () => {
      const intent = uuid7();
      await query(`
        INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state)
        VALUES($1,$2,'test',$3,250,'PLN','succeeded')
      `, [intent, owner.id, `test-${intent}`]);
      for (const path of ['/admin/payment-review', '/admin/commerce/reconcile']) {
        await assertRouteResponse(await staff.page.goto(`${path}?lang=${locale}&id=${intent}`), path);
        await refusedForm(staff.page, locale, staff.page.locator('main form[method=post]'), {
          version: 'invalid-review-version', reason: 'evidence_reviewed',
        }, 422);
      }
      expect(await query('SELECT 1 FROM commerce_operations WHERE target_id=$1', [intent])).toEqual([]);
    });

    await test.step('owner sanction appeal', async () => {
      await assertRouteResponse(await staff.page.goto('/admin/moderation/enforcement/create'), '/admin/moderation/enforcement/create');
      const now = Math.floor(Date.now() / 1000);
      await action(staff.page, 'form:has(input[value=sanction])', {
        target: owner.id, category: 'restriction', status: 'active', public_reason: unique(),
        effective_at: String(now - 1), expires_at: String(now + 3600),
      });
      await expect(staff.page).toHaveURL(/enforcement\/[a-f0-9-]+$/u);
      const sanction = new URL(staff.page.url()).pathname.split('/').at(-1);
      const path = `/${locale}/support/appeals/${sanction}`;
      await assertRouteResponse(await page.goto(path), path);
      const form = page.locator('main form');
      const workflow = () => query(`
        SELECT status, message, outcome, version FROM support_appeal_workflows WHERE sanction_id=$1
      `, [sanction]);
      const before = await workflow();
      expect(before).toEqual([{ status: 'none', message: null, outcome: null, version: '1' }]);
      await refusedForm(page, locale, form, { message: '   ' }, 422);
      expect(await workflow()).toEqual(before);
    });
  } finally {
    await staff.context.close();
  }
});

const playerGroups = {
  account: ['account', 'password', 'support'],
  commerce: ['native-account', 'panel', 'shop', 'collection', 'collection-pvp', 'league', 'pass', 'puzzles'],
  community: ['atlas', 'bazaar', 'characters', 'deaths', 'events', 'guilds', 'houses', 'online', 'party', 'rankings', 'stats', 'worlds'],
  search: ['search', 'wiki'],
};
const playerRouteGroups = new Map(Object.entries(playerGroups).flatMap(([group, segments]) =>
  segments.map((segment) => [segment, group])));

function playerRouteGroup(route) {
  const [, segment] = route.replace(/^\/(?:pl|en)(?=\/|$)/u, '').split('/');
  return playerRouteGroups.get(segment) ?? 'general';
}

const crawls = [
  { actor: 'anonymous' },
  ...[...Object.keys(playerGroups), 'general'].map((group) => ({ actor: 'player', group })),
  { actor: 'staff' },
];
for (const { actor, group } of crawls) {
  const scope = group ? `${actor} ${group}` : actor;
  test(`${scope} HTML pages use the requested language`, async ({ page }, info) => {
    test.setTimeout(600_000);
    const locale = info.project.metadata.locale;
    if (actor === 'staff') await signInStaff(page, locale);
    let checkout;
    if (actor === 'player') {
      const account = await register(page, locale);
      await login(page, locale, account);
      if (group === 'commerce') {
        const order = await seedCheckout(page, account.email);
        checkout = checkoutPrefix + order.id;
      }
      if (group === 'account') await publishOwnership(account.email);
    }
    if (actor === 'anonymous') {
      await seedPasswordRange('registration');
      await assertAnonymousCheckoutRedirects(page, locale);
    }
    const origin = new URL(`/${locale}/`, portalOrigin);
    const languagePath = `/language/${locale}`;
    await assertRouteResponse(await page.goto(languagePath), languagePath, { pathname: origin.pathname });
    const targets = new Map(checkout ? [[checkout, 200]] : []);
    const knownRoutes = destinations(actor, locale);
    for (const [route, status] of knownRoutes) {
      if (!group || playerRouteGroup(route) === group) targets.set(route, status);
    }
    const probed = [];
    const documents = new Set();
    const checked = new Set();
    const uncovered = new Set();
    const missingProfiles = [missingProfile, `/${locale}${missingProfile}`];
    const missingProfilesChecked = [];
    let ownedProfileChecked = false;
    for (const [target, status] of targets) {
      if (target === languagePath) continue;
      await test.step(target, async () => {
        const url = new URL(target, origin);
        expect(url.origin, `${target}: crawl origin`).toBe(origin.origin);
        // These screens expose a locale query parameter. Other routes use
        // the language cookie or Accept-Language and can reject extra queries.
        if (localeQueryRoute.test(url.pathname)) {
          url.searchParams.set('lang', locale);
        }
        await waitForWikiBudget(target, 2);
        const accountOverview = group === 'account' && url.pathname === '/account';
        if (accountOverview) await ensureFreshOwnership();
        const missingOwnedProfile = actor === 'player' && missingProfiles.includes(url.pathname);
        if (missingOwnedProfile) {
          if (!ownedProfileChecked) {
            const ownedProfile = `/${locale}/account/characters/${encodeURIComponent(ownedCharacterName)}/profile`;
            await ensureFreshOwnership();
            await assertRouteResponse(await page.goto(ownedProfile), ownedProfile);
            await expect(page.locator('main')).toContainText(ownedCharacterName);
            await expect(page.locator('main textarea[name=comment]')).toBeVisible();
            await assertDocument(page, locale, ownedProfile, actor);
            await assertProfileControls(page, locale);
            documents.add(ownedProfile);
            ownedProfileChecked = true;
          }
          await ensureFreshOwnership();
        }
        const probe = await page.request.get(url.href, { headers: { accept: 'text/html', 'accept-language': locale } });
        if (missingOwnedProfile) {
          expect(probe.status(), target).toBe(404);
          expect(new URL(probe.url()).pathname, target).toBe(url.pathname);
          missingProfilesChecked.push(url.pathname);
        }
        const announcements = actor === 'staff' && [announcementsPath, announcementEditor].includes(target);
        const required = target === checkout || announcements;
        const contract = routeContract(url.pathname, status, actor, locale);
        await assertRouteResponse(probe, url.pathname, contract);
        probed.push(url.pathname);
        if (!contract.document) {
          if (url.pathname === '/oauth/authorize') {
            expect(probe.headers()['cache-control']).toBe('no-store');
            expect(await probe.json()).toEqual({
              error: 'invalid_request',
              error_description: 'The request is missing a required parameter or is malformed.',
            });
          }
          return;
        }
        if (target === checkout) {
          expect(probe.headers()['referrer-policy']).toBe('same-origin');
          await assertRouteResponse(await page.goto(url.href), url.pathname);
          await assertDocument(page, locale, target, actor);
          await assertCheckout(page, locale, 'awaiting_payment');
          documents.add(url.pathname);
          await nativeValidation(page, locale, checked, uncovered);
          for (const action of ['pay', 'fail']) {
            await refusedForm(page, locale, page.locator(`main form:has(input[name=action][value=${action}])`),
              { action: 'unsupported-payment-action' }, 400);
          }
          await test.step('failed sandbox payment return instructions', async () => {
            const [response, navigation] = await Promise.all([
              posted(page, url.pathname),
              page.waitForNavigation(),
              submit(page, 'main form:has(input[name=action][value=fail])'),
            ]);
            expect(response.status()).toBe(303);
            await assertRouteResponse(navigation, url.pathname);
            await assertDocument(page, locale, target, actor);
            await assertCheckout(page, locale, 'failed');
          });
          return;
        }
        const final = new URL(probe.url());
        if (!required && documents.has(final.pathname + final.search)) return;
        documents.add(final.pathname + final.search);
        const navigation = await page.goto(url.href);
        expect.soft(navigation.status(), target).toBe(probe.status());
        await assertRouteResponse(navigation, url.pathname, contract);
        const localizedPath = /^\/(pl|en)(?:\/|$)/u.exec(contract.pathname)?.[1];
        const editorialLocale = /^\/admin\/(?:pages\/[^/]+|wiki\/articles\/[^/]+\/preview)\/(pl|en)$/u
          .exec(contract.pathname)?.[1];
        const documentLocale = localizedPath ?? editorialLocale ?? locale;
        await assertDocument(page, documentLocale, target, actor);
        if (navigation.ok() && /^\/(?:(?:pl|en)\/)?(?:mfa\/)?login$/u.test(contract.pathname)) {
          await assertSignInControls(page, documentLocale);
        }
        if (navigation.ok() && searchRoute.test(contract.pathname)) {
          const search = page.locator('main form[action$="/search"] button[type=submit]');
          await expect(search).toHaveText(searchCopy[documentLocale]);
        }
        if (announcements) await assertAnnouncements(page, locale, target);
        if (accountOverview) {
          const publicProfile = `/${locale}/characters/${encodeURIComponent(ownedCharacterName)}`;
          await expect(page.locator('main').getByRole('link', { name: ownedCharacterName, exact: true }))
            .toHaveAttribute('href', publicProfile);
          // The fixture publishes ownership. The public projection remains unavailable.
          targets.set(publicProfile, knownRoutes.get(sample('/{lang}/characters/{name}', locale)));
        }
        if (navigation.ok()) {
          await addLinkedPages(page, targets, knownRoutes, locale, actor);
          await nativeValidation(page, documentLocale, checked, uncovered);
          await serverValidation(page, locale, actor, target, targets);
        }
      });
    }
    if (group === 'account') {
      expect(ownedProfileChecked, 'owned profile checked before missing profiles').toBe(true);
      expect(missingProfilesChecked.sort(), 'both missing profiles checked').toEqual(missingProfiles.sort());
    }
    expect(documents.size, `${scope}: HTML documents visited`).toBeGreaterThan(0);
    await info.attach('i18n-coverage', {
      body: JSON.stringify({ routes: probed, documents: [...documents].map((url) => new URL(url, origin).pathname),
        nativeValidation: [...checked], formsWithoutNativeValidation: [...uncovered] }),
      contentType: 'application/json',
    });
  });
}

test('invalid sign-in submits a localized server error', async ({ page }, info) => {
  const locale = info.project.metadata.locale;
  await login(page, locale, { email: 'i18n-missing@example.invalid', password: 'Invalid local input!' });
  await expect(page).toHaveURL(new RegExp(`/${locale}/login$`, 'u'));
  await expect(page.locator('main [role=alert]')).toHaveText(signInCopy[locale].error);
  await assertDocument(page, locale, 'sign-in refusal');
  await assertSignInControls(page, locale);
});


test('raw Fluent tokens distinguish leaks from approved prose', () => {
  const passkeys = '/en/account/passkeys';
  const revoke = catalogText('templates/account/passkeys/en.ftl', 'passkeys-revoke');
  expect(rawKeysIn('Help: passkeys-help', passkeys)).toEqual(['passkeys-help']);
  expect(rawKeysIn(revoke, passkeys)).toEqual([]);
  expect(rawKeysIn(revoke + ' sign-in', passkeys)).toEqual(['sign-in', 'sign-in']);
  expect(rawKeysIn('Help: sign-in', '/en/atlas')).toEqual(['sign-in']);
  for (const path of ['/en/legal/cookies', '/en/legal/terms']) {
    expect(rawKeysIn(signInProse.get(path), path)).toEqual([]);
    expect(rawKeysIn(signInProse.get(path), '/en/atlas')).toEqual(['sign-in']);
    expect(rawKeysIn('sign-in', path)).toEqual(['sign-in']);
  }
  expect(rawKeysIn('passkeys-help', '/pl/login')).toEqual(['passkeys-help']);
  expect(rawKeysIn('search', '/pl/search')).toEqual(['search']);
  expect(rawKeysIn('search', '/en/search')).toEqual(['search']);
  expect(rawKeysIn('search', '/en/atlas')).toEqual([]);
  expect(rawKeysIn('Use search', '/en/search')).toEqual([]);
  for (const prefix of ['', '/pl', '/en']) {
    expect(rawKeysIn('Fields: login-email, mfa-password', prefix + '/login')).toEqual(['login-email', 'mfa-password']);
    const profile = prefix + '/account/characters/Parity%20Fixture/profile';
    expect(rawKeysIn('Settings: hide-all and online-time', profile)).toEqual(['hide-all', 'online-time']);
  }
});
