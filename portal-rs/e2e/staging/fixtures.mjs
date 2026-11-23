import { test as base, expect } from '@playwright/test';
import { randomBytes } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { open } from 'node:fs/promises';
import { join } from 'node:path';
import { startStagingBrowserProxy } from './browser-proxy.mjs';
import { accessHeaders, requestTo, safePath, stagingHost, stagingOrigin, stagingProxy, stagingURL } from './transport.mjs';

export const test = base.extend({
  browser: [async ({ playwright, launchOptions, headless }, use) => {
    const proxy = await startStagingBrowserProxy();
    try {
      const browser = await playwright.chromium.launch({ ...launchOptions, headless, proxy: proxy.options });
      try {
        await use(browser);
      } finally {
        await browser.close();
      }
    } finally {
      await proxy.close();
    }
  }, { scope: 'worker' }],

  request: async ({ playwright }, use) => {
    const request = await playwright.request.newContext({ baseURL: stagingOrigin, proxy: stagingProxy });
    try {
      await use(request);
    } finally {
      await request.dispose();
    }
  },

  readiness: [async ({ request }, use) => {
    await waitUntilReady(request);
    await use();
  }, { auto: true, timeout: 305_000 }],

  page: async ({ page, context }, use, info) => {
    await context.route('**/*', async route => {
      const request = route.request();
      const url = new URL(request.url());
      if (url.host !== stagingHost || url.protocol !== 'https:') {
        await route.continue();
        return;
      }
      if (request.method() === 'POST' &&
          (/^\/(?:pl|en)\/register$/.test(url.pathname) || url.pathname === '/register/abuse/challenge')) {
        await registrationSlot(url.pathname.endsWith('/challenge') ? 'challenge' : 'submit');
      }
      await route.continue({ headers: { ...request.headers(), ...accessHeaders() } });
    });
    await use(page);
    if (info.status === info.expectedStatus || page.isClosed()) return;
    const path = info.outputPath('failure.png');
    try {
      // Exclude rendered credentials and MFA factors from failure evidence.
      const access = [
        process.env.STAGING_CF_ACCESS_CLIENT_ID,
        process.env.STAGING_CF_ACCESS_CLIENT_SECRET,
      ];
      const text = await page.locator('body').textContent();
      if (access.some(value => text.includes(value))) return;
      await page.screenshot({
        path,
        timeout: 5_000,
        mask: ['input', 'textarea', 'main .op-prose'].map(selector => page.locator(selector)),
      });
      await info.attach('screenshot', { path, contentType: 'image/png' });
    } catch {
      console.log('A failure screenshot could not be captured.');
    }
  },

  // Public and input-validation cases use an isolated identity without registering it.
  account: async ({}, use) => {
    await use({ email: `bb-${Date.now().toString(36)}-${randomBytes(9).toString('hex')}@example.test`, password: password() });
  },

  lang: async ({}, use, info) => use(info.project.metadata.locale),

  mail: async ({ request }, use) => {
    const blocker = process.env.OTERYN_BLACKBOX_MAIL_BLOCKER;
    test.skip(Boolean(blocker), blocker);
    await use(request);
  },
});

export { expect };

export function password() {
  return `Oteryn!a9-${randomBytes(18).toString('base64url')}`;
}

export async function waitUntilReady(request) {
  const deadline = Date.now() + 300_000;
  let actual;
  do {
    try {
      const response = await requestTo(request, 'GET', '/readyz', { timeout: Math.max(1, Math.min(12_000, deadline - Date.now())) });
      actual = response.status();
      await response.dispose();
      if (actual === 200) return;
    } catch {
      actual = 'network error';
    }
    const remaining = deadline - Date.now();
    if (remaining > 0) await delay(Math.min(5_000, remaining));
  } while (Date.now() < deadline);
  throw new Error(`GET /readyz expected HTTP 200 within five minutes. Last result: ${actual}.`);
}

