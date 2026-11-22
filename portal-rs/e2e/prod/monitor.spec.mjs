import { test } from '@playwright/test';
import { Secret, TOTP } from 'otpauth';
import { setTimeout as delay } from 'node:timers/promises';

const READ = 10_000;
const FORM = 15_000;
const MFA = 45_000;
const LANGUAGES = ['pl', 'en'];
const ACCOUNT_LINK = '[data-account-link][href="/account"]';
const LOCALE = 'select#locale[name="locale"]';
const VISIBILITY = ['public_account_association', 'public_status_visible'];

class MonitorFailure extends Error {}

function requireThat(condition, message) {
  if (!condition) throw new MonitorFailure(message);
}

test('Production portal synthetic monitor', async ({ page, context, baseURL }, info) => {
  const origin = new URL(baseURL).origin;
  const anonymousOnly = process.env.MONITOR_ANONYMOUS_ONLY === '1';
  const { MONITOR_EMAIL: email, MONITOR_PASSWORD: password, MONITOR_TOTP_SECRET: secret } = process.env;
  const cdp = await context.newCDPSession(page);
  let requestCount = 0;
  const assets = new Map();
  let allowedPost;
  let lastTotpCounter;
  let activeStep = 'browser setup';
  let nextScreenshot = 0;
  const problems = [];

  function record(message) {
    problems.push({ step: activeStep, message });
  }

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

  const safeReads = new Set(['/account', '/account/security', '/login', '/healthz', '/']);
  for (const lang of LANGUAGES) {
    for (const suffix of ['', 'login', 'account', 'account/privacy', 'account/security',
      'password/change', 'account/sessions', 'mfa/challenge']) {
      safeReads.add(`/${lang}/${suffix}`);
    }
  }

  // Chromium pauses every hop, including redirects, before sending it.
  await cdp.send('Fetch.enable', { patterns: [{ urlPattern: 'http*' }] });
  cdp.on('Fetch.requestPaused', async (event) => {
    const { requestId, request, resourceType, responseStatusCode, responseHeaders } = event;
    const { origin: targetOrigin, pathname } = new URL(request.url);
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
      if (request.method === 'POST' && targetOrigin === origin && pathname === allowedPost
          && resourceType === 'Document') {
        allowedPost = undefined;
      } else if (request.method !== 'GET' && request.method !== 'HEAD') {
        record('unexpected mutation blocked');
        await cdp.send('Fetch.failRequest', { requestId, errorReason: 'BlockedByClient' });
        return;
      } else if (resourceType === 'Document' && (targetOrigin !== origin || !safeReads.has(pathname))) {
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

  async function submit(form, path) {
    allowedPost = path;
    try {
      const [response] = await Promise.all([
        page.waitForNavigation(),
        form.locator('button[type="submit"], button:not([type])').first().click(),
      ]);
      checkResponse(response);
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

  for (const lang of LANGUAGES) {
    const home = `/${lang}/`;
    await step(`Anonymous ${lang} home`, READ, () => go(home), home);
  }
  await step('Public health', READ, async () => {
    const response = await go('/healthz');
    requireThat(response.ok() && currentPath() === '/healthz', 'unexpected health response');
  });
  await step('Anonymous login form', READ, async () => {
    await go('/login');
    await loginForm();
  });

  if (anonymousOnly) {
    console.log(`Monitor completed with ${requestCount} requests.`);
    return;
  }
  await step('Monitor credentials configured', READ, () => {
    requireThat(email && password, 'MONITOR_EMAIL and MONITOR_PASSWORD are required');
  });

  for (const lang of LANGUAGES) {
    const home = `/${lang}/`;
    const privacy = `/${lang}/account/privacy`;
    const sessions = `/${lang}/account/sessions`;
    await step(`${lang} home and login form`, READ, async () => {
      await go(home);
      checkResponse((await Promise.all([
        page.waitForNavigation(),
        page.locator(`.om-login[href="${home}login"]`).click(),
      ]))[0]);
      await loginForm();
    }, home);

    await step(`${lang} sign in`, FORM, async () => {
      const form = await loginForm();
      await form.locator('#email').fill(email);
      await form.locator('#password').fill(password);
      await submit(form, `${home}login`);
      requireThat([home, `${home}mfa/challenge`].includes(currentPath()), 'sign-in refused');
    });

    if (currentPath() === `${home}mfa/challenge`) {
      await step(`${lang} TOTP challenge`, MFA, async () => {
        requireThat(secret, 'MONITOR_TOTP_SECRET is required for this account');
        const form = page.locator(`form[action="${home}mfa/challenge"]`);
        const totp = new TOTP({ secret: Secret.fromBase32(secret) });
        // The portal consumes a TOTP counter, so the second locale needs a fresh code.
        const now = Date.now();
        const next = Math.max(lastTotpCounter === undefined ? 0 : (lastTotpCounter + 1) * 30_000,
          30_000 - now % 30_000 < 3_000 ? now + 30_000 - now % 30_000 : 0);
        if (next > now) await delay(next - now + 250);
        const timestamp = Date.now();
        lastTotpCounter = Math.floor(timestamp / 30_000);
        await form.locator('#code[autocomplete="one-time-code"]').fill(totp.generate({ timestamp }));
        await submit(form, `${home}mfa/challenge`);
        requireThat(currentPath() === home, 'TOTP sign-in refused');
      });
    }

    await step(`${lang} account overview`, READ, async () => {
      await page.locator(ACCOUNT_LINK).waitFor();
      await go('/account');
      await page.locator('main dl > dd').first().waitFor();
      await page.locator('main #characters').waitFor();
    });
    const original = await step(`${lang} account settings`, READ, async () => {
      await go(privacy);
      const locale = await page.locator(LOCALE).inputValue();
      requireThat(LANGUAGES.includes(locale), 'unexpected saved preference');
      return { locale, visibility: await readVisibility() };
    });
    await step(`${lang} account security`, READ, async () => {
      const response = await go(`${home}account/security`);
      requireThat(currentPath() === `${home}password/change` && response.request().redirectedFrom(),
        'unexpected security redirect');
      await page.locator('main input#current[name="current_password"]').waitFor();
    });

    async function readVisibility() {
      return Promise.all(VISIBILITY.map((name) => page.locator(`input[name="${name}"]`).isChecked()));
    }

    async function saveLocale(value) {
      requireThat(/^\/(pl|en)\/account\/privacy$/.test(currentPath()), 'privacy page unavailable');
      requireThat((await readVisibility()).every((checked, i) => checked === original.visibility[i]),
        'visibility settings changed');
      await page.locator(LOCALE).selectOption(value);
      const savedPath = `/${value}/account/privacy`;
      await submit(page.locator('main form').filter({ has: page.locator(LOCALE) }), currentPath());
      requireThat(currentPath() === savedPath, 'preference save did not redirect');
      requireThat(await page.locator(LOCALE).inputValue() === value, 'saved preference differs');
      requireThat((await readVisibility()).every((checked, i) => checked === original.visibility[i]),
        'visibility settings changed');
    }

    const alternate = LANGUAGES.find((value) => value !== original.locale);
    await step(`${lang} reopen account settings`, READ, () => go(privacy));
    let toggleError;
    try {
      await step(`${lang} save alternate language`, FORM, () => {
        requireThat(requestCount <= 52, 'request budget cannot cover preference restoration');
        return saveLocale(alternate);
      });
    } catch (error) {
      toggleError = error;
    } finally {
      try {
        await step(`${lang} restore original language`, FORM, async () => {
          if (toggleError) await go(privacy);
          if (await page.locator(LOCALE).inputValue() !== original.locale) {
            await saveLocale(original.locale);
          }
          requireThat(await page.locator(LOCALE).inputValue() === original.locale,
            'original preference was not restored');
        });
      } catch (restoreError) {
        if (!toggleError) throw restoreError;
        throw new Error(`${toggleError.message}\n${restoreError.message}`);
      }
    }
    if (toggleError) throw toggleError;

    await step(`${lang} current session listed`, READ, async () => {
      await go(sessions);
      await page.locator('main li > p').filter({
        hasText: lang === 'pl' ? /^Ta sesja$/ : /^This session$/,
      }).waitFor();
    });
    await step(`${lang} log out`, FORM, async () => {
      await go(home);
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
  }
  console.log(`Monitor completed with ${requestCount} requests.`);
});
