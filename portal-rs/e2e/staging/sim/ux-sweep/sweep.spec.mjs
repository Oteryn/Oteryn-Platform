import { mkdir, writeFile } from 'node:fs/promises';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { expect, test } from '@playwright/test';
import {
  apiContext, containsAccessValue, expectPath, get, languageOf, localeText, localization, mailCode, mailLink,
  mailStatus, open, origin, password, playerEmail, projects, protect, ready, redact,
  routeKey, safeError, StagingBlocked, submit, totp, visibleText,
} from './helpers.mjs';

const layout = 'portal-rs/templates/layout.html';
const limits = { pages: 180, depth: 7, tabs: 32 };
const mailInterval = 10 * 60_000;
const ui = 'crates/ui/i18n/{lang}.ftl';
const resetLocale = 'templates/account/reset/{lang}.ftl';
const mfaLocale = 'templates/account/mfa/{lang}.ftl';
const searchLocale = 'crates/web/src/search/{lang}.ftl';
const staffEmail = 'staff-platform-admin@example.invalid';
const routeInventory = JSON.parse(readFileSync(new URL('../../../route-expectations.json', import.meta.url), 'utf8'));
const knownCold = routeInventory.filter(row => row.method === 'GET' && row.status === 503).map(row =>
  new RegExp('^' + row.route.replace('{lang}', '(?:en|pl)').replace(/\{[^}]+\}/g, '[^/]+') + '$'));
const bazaar = /^\/(?:en|pl)\/bazaar$/;
const league = /^\/(?:en|pl)\/league$/;
const shopFallback = /^\/static\/thais\/shop-(?:en|pl)\.html$/;
const shopClosed = {
  en: 'The shop is currently closed. Purchases and coin top-ups are temporarily unavailable.',
  pl: 'Sklep jest obecnie zamknięty. Zakupy i doładowania są chwilowo niedostępne.',
};

test.beforeEach(async () => {
  const api = await apiContext();
  try {
    await ready(api);
  } catch (error) {
    if (!(error instanceof StagingBlocked)) throw error;
    test.skip(true, safeError(error));
  } finally {
    await api.dispose();
  }
});

async function report(info, project, actor, result) {
  const dir = join(info.config.outputDir || info.project.outputDir, 'pages');
  await mkdir(dir, { recursive: true });
  await writeFile(join(dir, `${project}-${actor}.json`), JSON.stringify(result, null, 2) + '\n');
}

async function screen(browser, project) {
  const context = await protect(await browser.newContext({
    baseURL: origin,
    locale: project.lang,
    viewport: project.viewport,
    hasTouch: project.viewport.width === 390,
    isMobile: project.viewport.width === 390,
    serviceWorkers: 'block',
    acceptDownloads: false,
    ignoreHTTPSErrors: Boolean(process.env.HTTPS_PROXY || process.env.https_proxy),
  }));
  return context;
}

function requestDetails(page) {
  const collected = [];
  const add = (kind, url, detail) => {
    const parsed = new URL(url || page.url(), origin);
    const path = (parsed.origin === origin ? '' : parsed.hostname + ' ') + routeKey(parsed);
    collected.push({ kind, path, detail: redact(detail) });
  };
  page.on('response', response => {
    if (response.status() >= 400) add('HTTP', response.url(), response.status());
  });
  page.on('requestfailed', request => add('request failed', request.url(), request.failure()?.errorText));
  page.on('pageerror', error => add('page error', page.url(), error.message));
  page.on('console', message => {
    if (message.type() === 'error') add('console error', message.location().url, message.text());
  });
  return {
    issues(path, status) {
      return [...new Map(collected.filter(issue =>
        !(issue.path === path && status >= 400 &&
          (issue.kind === 'HTTP' || issue.detail.startsWith(`Failed to load resource: the server responded with a status of ${status}`))) &&
        !(issue.kind === 'HTTP' && issue.detail === '429') &&
        !/^Failed to load resource: the server responded with a status of 429\b/.test(issue.detail),
      ).map(issue => [JSON.stringify(issue), issue])).values()].map(issue => ({
        check: issue.kind, detail: `${issue.path}: ${issue.detail}`,
        source: `${layout}:8`,
      }));
    },
  };
}

