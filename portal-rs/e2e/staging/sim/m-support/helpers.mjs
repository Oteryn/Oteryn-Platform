import { randomBytes } from 'node:crypto';
import { setTimeout as sleep } from 'node:timers/promises';
import { expect, test as base } from '@playwright/test';

const origin = 'https://staging.oteryn.com';
const id = process.env.STAGING_CF_ACCESS_CLIENT_ID;
const secret = process.env.STAGING_CF_ACCESS_CLIENT_SECRET;
if (!id || !secret) throw new Error('Staging Cloudflare Access credentials are required');
const stagingHeaders = {
  'CF-Access-Client-Id': id,
  'CF-Access-Client-Secret': secret,
};
const mailRoot = '/__mailpit/api/v1';
const run = randomBytes(6).toString('hex');
let accounts = 0;

export const test = base.extend({
  page: async ({ context, page }, use) => {
    await restrictToStaging(context);
    await use(page);
  },
});

export async function restrictToStaging(context) {
  await context.route('**/*', async route => {
    const browserRequest = route.request();
    const url = new URL(browserRequest.url());
    if (url.host !== 'staging.oteryn.com' || url.protocol !== 'https:') return route.continue();

    // A header override on route.continue() follows redirects. Fetch one hop
    // and let Chromium handle the response without those request headers.
    let response;
    try {
      response = await route.fetch({
        headers: { ...browserRequest.headers(), ...stagingHeaders },
        maxRedirects: 0,
        timeout: 20_000,
      });
      const location = response.headers().location;
      if (response.status() >= 300 && response.status() < 400 && location
          && new URL(location, browserRequest.url()).origin !== origin) return await route.abort();
      await route.fulfill({ response });
    } catch {
      await route.abort().catch(() => {});
    } finally {
      await response?.dispose().catch(() => {});
    }
  });
}

// APIRequestContext errors can include outgoing headers. Surface only the
// method and path template chosen by the test when transport fails.
export async function request(api, method, path, options = {}) {
  const url = new URL(path, origin);
  if (url.origin !== origin) throw new Error('Expected a staging path');
  try {
    return await api.fetch(url.href, {
      ...options, method, maxRedirects: 0, timeout: 12_000,
      headers: { origin, ...options.headers, ...stagingHeaders },
    });
  } catch {
    throw new Error(`Staging ${method} transport failed`);
  }
}

export async function status(api, method, path, expected, options) {
  const response = await request(api, method, path, options);
  expect(response.status(), `${method} ${path}`).toBe(expected);
  return response;
}

export async function ready(api) {
  await expect.poll(async () => {
    try {
      return (await request(api, 'GET', '/readyz')).status();
    } catch {
      return 0;
    }
  }, { timeout: 300_000, intervals: [2_000, 5_000], message: 'Staging /readyz' }).toBe(200);
}

export async function supportRouteStatus(api, path) {
  const code = (await request(api, 'OPTIONS', path)).status();
  expect([404, 405], `Route availability for ${path}`).toContain(code);
  return code === 404 ? 'unavailable' : 'mounted';
}

export async function mailStatus(api) {
  return (await request(api, 'GET', `${mailRoot}/messages`)).status();
}

export async function mail(api, address, since, match) {
  const deadline = Date.now() + 60_000;
  while (Date.now() < deadline) {
    const list = await status(api, 'GET', `${mailRoot}/messages`, 200);
    const { messages } = await list.json();
    const candidates = messages.sort((a, b) => Date.parse(b.Created) - Date.parse(a.Created))
      .slice(0, 100).filter(entry =>
      Date.parse(entry.Created) >= since && entry.To?.some(recipient => recipient.Address === address));
    for (const entry of candidates) {
      const message = await status(api, 'GET', `${mailRoot}/message/${encodeURIComponent(entry.ID)}`, 200);
      const { Text, HTML } = await message.json();
      const value = match([Text, HTML].filter(Boolean).join('\n'));
      if (value) return value;
    }
    await sleep(2_000);
  }
  throw new Error('No matching message to this test account arrived within 60 seconds');
}

