import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join } from 'node:path';
import { requestGate } from './request-gate.mjs';
import { isGameUnavailable } from './game-unavailable.mjs';
import { credentialURL, diagnosticURL, publicURL, redactURLs } from './crawl-url.mjs';

const require = createRequire(import.meta.url);
const { test, expect } = require('@playwright/test');
const budgetPath = join(process.env.OTERYN_SMOKE_RUN_DIR, 'budget.json');
const legalPath = /^\/(?:pl\/|en\/)?legal\/(?:terms|privacy|cookies)\/?$/;
const redirects = [301, 302, 303, 307, 308];
const assetSelector = 'img[src], script[src], link[rel~="stylesheet"][href], link[rel~="icon"][href], link[rel~="preload"][href], link[rel~="modulepreload"][href]';

let axeSource;
try {
  axeSource = readFileSync(require.resolve('axe-core/axe.min.js'), 'utf8');
} catch (error) {
  if (error.code !== 'MODULE_NOT_FOUND') throw error;
}

function canonical(value, base) {
  try {
    const url = new URL(value, base);
    url.hash = '';
    return url;
  } catch {
    return null;
  }
}

function pathPattern(url) {
  return url.pathname.replace(/\/$/, '')
    .replace(/\/(news|events|characters|guilds|houses|profiles|puzzles)\/[^/]+/g, '/$1/:id')
    .replace(/\/wiki\/([^/]+)\/[^/]+/g, '/wiki/$1/:id')
    .replace(/\/bazaar\/(items\/)?(?:\d+|[\da-f-]{36})(?=\/|$)/gi, '/bazaar/$1:id')
    .replace(/\/(?:\d+|[\da-f-]{36})(?=\/|$)/gi, '/:id');
}

function checkHeaders(url, headers, defect, note) {
  const checks = [
    ['strict-transport-security', /\bmax-age\s*=\s*[1-9]\d*/i],
    ['x-content-type-options', /^nosniff$/i],
    ['content-security-policy', /\S/],
    ['referrer-policy', /\S/],
    ['x-robots-tag', /\bnoindex\b/i],
  ];
  for (const [name, pattern] of checks) {
    const value = headers.get(name);
    if (!pattern.test(value || '')) defect('headers', url, `${name} = ${value || '(missing)'}`);
  }
  note(url, `headers: ${checks.map(([name, pattern]) => `${name}=${pattern.test(headers.get(name) || '') ? 'pass' : 'fail'}`).join(', ')}`);
  const cookies = headers.getSetCookie();
  if (!cookies.length) note(url, 'no Set-Cookie header');
  for (const cookie of cookies) {
    const [pair, ...attributes] = cookie.split(';');
    const name = pair.split('=', 1)[0];
    const flags = attributes.map(value => value.trim().toLowerCase());
    const missing = [];
    for (const flag of ['secure', 'httponly']) {
      if (!flags.includes(flag)) missing.push(flag);
    }
    if (!flags.some(flag => /^samesite=(strict|lax|none)$/.test(flag))) missing.push('SameSite');
    if (!name.startsWith('__Host-')) missing.push('__Host- prefix');
    if (name.startsWith('__Host-') &&
        (!flags.includes('path=/') || flags.some(flag => flag.startsWith('domain=')))) {
      missing.push('host-only scope with Path=/');
    }
    if (missing.length) defect('headers', url, `cookie ${name} missing ${missing.join(', ')}`);
    else note(url, `cookie ${name} passed Secure, HttpOnly, SameSite and __Host- scope checks`);
  }
}

function inspectPage({ language, assetSelector }) {
  const main = document.querySelector('main, [role="main"]') || document.body;
  const strings = new Set();
  const add = value => {
    const trimmed = value?.trim().replace(/\s+/g, ' ');
    if (trimmed) strings.add(trimmed);
  };
  add(document.title);
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  while (walker.nextNode()) {
    const { parentElement, textContent } = walker.currentNode;
    if (!parentElement.closest('script, style, template, noscript, [hidden], [aria-hidden="true"]') &&
        !parentElement.closest(`[lang]:not([lang|="${language}"]):not(html)`)) add(textContent);
  }
  for (const element of document.querySelectorAll('[aria-label], [placeholder], img[alt]')) {
    if (element.closest(`[lang]:not([lang|="${language}"]):not(html)`)) continue;
    for (const name of ['aria-label', 'placeholder', 'alt']) add(element.getAttribute(name));
  }
  const legal = main.querySelector('article')?.innerText.trim() || '';
  return {
    lang: document.documentElement.lang,
    title: document.title.trim(),
    headings: document.querySelectorAll('h1').length,
    width: Math.max(document.body.scrollWidth, document.documentElement.scrollWidth),
    viewport: document.documentElement.clientWidth,
    missingAlt: [...document.images].filter(img => !img.hasAttribute('alt'))
      .map(img => img.currentSrc || img.src || '(no src)'),
    links: [...document.querySelectorAll('a[href], area[href]')].map(link => link.href),
    assets: [...document.querySelectorAll(assetSelector)].map(element => element.src || element.href),
    strings: [...strings],
    legal,
  };
}

