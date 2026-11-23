import { randomBytes } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { setTimeout as sleep } from 'node:timers/promises';
import { test as base, expect } from '@playwright/test';

export const origin = 'https://staging.oteryn.com';
const portal = new URL('../../../../', import.meta.url);
const accessHeaders = {
  'CF-Access-Client-Id': process.env.STAGING_CF_ACCESS_CLIENT_ID,
  'CF-Access-Client-Secret': process.env.STAGING_CF_ACCESS_CLIENT_SECRET,
};
const runId = randomBytes(6).toString('hex');
let registrationNumber = 0;

export const test = base.extend({
  context: async ({ context }, use) => {
    await routeStagingAccess(context);
    try {
      await use(context);
    } finally {
      await context.unrouteAll({ behavior: 'ignoreErrors' });
    }
  },
});

export { expect };

export async function routeStagingAccess(context) {
  await context.route('**/*', async route => {
    if (new URL(route.request().url()).origin !== origin) return route.continue();
    // Chromium's route.continue header overrides carry over to redirected
    // requests. Fetch one hop and let Chromium handle redirects without them.
    let response;
    try {
      response = await route.fetch({
        headers: { ...await route.request().allHeaders(), ...accessHeaders },
        maxRedirects: 0,
        timeout: 15_000,
      });
    } catch {
      return route.abort();
    }
    try {
      await route.fulfill({ response });
    } finally {
      await response.dispose();
    }
  });
}

export async function get(request, path, timeout = 8_000) {
  const target = new URL(path, origin);
  if (target.origin !== origin) throw new Error('Request outside staging');
  try {
    return await request.get(target.href, { headers: accessHeaders, maxRedirects: 0, timeout });
  } catch {
    // APIRequestContext's raw error can include request headers.
    throw new Error(`GET ${target.pathname} failed before receiving an HTTP response`);
  }
}

export async function postForm(request, path, form) {
  const target = new URL(path, origin);
  if (target.origin !== origin) throw new Error('Request outside staging');
  try {
    return await request.post(target.href, {
      form, headers: { ...accessHeaders, origin }, maxRedirects: 0, timeout: 8_000,
    });
  } catch {
    throw new Error(`POST ${target.pathname} failed before receiving an HTTP response`);
  }
}

test.beforeEach(async ({ request }) => {
  await expect.poll(async () => {
    try {
      return (await get(request, '/readyz')).status();
    } catch {
      return 'unreachable';
    }
  }, { timeout: 300_000, intervals: [2_000, 5_000, 10_000], message: 'Staging readiness' }).toBe(200);
});

export function messages(file, lang) {
  const ftl = readFileSync(new URL(file.replace('{lang}', lang), portal), 'utf8');
  return Object.fromEntries([...ftl.matchAll(/^([\w-]+) = ([^{}\r\n]+)$/gm)]
    .map(([, key, value]) => [key, value.trim()]));
}

export const ui = lang => messages('crates/ui/i18n/{lang}.ftl', lang);

export async function afterSharedQuota(response) {
  const seconds = Number(response.headers()['retry-after']);
  await sleep((seconds > 0 && seconds <= 60 ? seconds + 1 : 61) * 1000);
}

export function newcomer() {
  return {
    email: `sim-newcomer-${runId}-${++registrationNumber}@example.test`,
    password: `Oteryn!${randomBytes(15).toString('hex')}`,
    // Character names permit ASCII letters and spaces.
    name: `Sim ${randomBytes(6).toString('hex').replaceAll(/[0-9]/g, digit => 'ghijklmnop'[digit])}`,
  };
}

export async function navigate(page, path) {
  const url = new URL(path, origin);
  if (url.origin !== origin) throw new Error('Navigation outside staging');
  try {
    const visit = () => page.goto(url.href, { waitUntil: 'domcontentloaded' });
    let response = await visit();
    for (let waits = 0; response.status() === 429 && waits < 2; waits++) {
      await afterSharedQuota(response);
      response = await visit();
    }
    return response;
  } catch {
    throw new Error(`Navigation to ${url.pathname} failed`);
  }
}

export async function submit(form, { doubleClick = false } = {}) {
  const page = form.page();
  const target = await form.evaluate(node => new URL(node.action).pathname);
  const button = form.locator('button[type=submit], button:not([type])').first();
  const [response] = await Promise.all([
    page.waitForResponse(reply => reply.request().isNavigationRequest()
      && reply.request().method() === 'POST' && new URL(reply.url()).pathname === target,
    { timeout: 60_000 }),
    doubleClick ? button.dblclick() : button.click(),
  ]);
  return response;
}

export async function openRegistration(page, lang) {
  const response = await navigate(page, `/${lang}/register`);
  expect(response.status(), 'Registration form').toBe(200);
  await expect(page.locator('main h1')).toHaveText(ui(lang)['register-title']);
  return page.locator('main form');
}

