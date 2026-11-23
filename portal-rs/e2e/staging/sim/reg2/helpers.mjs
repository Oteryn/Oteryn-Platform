import { randomBytes } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { test as base, expect } from '@playwright/test';

export const origin = 'https://staging.oteryn.com';
const host = new URL(origin).host;

function accessHeaders() {
  const id = process.env.STAGING_CF_ACCESS_CLIENT_ID;
  const secret = process.env.STAGING_CF_ACCESS_CLIENT_SECRET;
  if (!id || !secret) throw new Error('Staging Cloudflare Access credentials are required');
  return { 'CF-Access-Client-Id': id, 'CF-Access-Client-Secret': secret };
}

function stagingURL(path) {
  const url = new URL(path, origin);
  if (url.origin !== origin) throw new Error('Expected a staging URL');
  return url;
}

export async function enableAccess(context) {
  await context.route('**/*', async route => {
    const request = route.request();
    if (new URL(request.url()).host !== host) return route.continue();
    try {
      stagingURL(request.url());
      // Keep Access headers on the server-side request only. Chromium manages
      // the same-origin navigation and cookies after the response is fulfilled.
      const response = await route.fetch({
        headers: { ...request.headers(), ...accessHeaders() },
        maxRedirects: 0,
        timeout: 20_000,
      });
      try {
        const location = response.headers().location;
        if (location && response.status() >= 300 && response.status() < 400) {
          stagingURL(new URL(location, request.url()));
        }
        await route.fulfill({ response });
      } finally {
        await response.dispose();
      }
    } catch {
      // Playwright network exceptions can contain request headers and URLs.
      await route.abort().catch(() => {});
    }
  });
}

async function get(request, path) {
  const url = stagingURL(path);
  try {
    return await request.get(url.href, {
      headers: { 'User-Agent': 'Mozilla/5.0', ...accessHeaders() },
      maxRedirects: 0,
      timeout: 12_000,
    });
  } catch {
    throw new Error('Staging HTTP request did not complete');
  }
}

export const test = base.extend({
  context: async ({ context }, use) => {
    await enableAccess(context);
    await use(context);
  },
});

test.beforeEach(async ({ request }, info) => {
  const deadline = Date.now() + 300_000;
  let status;
  do {
    const response = await get(request, '/readyz').catch(() => null);
    status = response?.status();
    await response?.dispose();
    if (status === 200) return;
    await delay(5_000);
  } while (Date.now() < deadline);
  info.annotations.push({ type: 'staging blocker', description: 'Readiness did not return HTTP 200 in five minutes.' });
  throw new Error(`Staging /readyz expected HTTP 200, last result: ${status ?? 'no response'}`);
});

test.afterEach(async ({}, info) => {
  if (info.errors.length) {
    // Omit the automatic DOM snapshot. It can contain one-time mail URLs.
    await info.attach('error-context', { body: 'DOM snapshot disabled for identity flows.', contentType: 'text/plain' });
  }
});

export function password() {
  return `Sim!${randomBytes(18).toString('hex')}`;
}

export function player() {
  return { email: `sim-reg2-${randomBytes(8).toString('hex')}-1@example.test`, password: password() };
}

export async function mail(request, address, subject) {
  const deadline = Date.now() + 60_000;
  do {
    const response = await get(request, '/__mailpit/api/v1/messages?limit=100');
    const status = response.status();
    if (status !== 200) {
      await response.dispose();
      throw new Error(`Mailpit list returned HTTP ${status}`);
    }
    const { messages } = await response.json().catch(() => {
      throw new Error('Mailpit returned invalid list JSON');
    });
    await response.dispose();
    const newest = messages.slice(0, 100)
      .filter(message => message.Subject === subject && message.To.some(to => to.Address === address))
      .sort((a, b) => Date.parse(b.Created) - Date.parse(a.Created))[0];
    if (newest) {
      const detail = await get(request, `/__mailpit/api/v1/message/${encodeURIComponent(newest.ID)}`);
      try {
        if (detail.status() !== 200) throw new Error(`Mailpit detail returned HTTP ${detail.status()}`);
        return await detail.json().catch(() => { throw new Error('Mailpit returned invalid message JSON'); });
      } finally {
        await detail.dispose();
      }
    }
    await delay(2_000);
  } while (Date.now() < deadline);
  throw new Error('No matching Mailpit message for the new player within 60 seconds');
}

export function mailLink(message, route) {
  for (const candidate of `${message.Text ?? ''}\n${message.HTML ?? ''}`.matchAll(/https?:\/\/[^\s<>"']+/g)) {
    const url = new URL(candidate[0].replaceAll('&amp;', '&'));
    if (!route.test(url.pathname)) continue;
    if (['localhost', '127.0.0.1'].includes(url.hostname)) {
      return new URL(url.pathname + url.search, origin);
    }
    return stagingURL(url);
  }
  throw new Error('The expected portal link was absent from the mail');
}

export async function open(page, target) {
  try {
    return await page.goto(stagingURL(target).href, { waitUntil: 'domcontentloaded' });
  } catch {
    throw new Error('Staging page navigation did not complete');
  }
}

export function mainForm(page, route) {
  return page.locator(`main form[action*="${route}"]`).first();
}

export async function submit(page, form, fields = {}, { nativeValidation = true, impatient = false } = {}) {
  for (const [name, value] of Object.entries(fields)) {
    const input = form.locator(`[name="${name}"]`);
    if (typeof value === 'boolean') await input.setChecked(value);
    else await input.fill(value);
  }
  if (!nativeValidation) await form.evaluate(element => { element.noValidate = true; });
  const pending = page.waitForResponse(response => response.request().isNavigationRequest()
    && response.request().method() === 'POST' && new URL(response.url()).origin === origin, { timeout: 90_000 });
  try {
    const button = form.locator('button[type=submit], button:not([type])').first();
    const [response] = await Promise.all([pending, button.click({ clickCount: impatient ? 2 : 1 })]);
    await page.waitForLoadState('domcontentloaded');
    return response;
  } catch {
    throw new Error('Submitting the portal form did not produce a completed navigation');
  }
}

export async function sharedLimit(response, info) {
  if (response.status() !== 429) return false;
  const value = Number(response.headers()['retry-after']);
  info.annotations.push({ type: 'shared staging rate limit', description: 'Waiting before repeating the user action' });
  await delay((value > 0 && value <= 120 ? value + 1 : 60) * 1_000);
  return true;
}

export async function withBudget(info, attempt, limitResponse = response => response) {
  for (let retry = 0; retry < 4; retry++) {
    const result = await attempt();
    if (!await sharedLimit(limitResponse(result), info)) return result;
  }
  throw new Error('Shared staging rate limit persisted across four attempts');
}

export async function localized(page, lang) {
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
}

export function at(page, path) {
  // Compare only the path. Mail links and form actions can contain credentials.
  expect(new URL(page.url()).pathname).toBe(path);
}

export { expect };