const englishUI = /^(?:log in|sign in|sign up|create account|privacy policy|terms of service|cookie policy|skip to (?:main )?content|world status|forgot password\??|read more|search|home|news|rankings|download|learn more)$/i;
const polishUI = /^(?:zaloguj się|załóż konto|utwórz konto|polityka prywatności|regulamin|polityka cookies|przejdź do treści|status świata|nie pamiętasz hasła\??|czytaj więcej|szukaj|strona główna|aktualności|rankingi|pobierz|dowiedz się więcej)$/i;
const englishWords = /(?<!\p{L})(?:the|and|your|with|from|this|where|for|you|will|are|have|which)(?!\p{L})/giu;
const polishWords = /(?<!\p{L})(?:oraz|który|które|których|jest|się|przez|jako|możesz|swoje|dla|będzie|został|można)(?!\p{L})/giu;
const translationKey = /^(?:[a-z][\w-]*\.){2,}[a-z][\w-]*$|^(?:register|login|shell|thais|nav|home|error|page|account|footer|header|wiki|legal|search)(?:-[a-z][a-z0-9]+){1,5}$|\{\{\s*[\w.-]+\s*\}\}/;
const mojibake = /\uFFFD|Ã.|Â[\u0080-\u00bf]|[ÄÅĂ][\u0080-\u00bf‚„…†‡‰Š‹ŒŽ‘’“”•–—™š›œžźśłż]/u;

function checkLanguage(url, data, language, defect) {
  if (data.lang.toLowerCase().split('-')[0] !== language) {
    defect('translation', url, `html lang=${JSON.stringify(data.lang)}, expected ${language}`);
  }
  for (const value of data.strings) {
    const quote = JSON.stringify(value.slice(0, 180));
    if (translationKey.test(value)) defect('translation', url, `raw key ${quote}`);
    if (mojibake.test(value)) defect('translation', url, `broken encoding ${quote}`);
    const english = value.match(englishWords)?.length || 0;
    const polish = value.match(polishWords)?.length || 0;
    if (language === 'pl' ? englishUI.test(value) || (english >= 3 && polish === 0)
      : polishUI.test(value) || (polish >= 3 && english === 0)) {
      defect('translation', url, `wrong-language text ${quote}`);
    }
  }
}

async function checkRegistration(page, url, language, defect) {
  const form = page.locator(`form[method="post"][action="/${language}/register"]`);
  if (await form.count() !== 1 || !await form.isVisible()) {
    defect('other', url, 'expected one visible registration form');
    return;
  }
  for (const [name, type] of Object.entries({
    email: 'email', password: 'password', password_confirmation: 'password',
    terms: 'checkbox', privacy: 'checkbox', _csrf: 'hidden', work_token: 'hidden', work_nonce: 'hidden',
  })) {
    const field = form.locator(`input[name="${name}"][type="${type}"]`);
    if (await field.count() !== 1 || (type !== 'hidden' && !await field.isVisible())) {
      defect('other', url, `registration field ${name} (${type}) missing or unusable`);
    }
  }
  for (const suffix of ['terms', 'privacy']) {
    if (!await form.locator(`a[href="/${language}/legal/${suffix}"]:visible`).count()) {
      defect('other', url, `registration link to ${suffix} missing`);
    }
  }
  if (!await form.locator('[type="submit"]:visible').count()) {
    defect('other', url, 'registration submit control missing');
  }
}

