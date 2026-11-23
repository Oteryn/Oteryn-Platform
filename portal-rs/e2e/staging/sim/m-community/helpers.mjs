import { randomBytes, randomUUID } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { test as base, expect } from '@playwright/test';

export const origin = 'https://staging.oteryn.com';
export const mailPath = '/__mailpit/api/v1/messages';

function accessHeaders() {
  const id = process.env.STAGING_CF_ACCESS_CLIENT_ID;
  const secret = process.env.STAGING_CF_ACCESS_CLIENT_SECRET;
  if (!id || !secret) throw new Error('Staging Access credentials are missing');
  return { 'CF-Access-Client-Id': id, 'CF-Access-Client-Secret': secret };
}

export const test = base.extend({
  context: async ({ context }, use) => {
    const access = accessHeaders();
    await context.route('**/*', async route => {
      const request = route.request();
      if (new URL(request.url()).origin !== origin) {
        await route.continue();
        return;
      }
      try {
        // Fulfill one response at a time so a redirect cannot inherit Access headers.
        const response = await route.fetch({
          headers: { ...await request.allHeaders(), ...access },
          maxRedirects: 0,
          timeout: 15_000,
        });
        try {
          await route.fulfill({ response });
        } finally {
          await response.dispose();
        }
      } catch {
        // Playwright network errors can contain header values in their call logs.
        await route.abort().catch(() => {});
      }
    });
    await use(context);
  },
});
export { expect };

export async function send(client, path, { method = 'GET', form, headers = {} } = {}) {
  const url = new URL(path, origin);
  if (url.origin !== origin) throw new Error('API request must target staging');
  try {
    return await client.fetch(url.href, {
      method,
      headers: { ...headers, ...accessHeaders() },
      ...(form === undefined ? {} : { form }),
      maxRedirects: 0,
      timeout: 12_000,
    });
  } catch {
    throw new Error(`Staging request failed for ${method} ${url.pathname.split('/').slice(0, 2).join('/')}`);
  }
}

async function waitForReady(request) {
  await expect.poll(async () => {
    try {
      const response = await send(request, '/readyz');
      const status = response.status();
      await response.dispose();
      return status;
    } catch {
      return 'unreachable';
    }
  }, { timeout: 300_000, intervals: [2000, 5000, 10_000] }).toBe(200);
}

test.beforeEach(async ({ request }) => {
  await waitForReady(request);
});

export async function userRequest(client, path, options) {
  for (let attempt = 0; attempt < 3; attempt++) {
    const response = await send(client, path, options);
    if (response.status() !== 429) return response;
    await response.dispose();
    if (attempt < 2) await delay(65_000);
  }
  test.skip(true, 'Shared staging egress remained rate-limited after two waits');
}

export async function status(client, method, path, expected, options) {
  const response = await send(client, path, { ...options, method });
  expect.soft(response.status(), `${method} ${path}`).toBe(expected);
  return response;
}

export async function fill(form, fields) {
  for (const [name, value] of Object.entries(fields)) {
    const input = form.locator(`[name="${name}"]`);
    if (typeof value === 'boolean') await input.setChecked(value);
    else await input.fill(value);
  }
}

export async function submit(page, path, fields, selector = 'main form') {
  const registration = path.endsWith('/register');
  for (let attempt = 0; attempt < 3; attempt++) {
    const form = page.locator(selector);
    await fill(form, fields);
    let challengeStatus;
    const watchChallenge = response => {
      if (new URL(response.url()).pathname === '/register/abuse/challenge') {
        challengeStatus = response.status();
      }
    };
    if (registration) page.on('response', watchChallenge);
    let response;
    try {
      [response] = await Promise.all([
        page.waitForResponse(r => new URL(r.url()).pathname === path && r.request().method() === 'POST',
          { timeout: 90_000 }),
        form.locator('button[type="submit"]').click(),
      ]);
    } finally {
      if (registration) page.off('response', watchChallenge);
    }
    if (response.status() !== 429 && challengeStatus !== 429) {
      if (registration) {
        test.info().annotations.push({
          type: 'registration challenge status',
          description: String(challengeStatus ?? 'no response'),
        });
      }
      return response;
    }
    if (attempt === 2) break;
    // The registration script may submit an empty proof after a challenge
    // budget refusal. Give the shared one-minute budget a whole window.
    await delay(65_000);
    await waitForReady(page.request);
    let reopened = await page.goto(path);
    if ([502, 504].includes(reopened.status())) {
      await delay(10_000);
      await waitForReady(page.request);
      reopened = await page.goto(path);
    }
    if (reopened.status() === 429) {
      await delay(65_000);
      reopened = await page.goto(path);
    }
    if (reopened.status() === 429) {
      test.skip(true, 'Shared staging egress limited reopening the form');
    }
    expect(reopened.status(), 'Reopen the form after a rate limit').toBe(200);
  }
  test.skip(true, 'Shared staging egress remained rate-limited after two waits');
}

export function newPlayer() {
  return {
    email: `sim-m-community-${randomUUID()}-1@example.test`,
    password: `Sim!-${randomBytes(24).toString('base64url')}`,
  };
}

export async function mailAvailability(client) {
  const response = await send(client, mailPath);
  const code = response.status();
  await response.dispose();
  return code;
}

export async function verifyPlayer(page, account, since) {
  const deadline = Date.now() + 60_000;
  while (Date.now() < deadline) {
    const response = await send(page.request, `${mailPath}?limit=100`);
    if ([404, 405].includes(response.status())) {
      await response.dispose();
      test.skip(true, 'Staging Mailpit unavailable for player verification');
    }
    expect(response.status(), 'Mailpit list status').toBe(200);
    const { messages = [] } = await response.json();
    await response.dispose();
    const latest = messages.slice(0, 100)
      .filter(message => (message.To ?? []).some(to => to.Address.toLowerCase() === account.email)
        && ['Verify your email address', 'Potwierdź adres e-mail'].includes(message.Subject)
        && Date.parse(message.Created) >= since)
      .sort((a, b) => Date.parse(b.Created) - Date.parse(a.Created))[0];
    if (latest) {
      const detail = await send(page.request, `/__mailpit/api/v1/message/${encodeURIComponent(latest.ID)}`);
      expect(detail.status(), 'Mailpit message status').toBe(200);
      const { Text = '', HTML = '' } = await detail.json();
      await detail.dispose();
      const candidate = `${Text}\n${HTML}`.match(/https:\/\/[^\s"'<>]+\/(?:en|pl)\/verify-email\?token=[a-f0-9]+/i)?.[0];
      if (candidate) {
        const link = new URL(candidate);
        if (![new URL(origin).hostname, 'localhost', '127.0.0.1'].includes(link.hostname)) {
          throw new Error('Unexpected verification link host');
        }
        // Prevent assertion call logs from including the URL of the email link.
        try {
          const opened = await page.goto(`${link.pathname}${link.search}`);
          if (opened.status() !== 200) throw new Error('Verification page refused');
          const confirm = page.locator('main form button[type=submit]');
          if (await confirm.count()) await confirm.click();
        } catch {
          throw new Error('Player verification could not be completed');
        }
        return;
      }
    }
    await delay(2000);
  }
  test.skip(true, 'No player verification message received within 60 seconds');
}
