import { randomBytes } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { expect, test as base } from '@playwright/test';

export const staging = new URL('https://staging.oteryn.com');
export const proxy = process.env.HTTPS_PROXY ? { server: process.env.HTTPS_PROXY } : undefined;

function accessHeaders() {
  const id = process.env.STAGING_CF_ACCESS_CLIENT_ID;
  const secret = process.env.STAGING_CF_ACCESS_CLIENT_SECRET;
  if (!id || !secret) throw new Error('Staging Cloudflare Access credentials are missing.');
  return { 'CF-Access-Client-Id': id, 'CF-Access-Client-Secret': secret };
}

function stagingURL(path) {
  const url = new URL(path, staging);
  if (url.origin !== staging.origin || url.username || url.password) {
    throw new Error('Expected an HTTPS staging URL.');
  }
  return url;
}

export async function send(request, method, path, fields = {}) {
  const url = stagingURL(path);
  try {
    return await request.fetch(url.href, {
      ...fields,
      method,
      headers: { ...fields.headers, ...accessHeaders() },
      maxRedirects: 0,
      timeout: 12_000,
    });
  } catch {
    // Playwright transport errors can include request headers and sensitive URLs.
    throw new Error(`Staging ${method} request failed.`);
  }
}

export async function protect(context) {
  const headers = accessHeaders();
  await context.route('**/*', async route => {
    const request = route.request();
    if (new URL(request.url()).host !== staging.host) {
      await route.continue();
      return;
    }
    if (new URL(request.url()).origin !== staging.origin) {
      await route.abort();
      return;
    }
    try {
      // A header override in route.continue also applies to redirect requests.
      // Fulfil each staging request individually without following redirects.
      const response = await route.fetch({
        headers: { ...request.headers(), ...headers },
        maxRedirects: 0,
        timeout: 16_000,
      });
      try {
        await route.fulfill({ response });
      } finally {
        await response.dispose();
      }
    } catch {
      await route.abort().catch(() => {});
    }
  });
}

export const test = base.extend({
  context: async ({ context }, use) => {
    await protect(context);
    await use(context);
  },
  stagingReady: [async ({ request }, use) => {
    await expect.poll(async () => {
      try {
        const response = await send(request, 'GET', '/readyz');
        const status = response.status();
        await response.dispose();
        return status;
      } catch {
        return 0;
      }
    }, { timeout: 300_000, intervals: [3_000, 8_000, 15_000] }).toBe(200);
    await use();
  }, { auto: true, timeout: 315_000 }],
});

export async function status(request, method, path, expected, fields) {
  const response = await send(request, method, path, fields);
  expect.soft(response.status(), `${method} ${path}`).toBe(expected);
  return response;
}

export async function visit(page, path, expected, title) {
  const response = await page.goto(stagingURL(path).href, { waitUntil: 'domcontentloaded' });
  expect(response.status(), path).toBe(expected);
  if (title) await expect(page.locator('main h1')).toHaveText(title);
  return response;
}

export async function follow(page, href, title, status = 200) {
  stagingURL(href);
  const links = page.locator(`a[href="${href}"]:visible`);
  if (!await links.count()) {
    await page.locator('header').getByRole('button', { name: 'Menu', exact: true }).click();
  }
  const [response] = await Promise.all([
    page.waitForResponse(r => r.request().isNavigationRequest() && new URL(r.url()).pathname === href),
    links.first().click(),
  ]);
  expect(response.status(), href).toBe(status);
  await expect(page).toHaveURL(stagingURL(href).href);
  await expect(page.locator('main h1')).toHaveText(title);
}

export async function safeForm(form, values, button = form.locator('button[type=submit], button:not([type])').first()) {
  for (const [name, value] of Object.entries(values)) {
    const field = form.locator(`[name="${name}"]`);
    const type = await field.getAttribute('type');
    if (type === 'checkbox') await field.setChecked(value);
    else if (await field.evaluate(node => node.tagName) === 'SELECT') await field.selectOption(value);
    else await field.fill(value);
  }
  await button.click();
}

export function password() {
  return `Sim!-4${randomBytes(19).toString('hex')}`;
}

export const inboxPath = '/__mailpit/api/v1/messages?limit=100';

export async function mailLink(request, email, createdAfter, subject, pathPattern) {
  const deadline = Date.now() + 60_000;
  while (Date.now() < deadline) {
    const response = await send(request, 'GET', inboxPath);
    try {
      if (response.status() === 404 || response.status() === 405) return null;
      expect(response.status(), 'Mailpit inbox response').toBe(200);
      const { messages } = await response.json();
      const newest = messages.slice(0, 100)
        .filter(message => message.To.some(to => to.Address.toLowerCase() === email.toLowerCase())
          && subject.test(message.Subject) && Date.parse(message.Created) >= createdAfter)
        .sort((a, b) => Date.parse(b.Created) - Date.parse(a.Created))[0];
      if (newest && /^[a-zA-Z0-9_-]+$/.test(newest.ID)) {
        const detail = await send(request, 'GET', `/__mailpit/api/v1/message/${newest.ID}`);
        try {
          expect(detail.status(), 'Mailpit message response').toBe(200);
          const { Text, HTML } = await detail.json();
          for (const link of `${Text}\n${HTML}`.match(/https?:\/\/[^\s"'<>]+/g) ?? []) {
            let url;
            try { url = new URL(link.replaceAll('&amp;', '&')); } catch { continue; }
            if (pathPattern.test(url.pathname) &&
                (url.origin === staging.origin || ['localhost', '127.0.0.1'].includes(url.hostname))) {
              return stagingURL(url.pathname + url.search).href;
            }
          }
        } finally {
          await detail.dispose();
        }
      }
    } finally {
      await response.dispose();
    }
    await delay(3_000);
  }
  return null;
}