async function openPage(context, source, report, note, request, responseNote) {
  const page = await context.newPage();
  const session = await context.newCDPSession(page);
  const documentResponses = new Map();
  const pending = new Set();
  const policyBlocks = new Set();
  const expectedDocumentStatuses = new Map();
  let closing = false;
  const requestFailure = (url, reason) => report('console/CSP', source, `${url}: ${reason}`);
  page.on('console', message => {
    const value = message.text();
    if (message.type() === 'error' || /mixed content/i.test(value)) {
      const location = message.location();
      const status = expectedDocumentStatuses.get(location.url);
      const documentStatusMessage = /^Failed to load resource: the server responded with a status of (\d+)(?: \([^)]*\))?$/.exec(value);
      if (!policyBlocks.has(location.url) && !(status && Number(documentStatusMessage?.[1]) === status)) {
        requestFailure(location.url || source, value);
      }
    }
  });
  page.on('pageerror', error => requestFailure(source, error.message));
  page.on('requestfailed', failed => {
    if (!policyBlocks.has(failed.url())) requestFailure(failed.url(), failed.failure()?.errorText);
  });
  await page.exposeBinding('smokeCSPViolation', (_, directive, blocked) => {
    requestFailure(blocked || source, `CSP ${directive}`);
  });
  await page.addInitScript(() => {
    addEventListener('securitypolicyviolation', event => {
      void window.smokeCSPViolation(event.violatedDirective, event.blockedURI);
    });
  });
  await session.send('Fetch.enable', {
    patterns: [{ urlPattern: 'http://*' }, { urlPattern: 'https://*' }],
  });
  session.on('Fetch.requestPaused', event => {
    const task = (async () => {
      const { requestId, resourceType, request: intercepted } = event;
      const url = new URL(intercepted.url);
      if (credentialURL(url)) {
        policyBlocks.add(url.href);
        note(source, `excluded URL ${diagnosticURL(url)}`);
        await session.send('Fetch.failRequest', { requestId, errorReason: 'BlockedByClient' }).catch(() => {});
        return;
      }
      try {
        const response = await request(url, intercepted.method);
        if (resourceType === 'Document') documentResponses.set(url.href, response);
        const expectedUnavailable = resourceType === 'Document' && responseNote(url, response);
        if (expectedUnavailable) expectedDocumentStatuses.set(url.href, response.status);
        if (!expectedUnavailable && (response.status < 200 || response.status >= 400)) {
          report('broken links', source, `HTTP ${response.status} ${url.href}`);
        }
        const responseHeaders = [...response.headers]
          .filter(([name]) => !['content-encoding', 'content-length', 'transfer-encoding', 'set-cookie'].includes(name))
          .map(([name, value]) => ({ name, value }));
        await session.send('Fetch.fulfillRequest', {
          requestId,
          responseCode: response.status,
          responseHeaders,
          body: response.body.toString('base64'),
        }).catch(error => {
          if (!closing || !error.message.includes('Target page, context or browser has been closed')) {
            throw error;
          }
        });
      } catch (error) {
        policyBlocks.add(url.href);
        requestFailure(url.href, error.message);
        await session.send('Fetch.failRequest', { requestId, errorReason: 'BlockedByClient' }).catch(() => {});
      }
    })();
    pending.add(task);
    void task.finally(() => pending.delete(task));
  });
  return {
    page,
    documentResponses,
    async close() {
      await Promise.allSettled(pending);
      closing = true;
      await page.close();
      await Promise.allSettled(pending);
    },
  };
}