async function documentFacts(page) {
  return page.evaluate(() => {
    const rendered = element => Boolean(element.getClientRects().length) &&
      getComputedStyle(element).visibility === 'visible';
    const describe = element => element.localName + (element.id ? '#' + element.id : '') +
      (element.name ? '[name=' + element.name + ']' : '');
    const text = element => (element?.innerText || element?.textContent || '').trim();
    const controls = [...document.querySelectorAll('input:not([type=hidden]),select,textarea,button,[role=textbox],[role=combobox],[role=checkbox],[role=radio],[role=switch],[role=button]')];
    const unnamed = controls.filter(element => {
      const labelledBy = (element.getAttribute('aria-labelledby') || '').split(/\s+/)
        .some(id => text(document.getElementById(id)));
      const button = element.matches('button,[role=button]');
      const namedInput = element.matches('input[type=submit],input[type=reset]') ||
        (element.matches('input[type=button],input[type=image]') && (element.value || element.alt));
      return !labelledBy && !element.getAttribute('aria-label')?.trim() &&
        !element.getAttribute('title')?.trim() &&
        !Array.from(element.labels || []).some(label => text(label) || label.querySelector('img[alt]:not([alt=""])')) &&
        !(button && (text(element) || element.querySelector('img[alt]:not([alt=""])'))) && !namedInput;
    });
    const visibleStrings = [...document.querySelectorAll('body *')].filter(element =>
      !element.matches('script,style,template,svg,svg *') && rendered(element)).flatMap(element => [
      ...(element.matches('h1,h2,h3,h4,a,button,label,p,summary,th,dt,option') ? [text(element)] : []),
      ...Array.from(element.childNodes).filter(node => node.nodeType === Node.TEXT_NODE).map(node => node.textContent),
      ...['alt', 'title', 'placeholder', 'aria-label'].map(attr => element.getAttribute(attr)).filter(Boolean),
    ]);
    return {
      lang: document.documentElement.lang,
      title: document.title,
      width: document.documentElement.scrollWidth,
      viewport: document.documentElement.clientWidth,
      imagesWithoutAlt: [...document.images].filter(image => !image.hasAttribute('alt'))
        .map(image => image.getAttribute('src') || describe(image)),
      unnamed: unnamed.map(describe),
      visibleStrings: [...new Set([...visibleStrings, document.title.split(' · ')[0]])].filter(Boolean),
      body: document.body?.innerText || '',
      ids: [...document.querySelectorAll('[id],a[name]')].map(element => element.id || element.name),
      links: [...document.querySelectorAll('a[href],area[href]')].map(link => ({
        href: link.getAttribute('href'), target: link.href, download: link.hasAttribute('download'),
      })),
    };
  });
}