export async function visit(page, target, status = 200, { destination = target } = {}) {
  const url = stagingURL(target);
  const route = safePath(target);
  const navigate = () => page.goto(url.href, { waitUntil: 'domcontentloaded' });
  let response;
  let retryReason;
  try {
    response = await navigate();
    if (response?.status() === 502) retryReason = 'HTTP 502';
  } catch (error) {
    if (!/net::ERR_(?:CONNECTION_|NETWORK_CHANGED|INTERNET_DISCONNECTED|NAME_NOT_RESOLVED|TIMED_OUT|HTTP2_PROTOCOL_ERROR|PROXY_CONNECTION_FAILED|TUNNEL_CONNECTION_FAILED)/.test(error.message)) {
      throw new Error(`GET ${route} did not complete: ${error.message.match(/net::ERR_[A-Z0-9_]+/)?.[0] ?? error.name}.`);
    }
    retryReason = 'network error';
  }
  if (retryReason) {
    console.log(`Retrying GET ${route} once after ${retryReason}.`);
    await delay(2_000);
    try {
      response = await navigate();
    } catch (error) {
      throw new Error(`GET ${route} did not complete on retry: ${error.message.match(/net::ERR_[A-Z0-9_]+/)?.[0] ?? error.name}.`);
    }
  }
  const isExpectedPath = response && new URL(response.url()).pathname === stagingURL(destination).pathname;
  expect(response?.status(), `GET ${route} (final path ${response ? safePath(response.url()) : 'none'})`).toBe(status);
  expect(Boolean(isExpectedPath), `GET ${route} stayed on the expected route`).toBe(true);
  return response;
}

export async function submit(page, target, status, values = {}, selector = 'main form') {
  const form = page.locator(selector).first();
  for (const [name, value] of Object.entries(values)) {
    const input = form.locator(`[name="${name}"]`);
    if (typeof value === 'boolean') await input.setChecked(value);
    else await input.fill(value);
  }
  const pathname = stagingURL(target).pathname;
  // The deployed registration form allows four minutes for browser proof.
  const timeout = pathname.endsWith('/register') ? 300_000 : 65_000;
  const [response] = await Promise.all([
    page.waitForResponse(response => response.request().method() === 'POST' &&
      new URL(response.url()).pathname === pathname, { timeout }),
    form.locator('button[type=submit], button:not([type])').first().click(),
  ]);
  if (response.status() === 429) {
    const retryAfter = response.headers()['retry-after'];
    const wait = /^\d+$/.test(retryAfter ?? '') ? ` Retry after ${retryAfter} seconds.` : '';
    throw new Error(`POST ${safePath(target)} was rate limited (HTTP 429).${wait}`);
  }
  expect(response.status(), `POST ${safePath(target)}`).toBe(status);
  return response;
}

export async function atPath(page, path) {
  await expect.poll(() => safePath(page.url()), { message: 'Browser destination' }).toBe(path);
}

export async function localized(page, lang, title) {
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.locator('main')).toBeVisible();
  if (title) await expect(page.getByRole('heading', { level: 1 })).toHaveText(title);
  // Missing Fluent messages render as their keys. Match standalone labels so
  // ordinary hyphenated prose, dates and player-provided content remain valid.
  const keys = await page.locator('main').evaluate(main => main.innerText.split('\n')
    .map(line => line.trim())
    .filter(line => /^(?:account|auth|error|legal|wiki|rankings|news|mfa|passkeys|register|reset|shell|thais|verify)-[a-z]+(?:-[a-z0-9]+)*$/.test(line)));
  expect(keys, 'Visible untranslated keys').toEqual([]);
}

// Share separate source budgets for the challenge and the actual registration
// POST. Budget at the network boundary because browser proof time varies.
async function registrationSlot(kind) {
  const interval = 31_000;
  const directory = process.env.OTERYN_BLACKBOX_SLOTS;
  for (let slot = Math.floor(Date.now() / interval); ; slot++) {
    try {
      const file = await open(join(directory, `${kind}-${slot}`), 'wx');
      await file.close();
      const wait = slot * interval - Date.now();
      if (wait > 0) await delay(wait);
      return;
    } catch (error) {
      if (error.code !== 'EEXIST') throw error;
    }
  }
}
