import { test as base, expect } from '@playwright/test';
import { createHmac, randomBytes } from 'node:crypto';
import { setTimeout as wait } from 'node:timers/promises';

export const origin = 'https://staging.oteryn.com';
const access = {
  'CF-Access-Client-Id': process.env.STAGING_CF_ACCESS_CLIENT_ID,
  'CF-Access-Client-Secret': process.env.STAGING_CF_ACCESS_CLIENT_SECRET,
};
const mailPath = '/__mailpit/api/v1/messages';
const run = randomBytes(6).toString('hex');
const registrationChallenges = new WeakMap();
let sequence = 0;

export const password = () => `Sim!${randomBytes(18).toString('hex')}`;
export const player = () => ({
  email: `sim-m-identity-${run}-${++sequence}@example.test`,
  password: password(),
});

export const test = base.extend({
  context: async ({ context }, use) => {
    await routeStaging(context);
    await use(context);
  },
  ready: [async ({ request }, use) => {
    await expect.poll(async () => {
      try {
        return (await http(request, 'GET', '/readyz')).status();
      } catch {
        return 'unreachable';
      }
    }, { timeout: 300_000, intervals: [2_000, 5_000, 10_000] }).toBe(200);
    await use();
  }, { auto: true }],
});
export { expect };
export const registrationChallenge = page => registrationChallenges.get(page);

export async function routeStaging(context) {
  await context.route('**/*', async route => {
    const request = route.request();
    if (new URL(request.url()).origin !== origin) return route.continue();
    let response;
    try {
      // Fetch a single hop so header overrides never carry over to a redirect.
      response = await route.fetch({
        headers: { ...request.headers(), ...access },
        maxRedirects: 0,
        timeout: 15_000,
      });
      if (request.method() === 'POST'
          && new URL(request.url()).pathname === '/register/abuse/challenge') {
        registrationChallenges.set(request.frame().page(), {
          status: response.status(),
          retryAfter: response.headers()['retry-after'],
          bits: response.ok() ? await response.json().then(body => body.bits).catch(() => undefined) : undefined,
        });
      }
      const location = response.headers().location;
      if (response.status() >= 300 && response.status() < 400 && location
          && new URL(location, request.url()).origin !== origin) {
        await route.abort();
      } else {
        await route.fulfill({ response });
      }
    } catch {
      await route.abort().catch(() => {});
    } finally {
      await response?.dispose().catch(() => {});
    }
  });
}

// Playwright transport exceptions include request headers. Keep only the safe path.
export async function http(request, method, path, options = {}) {
  const url = new URL(path, origin);
  if (url.origin !== origin) throw new Error('Only the staging origin is allowed.');
  try {
    for (let attempt = 0; ; attempt++) {
      const result = await request.fetch(url.href, {
        ...options,
        headers: { ...options.headers, ...access },
        method,
        maxRedirects: 0,
        timeout: 12_000,
      });
      if (result.status() !== 429 || attempt === 2) return result;
      const delay = budgetDelay(result.headers()['retry-after']);
      await result.dispose();
      await wait(delay);
    }
  } catch {
    throw new Error(`Staging transport failed for ${method} ${url.pathname.replace(/\/[a-f0-9]{64}$/i, '/[redacted]')}`);
  }
}

function budgetDelay(retryAfter) {
  const seconds = Number(retryAfter);
  return Number.isFinite(seconds) && seconds > 0 ? (Math.min(60, seconds) + 1) * 1000 : 61_000;
}

async function afterBudget(response) {
  await wait(budgetDelay((await response.allHeaders())['retry-after']));
}

export async function open(page, path) {
  for (let attempt = 0; ; attempt++) {
    const result = await page.goto(path, { waitUntil: 'domcontentloaded' });
    if (result.status() !== 429 || attempt === 2) return result;
    await afterBudget(result);
  }
}

export async function postForm(page, path, values, method = 'POST') {
  const csrf = await page.locator('main form [name="_csrf"]').first().inputValue();
  return http(page.request, method, path, {
    headers: { origin },
    form: { ...values, _csrf: csrf },
  });
}

export async function submit(form, values = {}, doubleClick = false) {
  for (const [name, value] of Object.entries(values)) {
    const field = form.locator(`[name="${name}"]`);
    if (typeof value === 'boolean') await field.setChecked(value);
    else await field.fill(value);
  }
  const button = form.locator('button[type=submit], button:not([type])').first();
  if (doubleClick) await button.dblclick();
  else await button.click();
}

export async function submitWithBudget(form, values, doubleClick = false) {
  const page = form.page();
  const start = page.url();
  const endpoint = new URL(await form.getAttribute('action') || start, start).pathname;
  for (let attempt = 0; ; attempt++) {
    const response = page.waitForResponse(result =>
      result.request().method() === 'POST' && new URL(result.url()).pathname === endpoint,
    { timeout: 90_000 });
    const [result] = await Promise.all([response, submit(form, values, doubleClick)]);
    const challenge = endpoint.endsWith('/register') ? registrationChallenge(page) : undefined;
    if (attempt === 2) return result;
    if (result.status() === 429) await afterBudget(result);
    else if (challenge?.status === 429) await wait(budgetDelay(challenge.retryAfter));
    else return result;
    await page.goto(start, { waitUntil: 'domcontentloaded' });
  }
}

export async function nav(page, path) {
  const toggle = page.locator('#portal-menu-toggle');
  if (await toggle.isVisible() && await toggle.getAttribute('aria-expanded') === 'false') {
    await toggle.click();
  }
  await page.locator(`header a[href="${path}"]`).first().click();
}