async function keyboardFocus(page) {
  const sampled = new Set();
  const missing = new Set();
  for (let i = 0; i < limits.tabs; i++) {
    await page.keyboard.press('Tab');
    const focus = await page.evaluate(async () => {
      // Scroll-to-focus and layout can complete after the keyboard event.
      await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const element = document.activeElement;
      if (!element || element === document.body) return null;
      const style = getComputedStyle(element);
      const label = element.id ? '#' + element.id : element.localName +
        (element.name ? '[name=' + element.name + ']' : '') + ' ' +
        (element.getAttribute('href') || '').split(/[?#]/)[0];
      const outline = style.outlineStyle !== 'none' && parseFloat(style.outlineWidth) > 0 &&
        !['transparent', 'rgba(0, 0, 0, 0)'].includes(style.outlineColor);
      const shadow = style.boxShadow !== 'none';
      const inViewport = () => {
        const rect = element.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0 && rect.right > 0 && rect.bottom > 0 &&
          rect.left < innerWidth && rect.top < innerHeight;
      };
      if (!inViewport()) await new Promise(resolve => setTimeout(resolve, 320));
      const position = [...document.querySelectorAll('body *')].indexOf(element);
      return { label, visible: element.matches(':focus-visible') && (outline || shadow) &&
        inViewport(), position };
    });
    if (!focus) continue;
    if (sampled.has(focus.position)) break;
    sampled.add(focus.position);
    if (!focus.visible) missing.add(focus.label);
  }
  return { sampled: sampled.size, missing: [...missing].map(redact) };
}

function pathURL(value) {
  const url = new URL(value, origin);
  const fragment = url.hash.slice(1);
  url.hash = '';
  return { url, fragment };
}

function screenshotPath(info, index) {
  return info.outputPath(`page-${String(index).padStart(3, '0')}.png`);
}

async function capture(page, info, index) {
  const url = new URL(page.url());
  if (url.searchParams.has('token') || /\/(?:mfa|verify-email|reset-password\/[^/]+)(?:\/|$)/.test(url.pathname)) {
    return 'Withheld on a credential page';
  }
  const path = screenshotPath(info, index);
  const body = await page.locator('body').innerText({ timeout: 2_000 });
  if (containsAccessValue(body) || /(?:reset-password\/|verify-email\?token=)[0-9a-f]{32,}/i.test(body)) return 'Withheld';
  await page.screenshot({
    path,
    timeout: 8_000,
    mask: [page.locator('input, textarea, [data-token]')],
  });
  return path;
}

const stagingUnavailable = (path, status, facts, lang) => {
  if (status !== 503) return false;
  if (bazaar.test(path)) return facts.body.includes(localeText('templates/bazaar/{lang}.ftl', lang, 'authority-unavailable'));
  if (league.test(path)) return facts.visibleStrings.includes(localeText('crates/minigames/src/league/{lang}.ftl', lang, 'unavailable'));
  return knownCold.some(known => known.test(path)) && facts.visibleStrings.some(text =>
    lang === 'pl' ? /niedostępn|brak zatwierdzonych danych/i.test(text) : /unavailable|no qualified data/i.test(text));
};

async function crawl(context, info, project, start, reportData, actor) {
  const queue = start.map(target => ({ url: new URL(target, origin), from: '(entry)', depth: 0 }));
  const visited = new Set(queue.map(item => item.url.href));
  const ids = new Map();
  const edges = [];
  for (const item of queue) {
    if (reportData.pages.length === limits.pages) break;
    if (item.depth > limits.depth) break;
    const path = routeKey(item.url);
    const row = { path, from: item.from, depth: item.depth, issues: [] };
    reportData.pages.push(row);
    const issue = (check, detail, source) => row.issues.push({ check, detail: redact(detail), source });
    if (item.download) {
      try {
        const response = await get(context.request, item.url);
        row.status = response.status();
        row.nonHtml = response.headers()['content-type'] || '(download)';
        await response.dispose();
        if (row.status !== 200) issue('HTTP', `Download link returned HTTP ${row.status}`, `${layout}:19`);
      } catch (error) {
        issue('download link', safeError(error), `${layout}:19`);
      }
      continue;
    }
    const page = await context.newPage();
    const network = requestDetails(page);
    try {
      const { response, chain } = await open(page, item.url);
      row.status = response?.status();
      row.final = routeKey(page.url());
      row.redirects = chain;
      if (row.status === 429) {
        row.blocked = 'Shared egress rate limit persisted after two retries';
        continue;
      }
      const final = new URL(page.url());
      const language = languageOf(item.url) || languageOf(final) || project.lang;
      if (actor === 'anonymous' && /^\/(pl|en)\/support\/tickets\/create$/.test(item.url.pathname) &&
          (chain[0]?.status !== 303 || final.pathname !== `/${language}/login`)) {
        issue('redirect', `Expected HTTP 303 to /${language}/login, reached ${row.final}`,
          'portal-rs/crates/support/src/tickets/web.rs:438');
      }
      const contentType = response?.headers()['content-type'] || '';
      if (!contentType.includes('text/html')) {
        if (row.status !== 200) issue('HTTP', `Expected an available link; received HTTP ${row.status}`, `${layout}:19`);
        row.nonHtml = contentType || '(no content-type)';
        row.issues.push(...network.issues(row.final, row.status));
        continue;
      }
      const facts = await documentFacts(page);
      ids.set(item.url.href, new Set(facts.ids));
      ids.set(pathURL(final).url.href, new Set(facts.ids));
      row.lang = facts.lang;
      row.title = redact(facts.title);
      row.available = !stagingUnavailable(final.pathname, row.status, facts, language);
      if (row.status === 200 && shopFallback.test(final.pathname)) {
        const shown = facts.visibleStrings.includes(shopClosed[language]);
        if (shown) row.available = false;
        else issue('store state', 'The storefront does not show its closed-state message', `portal-rs/static/thais/shop-${language}.html:38`);
      }
      if (row.status !== 200 && row.available) {
        issue('HTTP', `Expected HTTP 200 or an intended redirect; received HTTP ${row.status}`, `${layout}:19`);
      }
      if (facts.lang !== language) issue('html lang', `Expected ${language}; received ${facts.lang || '(empty)'}`, `${layout}:2`);
      if (!facts.title.trim()) issue('title', 'Document title is empty', `${layout}:7`);
      row.issues.push(...localization(facts.visibleStrings, language));
      if (project.viewport.width === 390 && (facts.width > 390 || facts.viewport > 390)) {
        issue('overflow', `${facts.width}px content in ${facts.viewport}px viewport`, `${layout}:5`);
      }
      for (const image of facts.imagesWithoutAlt) issue('image alt', image, `${layout}:19`);
      for (const field of facts.unnamed) issue('form label', field, `${layout}:19`);
      const focus = await keyboardFocus(page);
      row.focusSample = focus.sampled;
      if (!focus.sampled && facts.links.length) issue('keyboard focus', 'Tab did not reach an interactive target', `${layout}:19`);
      for (const target of focus.missing) issue('keyboard focus', target, `${layout}:19`);
      row.issues.push(...network.issues(row.final, row.status));
      row.external = 0;
      for (const link of facts.links) {
        if (link.target.startsWith('mailto:') || link.target.startsWith('tel:')) {
          if (!link.target.split(':')[1]) issue('external href', 'Empty address', `${layout}:19`);
          row.external++;
          continue;
        }
        if (link.target.startsWith('javascript:') || !link.href?.trim()) {
          issue('link href', 'Anchor lacks a navigable href', `${layout}:19`);
          continue;
        }
        const { url, fragment } = pathURL(link.target);
        if (url.origin !== origin) {
          if (!['http:', 'https:'].includes(url.protocol) || !url.hostname) {
            issue('external href', 'Anchor lacks a valid HTTP(S) destination', `${layout}:19`);
          }
          row.external++;
          continue;
        }
        if (fragment) edges.push({ row, url, fragment });
        if (languageOf(url) !== project.lang && /^\/(pl|en)\/$/.test(url.pathname) && !url.search) {
          // Each project explicitly exercises its counterpart home via the language links.
          reportData.otherLocale.add(routeKey(url));
          continue;
        }
        if (visited.has(url.href)) continue;
        visited.add(url.href);
        queue.push({ url, from: path, depth: item.depth + 1, download: link.download });
      }
    } catch (error) {
      issue('navigation', safeError(error), `${layout}:19`);
    } finally {
      if (row.issues.length && actor === 'anonymous') {
        try { row.screenshot = await capture(page, info, reportData.pages.length); }
        catch { row.screenshot = 'Unavailable after navigation failure'; }
      }
      await page.close();
      await delay(150);
    }
  }
  for (const { row, url, fragment } of edges) {
    let anchor;
    try { anchor = decodeURIComponent(fragment); } catch { anchor = fragment; }
    if (!ids.has(url.href) || ids.get(url.href).has(anchor)) continue;
    if (!row.issues.some(issue => issue.check === 'fragment' && issue.detail === redact(`${routeKey(url)}#${anchor}`))) {
      row.issues.push({
        check: 'fragment', detail: redact(`${routeKey(url)}#${anchor}`),
        source: 'portal-rs/templates/components/thais-header.html:1',
      });
      if (!row.screenshot && actor === 'anonymous') {
        const page = await context.newPage();
        try {
          await open(page, row.final);
          row.screenshot = await capture(page, info, reportData.pages.indexOf(row) + 1);
        } catch { row.screenshot = 'Unavailable after navigation failure'; }
        finally { await page.close(); }
      }
    }
  }
  reportData.pending = queue.slice(reportData.pages.length).map(item => routeKey(item.url));
}

const newReport = () => ({ pages: [], pending: [], otherLocale: new Set(), flow: [] });

async function save(info, project, actor, result) {
  await report(info, project, actor, { ...result, otherLocale: [...result.otherLocale] });
}

function checkReport(result) {
  const failed = result.pages.filter(row => row.issues.length).map(row => row.path);
  expect.soft(failed, 'Pages with UX defects. See the per-page JSON report.').toEqual([]);
  expect.soft(result.pending, 'Crawl bound left reachable destinations pending').toEqual([]);
  const blocked = result.pages.filter(row => row.blocked).map(row => row.path);
  expect.soft(blocked, 'Destinations blocked by shared staging capacity').toEqual([]);
}

function assertStatus(response, expected, name) {
  const actual = response?.status();
  if (actual === 429) throw new StagingBlocked(`Shared egress rate limit blocked ${name}`);
  const path = response ? ` at ${routeKey(response.url())}` : '';
  expect(actual, `${name}${path} expected HTTP ${expected}, received ${actual ?? 'no response'}`).toBe(expected);
}

async function flow(result, label, run, evidence) {
  try {
    await run();
    result.flow.push({ label, result: 'PASS' });
  } catch (error) {
    const entry = { label, result: error instanceof StagingBlocked ? 'BLOCKED' : 'FAIL', detail: safeError(error) };
    result.flow.push(entry);
    if (evidence && entry.result === 'FAIL') {
      const { page, info } = evidence;
      try { entry.screenshot = await capture(page, info, 900 + result.flow.length); }
      catch { entry.screenshot = 'Unavailable after form failure'; }
    }
    if (error instanceof StagingBlocked) result.blocked = safeError(error);
    else expect.soft(false, `${label}: ${safeError(error)}`).toBe(true);
  }
}

test('anonymous navigation and public interaction', async ({ browser }, info) => {
  const project = { name: info.project.name, ...info.project.metadata };
  const { lang } = project;
  const result = newReport();
  const context = await screen(browser, project);
  const page = await context.newPage();
  try {
    await flow(result, 'Language link and browser history', async () => {
      const other = lang === 'pl' ? 'en' : 'pl';
      assertStatus((await open(page, `/${lang}/`)).response, 200, 'Home');
      const switcher = page.locator(`a[href="/${other}/"]:visible`).first();
      await switcher.click();
      await expectPath(page, `/${other}/`, 'Language switch destination');
      await expect(page.locator('html')).toHaveAttribute('lang', other);
      await page.goBack({ waitUntil: 'load' });
      await expectPath(page, `/${lang}/`, 'Back returns to original language');
      await expect(page.locator('html')).toHaveAttribute('lang', lang);
    });
    await flow(result, 'World navigation opens and reaches the wiki', async () => {
      assertStatus((await open(page, `/${lang}/`)).response, 200, 'Home opens');
      const menu = page.locator('#portal-menu-toggle');
      if (project.viewport.width === 390) {
        await menu.tap();
        await expect(menu).toHaveAttribute('aria-expanded', 'true');
      }
      const world = page.locator('#portal-world-button');
      await world.focus();
      await page.keyboard.press('ArrowDown');
      await expect(world).toHaveAttribute('aria-expanded', 'true');
      const wiki = page.locator(`#portal-world-panel a[href="/${lang}/wiki"]`);
      await expect(wiki).toBeVisible();
      await wiki.click();
      await expectPath(page, `/${lang}/wiki`, 'Wiki opens from navigation');
    });
    await flow(result, 'Public search success and invalid input', async () => {
      assertStatus((await open(page, `/${lang}/search`)).response, 200, 'Search form');
      await visibleText(page.locator('h1'), localeText(searchLocale, lang, 'title'), 'Search title');
      const valid = await submit(page, 'main form', { q: 'ux sweep no matching editorial item', kind: '' });
      assertStatus(valid.response, 200, 'Valid search');
      await visibleText(page.locator('main p'), localeText(searchLocale, lang, 'empty'), 'Empty search result');
      assertStatus(await page.reload(), 200, 'Refresh the search results');
      await expect(page.locator('[name=q]')).toHaveValue('ux sweep no matching editorial item');
      await visibleText(page.locator('main p'), localeText(searchLocale, lang, 'empty'), 'Results persist after refresh');
      const invalid = await submit(page, 'main form', { q: 'x' });
      assertStatus(invalid.response, 422, 'One-character search');
      await visibleText(page.locator('main h1'), localeText(ui, lang, 'error-422-title'), 'Localized validation title');
      await visibleText(page.locator('main p'), localeText(ui, lang, 'error-422-description'), 'Localized validation explanation');
    });
    await flow(result, 'Anonymous account access is refused', async () => {
      const target = await open(page, '/account');
      expect(target.chain[0]?.status, 'Anonymous account redirects').toBe(303);
      assertStatus(target.response, 200, 'Sign-in page opens');
      await visibleText(page.locator('main h1'), localeText(mfaLocale, lang, 'mfa-login'), 'Localized sign-in title');
    });
    await crawl(context, info, project, [`/${lang}/`], result, 'anonymous');
    checkReport(result);
  } finally {
    await save(info, project.name, 'anonymous', result);
    await context.close();
  }
});

const registerForm = 'main form[action$="/register"]';
const signInForm = 'main form[action$="/login"]';
const mfaChallenge = 'main form[action$="/mfa/challenge"]';
const verificationSubjects = ['Potwierdź adres e-mail', 'Verify your email address'];

async function signIn(page, lang, credentials) {
  assertStatus((await open(page, `/${lang}/login`)).response, 200, 'Login form');
  return submit(page, signInForm, credentials);
}

async function registerPlayer(page, lang, credentials, doubleClick = false) {
  assertStatus((await open(page, `/${lang}/register`)).response, 200, 'Registration form');
  const since = Date.now();
  const submitted = await submit(page, registerForm, {
    ...credentials, password_confirmation: credentials.password, terms: true, privacy: true,
  }, { doubleClick });
  assertStatus(submitted.response, 200, 'Registration inbox page');
  expect(submitted.status, 'Registration redirects to inbox page').toBe(303);
  await expectPath(page, `/${lang}/register/check-inbox`, 'Open the check-inbox page');
  await visibleText(page.getByRole('status'), localeText(ui, lang, 'verify-inbox'), 'Inbox instructions');
  assertStatus(await page.reload(), 200, 'Refreshing the inbox page preserves the instructions');
  await visibleText(page.getByRole('status'), localeText(ui, lang, 'verify-inbox'), 'Inbox instructions after refresh');
  return since;
}

async function verifyPlayer(page, api, lang, email, since) {
  const link = await mailLink(api, {
    email, since, subject: verificationSubjects, route: /^\/(pl|en)\/verify-email\?token=[0-9a-f]+$/i,
  });
  assertStatus((await open(page, link)).response, 200, 'Verification confirmation page');
  const verified = await submit(page, 'main form');
  assertStatus(verified.response, 200, 'Verification accepted');
  await visibleText(page.getByRole('status'), localeText(ui, lang, 'verify-success'), 'Verified address confirmation');
}

async function playerSession(context, api, project, info, result) {
  const page = await context.newPage();
  const { lang } = project;
  const credentials = { email: playerEmail(), password: password() };
  let since;
  await flow(result, 'Registration rejects mismatched passwords', async () => {
    assertStatus((await open(page, `/${lang}/register`)).response, 200, 'Registration form');
    const { response } = await submit(page, registerForm, {
      ...credentials, password_confirmation: password(), terms: true, privacy: true,
    });
    assertStatus(response, 422, 'Mismatched registration');
    await visibleText(page.locator('#confirmation-error'), localeText(ui, lang, 'register-confirm-error'), 'Password mismatch feedback');
  }, { page, info });
  await flow(result, 'Double-click registration and refresh the localized inbox', async () => {
    since = await registerPlayer(page, lang, credentials, true);
  }, { page, info });
  if (!since) return;
  await flow(result, 'Pending account still sees a sign-in refusal', async () => {
    const denied = await signIn(page, lang, credentials);
    assertStatus(denied.response, 200, 'Refusal renders the login form');
    await expectPath(page, `/${lang}/login`, 'Pending account remains signed out');
    await visibleText(page.getByRole('alert'), localeText(mfaLocale, lang, 'mfa-error'), 'Localized sign-in refusal');
  });
  let verified = false;
  await flow(result, 'Verify the address from the newest addressed message', async () => {
    await verifyPlayer(page, api, lang, credentials.email, since);
    verified = true;
  });
  if (!verified) return false;
  let accountOpened = false;
  await flow(result, 'Account opens in a second tab and remains signed in after reload', async () => {
    const login = await signIn(page, lang, credentials);
    assertStatus(login.response, 200, 'Successful sign-in destination');
    await expectPath(page, `/${lang}/`, 'Signed-in home');
    const second = await context.newPage();
    const account = await open(second, '/account');
    assertStatus(account.response, 200, 'Account overview opens in the second tab');
    await expectPath(second, '/account', 'Account overview destination');
    const overview = localeText('templates/account/overview.{lang}.ftl', lang, 'overview');
    await visibleText(second.locator('main h1'), overview, 'Overview in the second tab');
    assertStatus(await second.reload(), 200, 'Account overview reload');
    await visibleText(second.locator('main h1'), overview, 'Overview persists after reload');
    accountOpened = true;
    await second.close();
  });
  if (!accountOpened) return false;
  await crawl(context, info, project, [`/${lang}/`, '/account'], result, 'player');
  const stillSignedIn = await open(page, '/account');
  if (/\/login$/.test(new URL(page.url()).pathname) && stillSignedIn.chain.length) {
    const login = await signIn(page, lang, credentials);
    if (login.response?.ok() && /\/login$/.test(new URL(page.url()).pathname)) return true;
    expect(false, 'The previously verified player lost its account page during the crawl').toBe(true);
  }
  assertStatus(stillSignedIn.response, 200, 'Account remains available at the end of the crawl');
  checkReport(result);
  return false;
}

test('fresh player registration and account navigation', async ({ browser }, info) => {
  const project = { name: info.project.name, ...info.project.metadata };
  const result = newReport();
  const api = await apiContext();
  try {
    for (let attempt = 0; attempt <= 1; attempt++) {
      const context = await screen(browser, project);
      let vanished;
      try { vanished = await playerSession(context, api, project, info, result); }
      finally { await context.close(); }
      if (!vanished) break;
      if (attempt === 1) {
        result.blocked = 'The staging player disappeared in both attempts';
        break;
      }
      result.flow.push({ label: 'Previously verified staging player disappeared; retrying once with a fresh account', result: 'RETRY' });
      result.pages = [];
      result.pending = [];
      result.otherLocale.clear();
      await ready(api);
    }
  } finally {
    await save(info, project.name, 'player', result);
    await api.dispose();
  }
  if (result.blocked && !info.errors.length) test.skip(true, result.blocked);
});

async function staffLogin(page, api, lang) {
  assertStatus((await open(page, `/${lang}/forgot-password`)).response, 200, 'Staff password-reset request page');
  const since = Date.now();
  const requested = await submit(page, 'main form', { email: staffEmail });
  assertStatus(requested.response, 200, 'Staff password-reset notice');
  await visibleText(page.getByRole('status'), localeText(resetLocale, lang, 'reset-notice'), 'Reset request receipt');
  const link = await mailLink(api, {
    email: staffEmail, since, subject: ['pl', 'en'].map(locale => localeText(resetLocale, locale, 'reset-mail-subject')),
    route: /^\/(pl|en)\/reset-password\/[0-9a-f]+$/i,
  });
  const linkLang = languageOf(link);
  const credentials = { email: staffEmail, password: password() };
  assertStatus((await open(page, link)).response, 200, 'Reset form opens');
  const refused = await submit(page, 'main form', { ...credentials, password_confirmation: password() });
  assertStatus(refused.response, 200, 'Mismatched reset renders a refusal');
  await visibleText(page.getByRole('alert'), localeText(resetLocale, linkLang, 'reset-error'), 'Reset mismatch feedback');
  const reset = await submit(page, 'main form', { ...credentials, password_confirmation: credentials.password });
  assertStatus(reset.response, 200, 'Reset success opens login');
  await expectPath(page, `/${linkLang}/login`, 'Reset redirects to sign-in');

  const login = await signIn(page, lang, credentials);
  assertStatus(login.response, 200, 'Staff sign-in completes its first step');
  let provisioning;
  if (new URL(page.url()).pathname !== `/${lang}/mfa/challenge`) {
    await expectPath(page, `/${lang}/`, 'Account opens before MFA enrollment');
    const beforeMfa = await open(page, '/admin');
    if (beforeMfa.response?.status() !== 404) {
      assertStatus(beforeMfa.response, 403, 'Admin requires MFA before entry');
      await visibleText(page.locator('main h1'), localeText('templates/admin/{lang}.ftl', lang, 'mfa'), 'MFA requirement');
    }
    assertStatus((await open(page, `/${lang}/mfa`)).response, 200, 'MFA settings');
    assertStatus((await submit(page, 'main form[action$="/mfa/enroll"]', { password: credentials.password })).response, 200, 'MFA enrollment');
    provisioning = (await page.locator('main .op-prose').innerText()).trim();
    // Use a fresh period for the challenge following enrollment.
    const enrolledAt = Date.now();
    assertStatus((await submit(page, 'main form[action$="/mfa/confirm"]', {
      password: credentials.password, code: totp(provisioning, enrolledAt),
    })).response, 200, 'MFA confirmation');
    await visibleText(page.locator('main p'), localeText(mfaLocale, lang, 'mfa-save'), 'Recovery-code instructions');
    await expect(page.locator('textarea[readonly]')).toHaveCount(8);
    assertStatus((await signIn(page, lang, credentials)).response, 200, 'Sign-in after enrollment');
    const nextPeriod = 30_000 - (enrolledAt % 30_000) + 1_000;
    await delay(Math.max(0, nextPeriod - (Date.now() - enrolledAt)));
  }
  await expectPath(page, `/${lang}/mfa/challenge`, 'Staff challenge destination');
  const invalidCode = await submit(page, mfaChallenge, { code: 'invalid code' });
  assertStatus(invalidCode.response, 200, 'Invalid MFA code returns to challenge');
  await expectPath(page, `/${lang}/mfa/challenge`, 'Invalid MFA code keeps the user at the challenge');
  await visibleText(page.getByRole('alert'), localeText(mfaLocale, lang, 'mfa-error'), 'Localized MFA refusal');
  let code;
  if (provisioning) {
    if (Date.now() % 30_000 > 26_000) await delay(5_000);
    code = totp(provisioning);
  } else {
    const emailRequested = Date.now();
    assertStatus((await submit(page, 'main form[action$="/mfa/email"]')).response, 200, 'Staff fallback code requested');
    code = await mailCode(api, { email: staffEmail, since: emailRequested, subject: [localeText(mfaLocale, lang, 'mfa-email-subject')] });
  }
  const verified = await submit(page, mfaChallenge, { code });
  assertStatus(verified.response, 200, 'MFA confirmation ends at signed-in home');
  await expectPath(page, `/${lang}/`, 'Signed-in home after challenge');
}

async function waitForStaff(page, api, lang, result) {
  const defaultStart = Date.now();
  const configuredStart = Number(process.env.OTERYN_UX_MAIL_WAIT_SINCE);
  const start = Number.isFinite(configuredStart) && configuredStart > 0 && configuredStart <= defaultStart
    ? configuredStart : defaultStart;
  const deadline = start + 90 * 60_000;
  for (;;) {
    const status = await mailStatus(api);
    const checked = new Date().toISOString();
    result.mailChecks.push({ checked, status });
    if (status === 200) {
      try {
        await ready(api);
        const panel = await get(api, '/admin');
        result.adminEntryStatus = panel.status();
        await panel.dispose();
        await staffLogin(page, api, lang);
        return;
      } catch (error) {
        if (!(error instanceof StagingBlocked)) throw error;
        result.mailChecks.at(-1).detail = safeError(error);
      }
    } else if (![404, 405, 429, 502, 503, 504].includes(status)) {
      expect(status, 'Mailpit list endpoint').toBe(200);
    }
    if (Date.now() >= deadline) {
      throw new StagingBlocked(status === 200
        ? `${result.mailChecks.at(-1).detail} at the end of the 90-minute retry window`
        : `Mailpit list returned HTTP ${status} at the end of the 90-minute retry window`);
    }
    const next = start + (Math.floor((Date.now() - start) / mailInterval) + 1) * mailInterval;
    await delay(Math.max(0, Math.min(deadline, next) - Date.now()));
  }
}

async function observePlayer(browser, staff, api, project) {
  const context = await screen(browser, project);
  const player = await context.newPage();
  const { lang } = project;
  const credentials = { email: playerEmail(), password: password() };
  const playersLocale = 'crates/admin/src/players/{lang}.ftl';
  try {
    const since = await registerPlayer(player, lang, credentials);
    assertStatus((await open(staff, '/admin/players')).response, 200, 'Staff player search');
    const form = 'main form[method=get]';
    assertStatus((await submit(staff, form, { q: 'not an account address' })).response, 400, 'Malformed account search');
    assertStatus((await open(staff, '/admin/players')).response, 200, 'Return to staff player search');
    assertStatus((await submit(staff, form, { q: credentials.email })).response, 200, 'Find the new player');
    const match = staff.locator('main ul a[href^="/admin/players/"]');
    await expect(match).toHaveCount(1);
    const detail = await match.getAttribute('href');
    await match.click();
    await expectPath(staff, detail, 'Staff opens the new player details');
    const verified = staff.locator('dt').filter({ hasText: localeText(playersLocale, lang, 'verified') })
      .locator('xpath=following-sibling::dd[1]');
    await visibleText(verified, localeText(playersLocale, lang, 'no'), 'Staff sees the pending address');
    await verifyPlayer(player, api, lang, credentials.email, since);
    assertStatus(await staff.reload(), 200, 'Reload the same player details');
    await visibleText(verified, localeText(playersLocale, lang, 'yes'), 'Staff sees the verified address');
  } finally {
    await context.close();
  }
}

async function staffCrawls(browser, info, api, session, result) {
  const reports = [];
  for (const project of projects) {
    const data = { ...newReport(), mailChecks: result.mailChecks, flow: [...result.flow] };
    const context = await screen(browser, project);
    try {
      await ready(api);
      await context.addCookies(session);
      const valid = await test.step(project.name, async () => {
        const page = await context.newPage();
        const staffPage = await open(page, '/admin');
        if (staffPage.chain.length && /\/login$/.test(new URL(page.url()).pathname)) return false;
        assertStatus(staffPage.response, 200, 'Staff administration opens with MFA');
        await expectPath(page, '/admin', 'Staff administration destination');
        await crawl(context, info, project, ['/admin', `/${project.lang}/`], data, 'staff');
        const last = await open(page, '/admin');
        if (last.chain.length && /\/login$/.test(new URL(page.url()).pathname)) return false;
        assertStatus(last.response, 200, 'Staff administration remains available after the crawl');
        return true;
      });
      if (!valid) return false;
      reports.push({ project, data });
    } finally {
      await context.close();
    }
  }
  for (const { project, data } of reports) {
    await save(info, project.name, 'staff', data);
    checkReport(data);
  }
  return true;
}

test('platform-admin sign-in and staff navigation', async ({ browser }, info) => {
  // One password reset and MFA session serve all four project combinations.
  test.skip(info.project.name !== projects[0].name, 'Staff project reports share the first project’s authenticated session');
  test.setTimeout(110 * 60_000);
  const first = projects[0];
  const result = { ...newReport(), mailChecks: [] };
  const api = await apiContext();
  try {
    const gate = await get(api, '/admin');
    result.adminEntryStatus = gate.status();
    await gate.dispose();
    for (let attempt = 0; attempt <= 1; attempt++) {
      const context = await screen(browser, first);
      try {
        const page = await context.newPage();
        await flow(result, 'Reset staff password and complete MFA',
          () => waitForStaff(page, api, first.lang, result), { page, info });
        if (result.blocked || info.errors.length) break;
        const panel = await open(page, '/admin');
        result.adminEntryStatus = panel.response?.status();
        if (result.adminEntryStatus === 404) {
          result.blocked = 'Authenticated staff receives HTTP 404 at /admin on the staging host';
          break;
        }
        assertStatus(panel.response, 200, 'Staff administration opens after sign-in');
        const staffView = newReport();
        await flow(staffView, 'Staff sees a new player’s address change from pending to verified',
          () => observePlayer(browser, page, api, first));
        result.flow.push(...staffView.flow);
        const session = await context.cookies(origin);
        if (await staffCrawls(browser, info, api, session, result)) return;
      } finally {
        await context.close();
      }
      if (attempt === 0) result.flow.push({ label: 'Staff session disappeared during the crawl; retrying once', result: 'RETRY' });
      else result.blocked = 'Previously authenticated staff session disappeared in both attempts';
    }
  } finally {
    await api.dispose();
  }
  for (const project of projects) await save(info, project.name, 'staff', result);
  if (result.blocked && !info.errors.length) test.skip(true, result.blocked);
});
