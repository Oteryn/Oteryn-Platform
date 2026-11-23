import { test } from '@playwright/test';
import { Secret, TOTP } from 'otpauth';
import { appendFileSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const READ = 10_000;
const FORM = 15_000;
const MFA = 45_000;
const LANGUAGES = ['pl', 'en'];
const NEWS_HEADING = { pl: 'Aktualności', en: 'News' };
const ACCOUNT_LINK = '[data-account-link][href="/account"]';
const LOCALE = 'select#locale[name="locale"]';
const VISIBILITY = ['public_account_association', 'public_status_visible'];

class MonitorFailure extends Error {}

function requireThat(condition, message) {
  if (!condition) throw new MonitorFailure(message);
}

test('Production portal synthetic monitor', async ({ browser, context, baseURL }, info) => {
  const { lang, checkPublicEndpoints } = info.project.metadata;
  const target = new URL(baseURL);
  const origin = target.origin;
  const runDirectory = process.env.MONITOR_RUN_DIRECTORY;
  const counter = join(runDirectory, 'requests');
  const diagnostics = join(runDirectory, 'diagnostics');
  const localAuth = process.env.MONITOR_LOCAL_AUTH === '1';
  const { MONITOR_EMAIL: email, MONITOR_PASSWORD: password, MONITOR_TOTP_SECRET: secret } = process.env;
  if (localAuth) {
    try {
      requireThat(['http:', 'https:'].includes(target.protocol)
        && ['localhost', '127.0.0.1', '[::1]'].includes(target.hostname)
        && email?.endsWith('@example.invalid') && password, 'disposable account required');
      const { query } = await import('../staff-rbac-helpers.mjs');
      const [account] = await query("SELECT id FROM accounts WHERE email=$1 AND status='active'", [email]);
      requireThat(account, 'seeded active account required');
    } catch {
      appendFileSync(diagnostics, `${info.project.name} failed at disposable auth fixture\n`);
      throw new MonitorFailure('Start the harness and provide a disposable seeded account');
    }
  }
  const page = await context.newPage();
  const cdp = await context.newCDPSession(page);
  const browserCdp = await browser.newBrowserCDPSession();
  const { targetInfo } = await cdp.send('Target.getTargetInfo');
  const { frameTree } = await cdp.send('Page.getFrameTree');
  const mainFrame = frameTree.frame.id;
  // Workers run sequentially, with a single request allowance shared by both projects.
  let requestCount = Number(readFileSync(counter, 'utf8'));
  requireThat(Number.isInteger(requestCount) && requestCount >= 0 && requestCount <= 60,
    'invalid shared request count');
  const assets = new Map();
  let allowedPost;
  let activeStep = 'browser setup';
  let nextScreenshot = 0;
  const problems = [];

  function record(message) {
    problems.push({ step: activeStep, message });
  }

  async function rejectTarget({ targetInfo: target }) {
    if (target.targetId === targetInfo.targetId) return;
    record(`unexpected browser target blocked (${target.type})`);
    // Never resume a child. Close it where Chromium supports closing that target.
    await browserCdp.send('Target.closeTarget', { targetId: target.targetId }).catch(() => {});
  }

  for (const session of [browserCdp, cdp]) {
    session.on('Target.attachedToTarget', rejectTarget);
    await session.send('Target.setAutoAttach', {
      autoAttach: true, waitForDebuggerOnStart: true, flatten: true,
      filter: ['browser', 'browser_ui', 'tab'].map((type) => ({ type, exclude: true })).concat({}),
    });
  }

  // Context routing also sees a popup's first navigation, before Playwright
  // can associate it with a page. Only the known top-level frame may proceed.
  await context.route('**/*', async (route) => {
    let main = false;
    try {
      main = route.request().frame() === page.mainFrame();
    } catch {
      // The request belongs to a target whose frame is not registered.
    }
    if (main) return route.fallback();
    record('unexpected browser request blocked');
    await route.abort('blockedbyclient');
  });

  page.on('console', (message) => {
    if (message.type() === 'error') record('browser console error');
  });
  page.on('pageerror', () => record('uncaught browser exception'));
  page.on('requestfailed', (request) => {
    const code = request.failure()?.errorText;
    if (request.isNavigationRequest() && /^net::ERR_[A-Z_0-9]+$/.test(code ?? '')) record(code);
  });
  page.on('response', (response) => {
    if (response.status() >= 500) {
      const { pathname } = new URL(response.url());
      // Avoid logging dynamic paths, URLs with query strings or response bodies.
      const location = safeReads.has(pathname) ? pathname : response.request().resourceType();
      record(`HTTP ${response.status()} (${location})`);
    }
  });
  await context.exposeBinding('reportMonitorCsp', () => record('CSP violation'));
  await context.addInitScript(() => {
    document.addEventListener('securitypolicyviolation', () => { void window.reportMonitorCsp(); });
  });
  await context.routeWebSocket(/.*/, (socket) => {
    record('unexpected WebSocket');
    return socket.close();
  });

  const safeReads = new Set(['/healthz', '/pl/', '/en/']);
  if (localAuth) {
    for (const path of ['/account', '/account/security', '/login', '/']) safeReads.add(path);
    for (const lang of LANGUAGES) {
      for (const suffix of ['login', 'account', 'account/privacy', 'account/security',
        'password/change', 'account/sessions', 'mfa/challenge']) {
        safeReads.add(`/${lang}/${suffix}`);
      }
    }
  }

  // Chromium pauses every hop, including redirects, before sending it.
  cdp.on('Fetch.requestPaused', async (event) => {
    const { requestId, request, resourceType, responseStatusCode, responseHeaders } = event;
    const { origin: targetOrigin, pathname, search } = new URL(request.url);
    const portal = targetOrigin === origin;
    const beacon = targetOrigin === 'https://static.cloudflareinsights.com'
      && pathname.startsWith('/beacon.min.js') && resourceType === 'Script';
    const asset = (portal && pathname.startsWith('/static/')
      && ['Stylesheet', 'Script'].includes(resourceType)) || beacon;
    try {
      if (responseStatusCode !== undefined || event.responseErrorReason) {
        if (responseStatusCode === 200) {
          const { body, base64Encoded } = await cdp.send('Fetch.getResponseBody', { requestId });
          const cached = {
            responseCode: responseStatusCode,
            responseHeaders: responseHeaders.filter(({ name }) =>
              !['content-encoding', 'content-length', 'set-cookie', 'transfer-encoding'].includes(name.toLowerCase())),
            body: base64Encoded ? body : Buffer.from(body).toString('base64'),
          };
          assets.set(request.url, cached);
          await cdp.send('Fetch.fulfillRequest', { requestId, ...cached });
        } else {
          await cdp.send('Fetch.continueResponse', { requestId });
        }
        return;
      }

      if (request.method === 'POST' && portal && pathname === '/cdn-cgi/rum'
          && ['Ping', 'XHR', 'Fetch'].includes(resourceType)) {
        // Acknowledge Cloudflare browser telemetry locally without sending it.
        await cdp.send('Fetch.fulfillRequest', { requestId, responseCode: 204 });
        return;
      }
      if (resourceType === 'Document' && event.frameId !== mainFrame) {
        record('unexpected frame navigation blocked');
        await cdp.send('Fetch.failRequest', { requestId, errorReason: 'BlockedByClient' });
        return;
      }
      if (localAuth && request.method === 'POST' && portal && !search && pathname === allowedPost?.path
          && resourceType === 'Document') {
        const { fields } = allowedPost;
        allowedPost = undefined;
        if (!matchesFormPost(request, fields)) {
          record('unexpected form payload blocked');
          await cdp.send('Fetch.failRequest', { requestId, errorReason: 'BlockedByClient' });
          return;
        }
      } else if (request.method !== 'GET' && request.method !== 'HEAD') {
        record('unexpected mutation blocked');
        await cdp.send('Fetch.failRequest', { requestId, errorReason: 'BlockedByClient' });
        return;
      } else if (resourceType === 'Document' && (targetOrigin !== origin || !safeReads.has(pathname) || (!localAuth && search))) {
        record('unexpected navigation blocked');
        await cdp.send('Fetch.failRequest', { requestId, errorReason: 'BlockedByClient' });
        return;
      } else if (resourceType !== 'Document' && !asset) {
        // Keep the browser journey small. CSS and JS still load and execute.
        await cdp.send('Fetch.fulfillRequest', { requestId, responseCode: 204 });
        return;
      }
      if (asset && assets.has(request.url)) {
        await cdp.send('Fetch.fulfillRequest', { requestId, ...assets.get(request.url) });
        return;
      }
      if (requestCount >= 60) {
        record('60-request limit reached');
        await cdp.send('Fetch.failRequest', { requestId, errorReason: 'BlockedByClient' });
        return;
      }
      requestCount += 1;
      writeFileSync(counter, String(requestCount));
      await cdp.send('Fetch.continueRequest', { requestId, interceptResponse: asset });
    } catch (error) {
      // Chromium can cancel a subresource when its document is replaced.
      const cancelled = /Invalid InterceptionId|Invalid requestId|Request is already handled/i.test(error.message);
      if (!page.isClosed() && !(cancelled && resourceType !== 'Document')) {
        const method = /Protocol error \(Fetch\.([A-Za-z]+)\)/.exec(error.message)?.[1];
        record(`browser request interception failed (${resourceType}, ${method ?? 'unknown'})`);
      }
    }
  });

  await cdp.send('Fetch.enable', { patterns: [{ urlPattern: 'http*' }] });

  function matchesFormPost(request, expected) {
    const contentType = Object.entries(request.headers)
      .find(([name]) => name.toLowerCase() === 'content-type')?.[1];
    if (!/^application\/x-www-form-urlencoded(?:;|$)/i.test(contentType ?? '')
        || typeof request.postData !== 'string') return false;
    const entries = [...new URLSearchParams(request.postData)];
    return entries.length === expected.size && new Set(entries.map(([name]) => name)).size === expected.size
      && entries.every(([name, value]) => expected.get(name) === value);
  }

  async function step(name, budget, run, publicHome) {
    activeStep = name;
    page.setDefaultTimeout(budget);
    page.setDefaultNavigationTimeout(budget);
    const started = performance.now();
    return test.step(name, async () => {
      let result;
      let problem;
      try {
        result = await run();
      } catch (error) {
        problem = error instanceof MonitorFailure ? error.message
          : error.name === 'TimeoutError' ? 'timed out' : 'browser operation failed';
      }
      const diagnostic = problems.splice(0).map(({ step, message }) => `${step}: ${message}`);
      if (performance.now() - started > budget) diagnostic.push(`exceeded ${budget} ms budget`);
      if (problem) diagnostic.push(problem);
      if (diagnostic.length) {
        if (publicHome && page.url() === new URL(publicHome, origin).href) {
          await page.screenshot({ path: info.outputPath(`public-home-${++nextScreenshot}.png`),
            timeout: 2_000 }).catch(() => {});
        }
        appendFileSync(diagnostics, `${info.project.name} failed at ${name}\n`);
        for (const reason of [...new Set(diagnostic)]) {
          appendFileSync(diagnostics, `Diagnostic: ${reason}\n`);
        }
        throw new Error(`MONITOR_FAILURE: ${name} (${[...new Set(diagnostic)].join('; ')})`);
      }
      return result;
    });
  }

  function currentPath() {
    return new URL(page.url()).pathname;
  }

  function checkResponse(response) {
    requireThat(response?.status() >= 200 && response.status() < 400,
      `unexpected document status ${response?.status() ?? 'missing'}`);
  }

  async function go(path) {
    const response = await page.goto(path);
    checkResponse(response);
    return response;
  }

  async function submit(form, path, fields = []) {
    const csrf = await form.locator('input[name="_csrf"]').inputValue();
    requireThat(csrf, 'form CSRF field is empty');
    allowedPost = { path, fields: new Map([['_csrf', csrf], ...fields]) };
    try {
      const [navigation, click] = await Promise.allSettled([
        page.waitForNavigation(),
        form.locator('button[type="submit"], button:not([type])').first().click(),
      ]);
      if (click.status === 'rejected') throw click.reason;
      if (navigation.status === 'rejected') throw navigation.reason;
      checkResponse(navigation.value);
    } finally {
      allowedPost = undefined;
    }
  }

  async function loginForm() {
    const form = page.locator('main form[action$="/login"]');
    await form.locator('#email[type="email"]').waitFor();
    await form.locator('#password[type="password"]').waitFor();
    return form;
  }

  const home = `/${lang}/`;
  const privacy = `/${lang}/account/privacy`;
  const sessions = `/${lang}/account/sessions`;

  const publicChecks = [];
  if (checkPublicEndpoints) {
    publicChecks.push(['Public health', async () => {
      const response = await go('/healthz');
      requireThat(response.ok() && currentPath() === '/healthz' && await response.text() === 'ok',
        'unexpected health response');
    }]);
    if (localAuth) {
      publicChecks.push(['Anonymous login form', async () => {
        await go('/login');
        await loginForm();
      }]);
    }
  }
  publicChecks.push([`Anonymous ${lang} home`, async () => {
    await go(home);
    requireThat(page.url() === new URL(home, origin).href, 'unexpected homepage destination');
    requireThat(await page.locator('html').getAttribute('lang') === lang, 'wrong homepage language');
    await page.locator('main #hero-title').and(
      page.getByRole('heading', { name: NEWS_HEADING[lang], exact: true, level: 1 }),
    ).waitFor();
  }, home]);
  const failures = [];
  for (const [name, check, screenshot] of publicChecks) {
    try {
      await step(name, READ, check, screenshot);
    } catch (error) {
      failures.push(error);
    }
  }
  if (failures.length) throw failures[0];

  async function finish() {
    await step('Close browser page', READ, () => page.close());
    appendFileSync(diagnostics, `${info.project.name} passed\n`);
  }

  if (!localAuth) {
    await finish();
    return;
  }

  async function openMobileMenu() {
    const toggle = page.locator('#portal-menu-toggle');
    if (await toggle.isVisible() && await toggle.getAttribute('aria-expanded') === 'false') {
      await toggle.click();
    }
  }

  await step(`${lang} home to login form`, READ, async () => {
    await openMobileMenu();
    checkResponse((await Promise.all([
      page.waitForNavigation(),
      page.locator(`.om-login[href="${home}login"]`).click(),
    ]))[0]);
    await loginForm();
  });

  await step(`${lang} sign in`, FORM, async () => {
    const form = await loginForm();
    await form.locator('#email').fill(email);
    await form.locator('#password').fill(password);
    await submit(form, `${home}login`, [['email', email], ['password', password]]);
    const challenge = currentPath() === `${home}mfa/challenge`;
    requireThat(!secret || challenge, 'configured TOTP challenge was skipped');
    requireThat(challenge || currentPath() === home, 'sign-in refused');
  });

  if (currentPath() === `${home}mfa/challenge`) {
    await step(`${lang} TOTP challenge`, MFA, async () => {
      requireThat(secret, 'MONITOR_TOTP_SECRET is required for this account');
      const form = page.locator(`form[action="${home}mfa/challenge"]`);
      const totp = new TOTP({ secret: Secret.fromBase32(secret) });
      // The account is shared by sequential projects and the portal consumes each counter.
      const totpCounter = join(runDirectory, 'totp-counter');
      const previous = Number(readFileSync(totpCounter, { encoding: 'utf8', flag: 'a+' }) || 0);
      const now = Date.now();
      const boundary = now - now % 30_000 + 30_000;
      // A manual run queued immediately after another run can share a counter.
      const minimum = previous ? (previous + 1) * 30_000 : boundary;
      const next = Math.max(minimum, boundary - now < 3_000 ? boundary : 0);
      if (next > now) await delay(next - now + 250);
      const timestamp = Date.now();
      writeFileSync(totpCounter, String(Math.floor(timestamp / 30_000)));
      const code = totp.generate({ timestamp });
      await form.locator('#code[autocomplete="one-time-code"]').fill(code);
      await submit(form, `${home}mfa/challenge`, [['code', code]]);
      requireThat(currentPath() === home, 'TOTP sign-in refused');
    });
  }

  await step(`${lang} account overview`, READ, async () => {
    await openMobileMenu();
    await page.locator(ACCOUNT_LINK).waitFor();
    await go('/account');
    await page.locator('main dl > dd').first().waitFor();
    await page.locator('main #characters').waitFor();
  });
  await step(`${lang} account settings`, READ, async () => {
    await go(privacy);
    const locale = await page.locator(LOCALE).inputValue();
    requireThat(LANGUAGES.includes(locale), 'unexpected saved preference');
    for (const name of VISIBILITY) {
      await page.locator(`input[type="checkbox"][name="${name}"]`).waitFor();
    }
  });
  await step(`${lang} account security`, READ, async () => {
    const response = await go(`${home}account/security`);
    requireThat(currentPath() === `${home}password/change` && response.request().redirectedFrom(),
      'unexpected security redirect');
    await page.locator('main input#current[name="current_password"]').waitFor();
  });

  await step(`${lang} current session listed`, READ, async () => {
    await go(sessions);
    await page.locator('main li > p').filter({
      hasText: lang === 'pl' ? /^Ta sesja$/ : /^This session$/,
    }).waitFor();
  });
  await step(`${lang} log out`, FORM, async () => {
    await go(home);
    await openMobileMenu();
    await page.locator(ACCOUNT_LINK).click();
    const form = page.locator(`#podglad-konta[open] form[action="${home}logout"]`);
    await submit(form, `${home}logout`);
    requireThat(currentPath() === home, 'unexpected logout destination');
  });
  await step(`${lang} protected page redirects to login`, READ, async () => {
    const response = await go(sessions);
    requireThat(/^\/(?:pl\/|en\/)?login$/.test(currentPath()) &&
      response.request().redirectedFrom(), 'protected page did not redirect to login');
    await loginForm();
  });
  await finish();
});