export async function mailStatus(request) {
  const response = await http(request, 'GET', `${mailPath}?limit=100`);
  return response.status();
}

export async function mailLink(request, address, subject, matchPath, since) {
  const body = await ownMail(request, address, subject, since);
  for (const content of [body.Text, body.HTML]) {
    for (const candidate of (content || '').match(/https?:\/\/[^\s<>"']+/g) || []) {
      const url = URL.parse(candidate.replaceAll('&amp;', '&'));
      if (!url || !matchPath.test(url.pathname)) continue;
      if (url.origin === origin || ['localhost', '127.0.0.1'].includes(url.hostname)) {
        return new URL(url.pathname + url.search, origin);
      }
    }
  }
  throw new Error('The newest matching delivery has no staging link for this action.');
}

export async function ownMail(request, address, subject, since) {
  const deadline = Date.now() + 60_000;
  while (Date.now() < deadline) {
    const response = await http(request, 'GET', `${mailPath}?limit=100`);
    if ([404, 405].includes(response.status())) throw new Error(`Mailpit returned HTTP ${response.status()}`);
    expect(response.status(), 'Mailpit list').toBe(200);
    const { messages } = await response.json();
    const newest = (messages ?? []).slice(0, 100)
      .filter(message => message.To.some(to => to.Address.toLowerCase() === address.toLowerCase())
        && subject.test(message.Subject) && Date.parse(message.Created) >= since)
      .sort((left, right) => Date.parse(right.Created) - Date.parse(left.Created))[0];
    if (newest) {
      const response = await http(request, 'GET', `/__mailpit/api/v1/message/${encodeURIComponent(newest.ID)}`);
      expect(response.status(), 'Mailpit message').toBe(200);
      return response.json();
    }
    await wait(2_000);
  }
  throw new Error('No matching delivery was available within 60 seconds.');
}

// Attach only in-memory browser pixels from a public, credential-free screen.
// Request headers are not collected and no traces or network recordings are enabled.
export async function publicScreenshot(page, info, name) {
  const path = info.outputPath(`${name}.png`);
  await page.screenshot({ path, mask: [page.locator('input, textarea')], timeout: 5_000 });
}

export async function requireMail(request) {
  const status = await mailStatus(request);
  test.skip([404, 405].includes(status), `Staging Mailpit returned HTTP ${status}`);
  expect(status, 'Mailpit list').toBe(200);
}

export async function signIn(page, locale, account) {
  await page.goto(`/${locale}/login`, { waitUntil: 'domcontentloaded' });
  return submitWithBudget(page.locator('main form'), account);
}

export async function verifiedPlayer(page, locale) {
  const account = player();
  await page.goto(`/${locale}/login`, { waitUntil: 'domcontentloaded' });
  await page.locator(`main a[href="/${locale}/register"]`).click();
  const since = Date.now() - 1_000;
  const registration = await submitWithBudget(page.locator('main form'), {
    ...account, password_confirmation: account.password, terms: true, privacy: true,
  });
  test.skip(registration.status() === 429 || registrationChallenge(page)?.status === 429,
    'Shared registration budget remained exhausted after two waits.');
  expect(registration.status(), 'fresh player registration').toBe(303);
  await expect(page).toHaveURL(`/${locale}/register/check-inbox`, { timeout: 90_000 });
  const link = await mailLink(page.request, account.email,
    /^(Potwierdź adres e-mail|Verify your email address)$/,
    /^\/(en|pl)\/verify-email$/, since);
  await page.goto(link.href, { waitUntil: 'domcontentloaded' });
  await submitWithBudget(page.locator('main form'), {});
  await expect(page.locator('main [role=status]')).toHaveText(locale === 'en'
    ? 'Your e-mail has been verified.' : 'Twój adres e-mail został zweryfikowany.');
  await page.reload({ waitUntil: 'domcontentloaded' });
  await expect(page.locator('main [role=status]')).toHaveText(locale === 'en'
    ? 'This link is invalid or has expired. Request a new verification e-mail.'
    : 'Ten link jest nieprawidłowy lub wygasł. Poproś o nową wiadomość weryfikacyjną.');
  await signIn(page, locale, account);
  await expect(page.locator('header [data-account-link]')).toBeAttached();
  return account;
}

export async function accountNav(page, path) {
  await nav(page, '/account');
  await page.locator(`#podglad-konta a[href="${path}"]`).click();
}

export function totp(secret) {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  const encoded = (URL.parse(secret)?.searchParams.get('secret') ?? secret)
    .toUpperCase().replace(/\s|=/g, '');
  let bits = 0;
  let buffer = 0;
  const key = [];
  for (const character of encoded) {
    const number = alphabet.indexOf(character);
    if (number < 0) throw new Error('The authenticator secret was malformed.');
    buffer = (buffer << 5) | number;
    bits += 5;
    if (bits >= 8) {
      bits -= 8;
      key.push((buffer >>> bits) & 255);
    }
  }
  const counter = Buffer.alloc(8);
  counter.writeBigUInt64BE(BigInt(Math.floor(Date.now() / 30_000)));
  const digest = createHmac('sha1', Buffer.from(key)).update(counter).digest();
  const offset = digest.at(-1) & 15;
  return String((digest.readUInt32BE(offset) & 0x7fffffff) % 1_000_000).padStart(6, '0');
}
