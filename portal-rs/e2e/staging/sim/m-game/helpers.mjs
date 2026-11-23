import { randomBytes } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { expect, test as base } from '@playwright/test';

export const origin = 'https://staging.oteryn.com';
const host = new URL(origin).host;
const accessHeaders = {
  'cf-access-client-id': process.env.STAGING_CF_ACCESS_CLIENT_ID,
  'cf-access-client-secret': process.env.STAGING_CF_ACCESS_CLIENT_SECRET,
};
const mailPath = '/__mailpit/api/v1/messages';

export async function staging(request, path, options = {}) {
  const url = new URL(path, origin);
  if (url.origin !== origin) throw new Error('Staging API requests must stay on the staging origin.');
  try {
    return await request.fetch(url.href, {
      ...options,
      headers: { ...options.headers, ...accessHeaders },
      maxRedirects: 0,
      timeout: 12_000,
    });
  } catch {
    throw new Error(`Staging ${options.method ?? 'GET'} request failed before an HTTP response.`);
  }
}

async function stagingPage(context) {
  await context.route('**/*', async route => {
    const request = route.request();
    const url = new URL(request.url());
    if (url.host !== host || url.protocol !== 'https:') return route.continue();
    await route.continue({ headers: { ...request.headers(), ...accessHeaders } });
  });

  const page = await context.newPage();
  const cdp = await context.newCDPSession(page);
  // Playwright routes the first hop. Chromium pauses redirects too, so
  // a cross-origin hop cannot inherit the Access headers.
  cdp.on('Fetch.requestPaused', async ({ requestId, request }) => {
    try {
      const requestHeaders = Object.entries(request.headers);
      const isAccessHeader = ([name]) => Object.hasOwn(accessHeaders, name.toLowerCase());
      if (new URL(request.url).origin !== origin && requestHeaders.some(isAccessHeader)) {
        const headers = requestHeaders.filter(header => !isAccessHeader(header))
          .map(([name, value]) => ({ name, value }));
        await cdp.send('Fetch.continueRequest', { requestId, headers });
        return;
      }
      await cdp.send('Fetch.continueRequest', { requestId });
    } catch {
      // Chromium can close the target while an intercepted request is pending.
    }
  });
  await cdp.send('Fetch.enable', { patterns: [{ urlPattern: '*' }] });
  return page;
}

export const test = base.extend({
  stagingReady: [async ({ request }, use) => {
    await expect.poll(async () => {
      try {
        const response = await staging(request, '/readyz');
        const status = response.status();
        await response.dispose();
        return status;
      } catch {
        return 0;
      }
    }, { timeout: 300_000, intervals: [2_000, 5_000, 10_000] }).toBe(200);
    await use();
  }, { auto: true, timeout: 310_000 }],
  screen: async ({ context, stagingReady }, use) => {
    const page = await stagingPage(context);
    await use(page);
  },
});

export { expect };

export function labels(lang) {
  return (pl, en) => lang === 'pl' ? pl : en;
}

export async function check(response, expected, info, page) {
  const url = new URL(response.url());
  if (response.status() !== expected && page) {
    await page.screenshot({ path: info.outputPath('public-page.png'), timeout: 5_000 }).catch(() => {});
  }
  expect(response.status(), `${url.pathname}${url.search}`).toBe(expected);
}

export async function mailStatus(request) {
  const response = await staging(request, mailPath);
  const status = response.status();
  await response.dispose();
  return status;
}

export async function newestMail(request, address, subject, since) {
  const deadline = Date.now() + 60_000;
  while (Date.now() < deadline) {
    const response = await staging(request, `${mailPath}?limit=100`);
    expect(response.status(), 'Mailpit list').toBe(200);
    const { messages } = await response.json();
    await response.dispose();
    const newest = messages
      .sort((a, b) => Date.parse(b.Created) - Date.parse(a.Created))
      .slice(0, 100)
      .find(message => Date.parse(message.Created) >= since && subject.test(message.Subject)
        && message.To.some(to => to.Address.toLowerCase() === address.toLowerCase()));
    if (newest) {
      const detail = await staging(request, `/__mailpit/api/v1/message/${encodeURIComponent(newest.ID)}`);
      expect(detail.status(), 'Mailpit message').toBe(200);
      const content = await detail.json();
      await detail.dispose();
      return content;
    }
    await delay(3_000);
  }
  throw new Error('The expected staging message did not arrive within 60 seconds.');
}

export function mailLink(message, acceptsPath) {
  const candidates = `${message.Text || ''}\n${message.HTML || ''}`.match(/https?:\/\/[^\s<>"']+/g) ?? [];
  for (const candidate of candidates) {
    const url = new URL(candidate.replaceAll('&amp;', '&'));
    if ((url.origin === origin || ['localhost', '127.0.0.1'].includes(url.hostname))
      && acceptsPath.test(url.pathname)) {
      return `${url.pathname}${url.search}`;
    }
  }
  throw new Error('The expected staging link was missing from the message.');
}

export async function register(page, lang) {
  const email = `sim-m-game-${randomBytes(8).toString('hex')}-1@example.test`;
  const password = `Oteryn!${randomBytes(18).toString('hex')}aA2`;
  const form = page.locator('main form');
  await form.locator('[name=email]').fill(email);
  await form.locator('[name=password]').fill(password);
  await form.locator('[name=password_confirmation]').fill(password);
  await form.locator('[name=terms]').check();
  await form.locator('[name=privacy]').check();
  const since = Date.now() - 2_000;
  const [response] = await Promise.all([
    page.waitForResponse(reply => reply.request().method() === 'POST'
      && new URL(reply.url()).pathname === `/${lang}/register`, { timeout: 25_000 }),
    form.locator('button[type=submit]').click(),
  ]);
  return { email, password, since, response };
}

export async function signIn(page, lang, { email, password }) {
  await page.goto(`/${lang}/login`);
  const form = page.locator('main form').filter({ has: page.locator('input[name=password]') });
  await form.locator('[name=email]').fill(email);
  await form.locator('[name=password]').fill(password);
  await form.locator('button[type=submit]').click();
}