export function newAccount() {
  const suffix = `${run}-${++accounts}`;
  return {
    email: `sim-m-support-${suffix}@example.test`,
    password: `Qf7!${randomBytes(18).toString('base64url')}`,
    label: `Sim ${suffix}`,
  };
}

export async function fill(form, values) {
  for (const [name, value] of Object.entries(values)) {
    const field = form.locator(`[name="${name}"]`);
    if (typeof value === 'boolean') await field.setChecked(value);
    else if (await field.evaluate(el => el.tagName === 'SELECT')) await field.selectOption(value);
    else await field.fill(value);
  }
}

export async function submit(form, values) {
  await fill(form, values);
  await form.locator('button').click();
}

export async function formData(form) {
  return form.evaluate(element => ({
    path: new URL(element.action).pathname,
    values: Object.fromEntries(new FormData(element)),
  }));
}

export class SharedRateLimit extends Error {}

export async function startRegistration(page, lang, account) {
  for (let attempt = 0; attempt < 3; attempt++) {
    const outcome = await submitRegistration(page, lang, account);
    if (outcome.rateLimited === false) {
      expect(outcome.challengeCode, 'Registration challenge HTTP status').toBe(200);
      expect(outcome.code, 'Registration submission HTTP status').toBe(303);
      await expect(page).toHaveURL(new RegExp(`/${lang}/register/check-inbox$`), { timeout: 45_000 });
      return outcome.since;
    }
    if (attempt === 2) break;
    await sleep((outcome.retryAfter + 1) * 1_000);
  }
  throw new SharedRateLimit('Registration was rate limited after two delayed retries');
}

async function submitRegistration(page, lang, account) {
  const registration = await page.goto(`/${lang}/register`);
  expect(registration.status()).toBe(200);
  const form = page.locator('main form[action$="/register"]');
  await form.locator('[name="work_token"]').waitFor({ state: 'attached' });
  const since = Date.now() - 1_000;
  const [challenge, response] = await Promise.all([
    page.waitForResponse(response => new URL(response.url()).pathname === '/register/abuse/challenge'
      && response.request().method() === 'POST', { timeout: 30_000 }),
    page.waitForResponse(response => new URL(response.url()).pathname === `/${lang}/register`
      && response.request().method() === 'POST', { timeout: 60_000 }),
    submit(form, {
      email: account.email, password: account.password, password_confirmation: account.password,
      terms: true, privacy: true,
    }),
  ]);
  const limited = [challenge, response].find(reply => reply.status() === 429);
  const retryAfter = Number(limited?.headers()['retry-after']);
  return {
    since,
    code: response.status(),
    challengeCode: challenge.status(),
    rateLimited: Boolean(limited),
    retryAfter: Number.isFinite(retryAfter) && retryAfter > 0 ? Math.min(60, retryAfter) : 60,
  };
}

export async function register(page, lang) {
  const account = newAccount();
  const since = await startRegistration(page, lang, account);
  const verification = await mail(page.request, account.email, since, body =>
    body.match(/\/(?:pl|en)\/verify-email\?token=[a-f0-9]+/)?.[0]);
  try {
    const response = await page.goto(verification);
    expect(response.status(), 'Email verification page').toBe(200);
    await submit(page.locator('main form[action*="/verify-email?"]'), {});
    await expect(page.getByRole('status')).toHaveText(lang === 'pl'
      ? 'Twój adres e-mail został zweryfikowany.'
      : 'Your e-mail has been verified.');
  } catch {
    throw new Error('The account email verification did not complete');
  }
  await page.goto(`/${lang}/login`);
  await submit(page.locator('main form[action$="/login"]'), accountFields(account));
  await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
  return account;
}

function accountFields({ email, password }) {
  return { email, password };
}

export async function openAccountLink(page, lang, section) {
  await page.goto(`/${lang}/`);
  const menu = page.locator('#portal-menu-toggle');
  if (await menu.isVisible()) await menu.click();
  const link = page.locator(`a[href="/${lang}/support/${section}"]`);
  if (!await link.first().isVisible()) await page.locator('[data-account-link]').first().click();
  await link.filter({ visible: true }).first().click();
  await expect(page).toHaveURL(new RegExp(`/${lang}/support/${section}$`));
}