export async function fillRegistration(form, player, {
  email = player.email, password = player.password,
  confirmation = password, terms = true, privacy = true,
} = {}) {
  await form.locator('[name=email]').fill(email);
  await form.locator('[name=password]').fill(password);
  await form.locator('[name=password_confirmation]').fill(confirmation);
  await form.locator('[name=terms]').setChecked(terms);
  await form.locator('[name=privacy]').setChecked(privacy);
}

export async function register(page, lang, player, email = player.email) {
  const response = await submitRegistration(page, lang, player, { email });
  expect(response.status(), 'Valid registration POST').toBe(303);
  await expect(page).toHaveURL(`${origin}/${lang}/register/check-inbox`);
  await expect(page.locator('main [role=status]')).toHaveText(ui(lang)['verify-inbox']);
}

export async function submitRegistration(page, lang, player, fields, openedForm) {
  for (let waits = 0; ; waits++) {
    const form = waits === 0 && openedForm ? openedForm : await openRegistration(page, lang);
    await fillRegistration(form, player, fields);
    let limitedChallenge;
    const observe = response => {
      if (new URL(response.url()).pathname === '/register/abuse/challenge' && response.status() === 429) {
        limitedChallenge = response;
      }
    };
    page.on('response', observe);
    let response;
    try {
      response = await submit(form, { doubleClick: Boolean(openedForm && waits === 0) });
    } finally {
      page.off('response', observe);
    }
    const underSharedQuota = response.status() === 429
      || (limitedChallenge && response.status() === 422 && !openedForm);
    if (!underSharedQuota) return response;
    test.skip(waits === 2, 'Shared staging registration quota still exhausted after two waits');
    await afterSharedQuota(limitedChallenge ?? response);
  }
}

export async function verificationLink(request, player) {
  let status;
  let link;
  const deadline = Date.now() + 60_000;
  do {
    const response = await get(request, '/__mailpit/api/v1/messages?limit=100',
      Math.min(8_000, Math.max(1, deadline - Date.now())));
    status = response.status();
    if (status === 404 || status === 405) return { status };
    expect(status, 'Mailpit list status').toBe(200);
    const { messages: inbox } = await response.json();
    const newest = (inbox ?? []).slice(0, 100)
      .filter(message => message.To?.some(to => to.Address.toLowerCase() === player.email))
      .sort((left, right) => Date.parse(right.Created) - Date.parse(left.Created))[0];
    if (newest && deadline > Date.now()) {
      const message = await get(request, `/__mailpit/api/v1/message/${encodeURIComponent(newest.ID)}`,
        Math.min(8_000, deadline - Date.now()));
      expect(message.status(), 'Mailpit message status').toBe(200);
      const { Text, HTML } = await message.json();
      const matches = `${Text ?? ''}\n${HTML ?? ''}`.matchAll(/https?:\/\/[^\s"<>]+\/\b(?:pl|en)\/verify-email\?token=[a-f0-9]+/g);
      for (const [candidate] of matches) {
        const url = new URL(candidate);
        if (url.origin === origin || ['localhost', '127.0.0.1'].includes(url.hostname)) {
          link = new URL(url.pathname + url.search, origin).href;
          break;
        }
      }
      if (link) return { status, link };
    }
    if (deadline > Date.now()) await sleep(Math.min(2_000, deadline - Date.now()));
  } while (Date.now() < deadline);
  return { status };
}

export async function requireVerifiedPlayer(page, lang, player) {
  await register(page, lang, player);
  const { status, link } = await verificationLink(page.request, player);
  test.skip(status === 404 || status === 405, `Mailpit GET returned HTTP ${status}`);
  expect(Boolean(link), 'Verification email for the new player arrived within 60 seconds').toBe(true);
  expect((await navigate(page, link)).status(), 'Verification confirmation page').toBe(200);
  await expect(page.locator('main h1')).toHaveText(ui(lang)['verify-title']);
  const response = await submit(page.locator('main form'));
  expect(response.status(), 'Verification POST').toBe(200);
  await expect(page.locator('main [role=status]')).toHaveText(ui(lang)['verify-success']);
  return link;
}

export async function login(page, lang, player, path = `/${lang}/login`) {
  if (path) await navigate(page, path);
  for (let waits = 0; ; waits++) {
    const form = page.locator('main form[action$="/login"]');
    await form.locator('[name=email]').fill(player.email);
    await form.locator('[name=password]').fill(player.password);
    const response = await submit(form);
    if (response.status() !== 429 || waits === 2) return response;
    await afterSharedQuota(response);
    await navigate(page, path ?? `/${lang}/login`);
  }
}