test('anonymous localized production crawl', async ({ context, baseURL, viewport, userAgent }, info) => {
  const [language] = info.project.name.split('-');
  const home = new URL(`/${language}/`, baseURL);
  const registration = new URL(`/${language}/register`, baseURL);
  const legal = ['terms', 'privacy', 'cookies'].map(name => new URL(`/${language}/legal/${name}`, baseURL));
  const www = new URL(baseURL);
  const api = new URL(baseURL);
  const production = home.hostname === 'oteryn.com';
  if (production) {
    www.hostname = 'www.oteryn.com';
    api.hostname = 'api.oteryn.com';
  }
  const origins = new Set([home.origin, www.origin, api.origin]);
  const request = requestGate(origins, language, userAgent, budgetPath);
  const findings = new Map();
  const notes = new Set();
  const defect = (category, url, evidence) => {
    const finding = { category, url: diagnosticURL(url), evidence: redactURLs(evidence) };
    findings.set(JSON.stringify(finding), finding);
  };
  const note = (url, evidence) => notes.add(JSON.stringify({ url: diagnosticURL(url), evidence: redactURLs(evidence) }));

  const expectedGameRefusal = (url, result) => url.origin === home.origin && isGameUnavailable(url, result);

  function responseNote(url, result) {
    if (legalPath.test(url.pathname) && [404, 410].includes(result.status)) {
      note(url, `legal content unavailable (HTTP ${result.status})`);
      return true;
    }
    if (expectedGameRefusal(url, result)) {
      note(url, 'Game data unavailable (HTTP 503, localized portal view, no-store)');
      return true;
    }
    return false;
  }

  async function probe(source, target, document = true) {
    const seen = new Set();
    let url = target;
    try {
      while (!seen.has(url.href) && seen.size < 10) {
        seen.add(url.href);
        const result = await request(url);
        const location = result.headers.get('location');
        if (redirects.includes(result.status) && location) {
          url = new URL(location, url);
          if (!origins.has(url.origin)) return result;
          if (credentialURL(url)) {
            note(source, `excluded redirect destination ${diagnosticURL(url)}`);
            return;
          }
          continue;
        }
        if ((result.status < 200 || result.status >= 400) && !(document && responseNote(url, result))) {
          defect('broken links', source, `HTTP ${result.status} ${url.href}`);
        }
        return { ...result, url };
      }
      defect('broken links', source, `redirect loop or over 10 hops from ${target.href}`);
    } catch (error) {
      defect('broken links', source, `${url.href}: ${error.message}${error.cause ? ` (${error.cause.message})` : ''}`);
    }
  }

  async function edgeCheck(path, check) {
    const url = new URL(path, home);
    try {
      const result = await request(url);
      const evidence = check(result);
      if (evidence) defect('other', url, evidence);
      else note(url, `edge check HTTP ${result.status}${result.headers.has('location') ? ` Location=${diagnosticURL(result.headers.get('location'))}` : ''}`);
    } catch (error) {
      defect('other', url, `${error.message}${error.cause ? ` (${error.cause.message})` : ''}`);
    }
  }

  if (!axeSource) note(home, 'axe-core unavailable in e2e dependencies; axe check skipped');
  const queue = [];
  const queued = new Set();
  const inspected = new Set();
  const patterns = new Map();
  function enqueue(url) {
    if (queued.has(url.href)) return;
    const pattern = pathPattern(url);
    const count = patterns.get(pattern) || 0;
    if (count >= 3) return;
    queued.add(url.href);
    patterns.set(pattern, count + 1);
    queue.push(url);
  }
  for (const url of [home, registration, ...legal]) enqueue(url);

  let innerChecked = false;
  for (const url of queue) {
    if (JSON.parse(readFileSync(budgetPath, 'utf8'))[0] >= 2000) {
      defect('other', url, 'shared 2000-request budget exhausted; crawl incomplete');
      break;
    }
    const first = await probe(url, url);
    if (!first || (first.status >= 400 && !expectedGameRefusal(first.url, first))) continue;
    if (!first.headers.get('content-type')?.includes('text/html')) {
      defect('other', url, `expected a rendered HTML page, received ${first.headers.get('content-type') || '(no content-type)'}`);
      continue;
    }
    const browser = await openPage(context, url.href, defect, note, request, responseNote);
    const { page, documentResponses } = browser;
    try {
      await page.goto(url.href, { waitUntil: 'load', timeout: 90_000 });
      // Give post-load script, CSP and failed-request events a bounded settling window.
      await page.waitForLoadState('networkidle', { timeout: 10_000 }).catch(() => {});
      const final = new URL(page.url());
      if (url.href === registration.href && final.pathname.replace(/\/$/, '') !== registration.pathname) {
        defect('other', url, `registration redirected to ${final.href}`);
      }
      if (url.href === home.href && !new RegExp(`^/${language}/?$`).test(final.pathname)) {
        defect('other', url, `localized home redirected to ${final.href}`);
      }
      const result = documentResponses.get(final.href);
      if (!result || (result.status >= 400 && !expectedGameRefusal(final, result)) ||
          inspected.has(final.href)) continue;
      inspected.add(final.href);
      console.log(`LIVE PAGE ${info.project.name} ${diagnosticURL(final)}`);
      const data = await page.evaluate(inspectPage, { language, assetSelector });
      if (final.pathname === `/${language}` || final.pathname.startsWith(`/${language}/`)) {
        checkLanguage(final, data, language, defect);
      }
      if (!data.title) defect('other', final, 'document title is empty');
      if (data.headings !== 1) defect('a11y', final, `expected one h1, found ${data.headings}`);
      if (viewport.width === 390 && data.width > Math.min(viewport.width, data.viewport)) {
        defect('other', final, `horizontal overflow: ${data.width}px content, ${viewport.width}px viewport`);
      }
      for (const asset of data.missingAlt) defect('a11y', final, `image missing alt: ${asset}`);
      if (legalPath.test(final.pathname)) {
        note(final, data.legal ? `legal article has content (${data.legal.length} characters)` : 'legal article empty');
      }
      if (final.pathname.replace(/\/$/, '') === registration.pathname) {
        const previousFindings = findings.size;
        await checkRegistration(page, final, language, defect);
        if (findings.size === previousFindings) note(final, 'registration form, fields and legal links present');
      }
      if (url.href === home.href || (!innerChecked && !legalPath.test(final.pathname))) {
        checkHeaders(final, result.headers, defect, note);
        if (url.href !== home.href) innerChecked = true;
      }
      if (axeSource) {
        await page.evaluate(axeSource);
        const violations = await page.evaluate(async () => {
          const { violations } = await window.axe.run();
          return violations.filter(item => ['serious', 'critical'].includes(item.impact))
            .map(item => ({ id: item.id, impact: item.impact, nodes: item.nodes.map(node => node.target) }));
        });
        for (const { id, impact, nodes } of violations) {
          defect('a11y', final, `axe ${impact} ${id}: ${JSON.stringify(nodes)}`);
        }
      }
      for (const value of new Set([...data.links, ...data.assets])) {
        const target = canonical(value, final);
        if (!target) continue;
        if (final.protocol === 'https:' && target.protocol === 'http:' && data.assets.includes(value)) {
          defect('console/CSP', final, `mixed content asset ${target.href}`);
        }
        if (target.origin !== home.origin || target.href === final.href) continue;
        if (!publicURL(target, origins)) {
          note(final, `excluded stateful URL ${diagnosticURL(target)}`);
          continue;
        }
        const link = data.links.includes(value);
        const locale = target.pathname.match(/^\/(pl|en)(?:\/|$)/)?.[1];
        const document = link && !data.assets.includes(value);
        const response = await probe(final, target, document);
        if (document && response && (response.status < 400 || expectedGameRefusal(response.url, response)) &&
            response.headers.get('content-type')?.includes('text/html') &&
            (!locale || locale === language)) enqueue(target);
      }
    } catch (error) {
      defect('other', url, `page inspection failed: ${error.message.split('\n')[0]}`);
    } finally {
      await browser.close();
    }
  }
  if (!innerChecked) defect('headers', home, 'no rendered inner page was available for header checks');

  await edgeCheck('/robots.txt', ({ status, body }) =>
    status === 200 && /^\s*user-agent\s*:/im.test(body.toString()) ? null : `robots.txt HTTP ${status}, expected rules`);
  await edgeCheck('/.well-known/security.txt', ({ status, body }) => {
    if (status === 404) {
      note(new URL('/.well-known/security.txt', home), 'security.txt pending (HTTP 404)');
      return null;
    }
    return status === 200 && /^contact:/im.test(body.toString()) ? null : `security.txt HTTP ${status}, expected Contact field`;
  });
  if (production) {
    await edgeCheck(www, ({ status, headers }) => {
      const location = headers.get('location');
      return redirects.includes(status) && location && new URL(location, www).origin === home.origin
        ? null : `HTTP ${status}, Location=${location ? diagnosticURL(location) : "(missing)"}; expected apex redirect`;
    });
    for (const path of ['/', '/pl/', '/en/', '/favicon.ico']) {
      await edgeCheck(new URL(path, api), ({ status }) => status === 404 ? null : `HTTP ${status}; expected 404`);
    }
  } else {
    note(home, 'www and API host checks apply to the production apex');
  }

  const [requests] = JSON.parse(readFileSync(budgetPath, 'utf8'));
  const summary = { project: info.project.name, pages: inspected.size, requests, runDir: process.env.OTERYN_SMOKE_RUN_DIR };
  for (const item of notes) console.log(`LIVE NOTE ${info.project.name} ${item}`);
  for (const item of findings.values()) console.log(`LIVE DEFECT ${info.project.name} ${JSON.stringify(item)}`);
  console.log(`LIVE SUMMARY ${JSON.stringify(summary)}`);
  await info.attach('live-report', {
    body: JSON.stringify({ ...summary, notes: [...notes].map(item => JSON.parse(item)), defects: [...findings.values()] }, null, 2),
    contentType: 'application/json',
  });
  expect(findings.size, `${info.project.name}: ${findings.size} hard findings; see LIVE DEFECT entries`).toBe(0);
});
