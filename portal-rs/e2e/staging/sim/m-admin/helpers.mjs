import { setTimeout as delay } from 'node:timers/promises';
import { test as base, expect } from '@playwright/test';

export const origin = 'https://staging.oteryn.com';
const stagingHost = new URL(origin).host;

function accessHeaders() {
  const id = process.env.STAGING_CF_ACCESS_CLIENT_ID;
  const secret = process.env.STAGING_CF_ACCESS_CLIENT_SECRET;
  if (!id || !secret) throw new Error('Staging Cloudflare Access credentials are missing');
  return { 'CF-Access-Client-Id': id, 'CF-Access-Client-Secret': secret };
}

export const test = base.extend({
  context: async ({ context }, use) => {
    const headers = accessHeaders();
    await context.route('**/*', async (route) => {
      const request = route.request();
      const url = new URL(request.url());
      if (url.host !== stagingHost || url.protocol !== 'https:') return route.continue();
      // Forward exactly one hop per intercepted request.
      let response;
      try {
        response = await route.fetch({
          headers: { ...await request.allHeaders(), ...headers },
          maxRedirects: 0,
          timeout: 15_000,
        });
        await route.fulfill({ response });
      } catch {
        await route.abort();
      } finally {
        await response?.dispose();
      }
    });
    await use(context);
  },
  serviceWorkers: 'block',
});
export { expect };

export async function routeStatus(request, method, path, timeout = 12_000) {
  const url = new URL(path, origin);
  if (!path.startsWith('/') || url.origin !== origin) {
    throw new Error('Expected a same-origin staging path');
  }
  const headers = {
    ...accessHeaders(),
    'Accept-Language': test.info().project.metadata.acceptLanguage,
  };
  let response;
  try {
    response = await request.fetch(url.href, {
      method,
      headers,
      maxRedirects: 0,
      timeout,
      ...(method === 'GET' ? {} : { form: {} }),
    });
  } catch {
    // Playwright transport errors can contain request metadata.
    throw new Error(`Staging transport failed for ${method} ${path}`);
  }
  const status = response.status();
  await response.dispose();
  return status;
}

export async function waitUntilReady(request) {
  accessHeaders();
  const deadline = Date.now() + 300_000;
  let lastResult;
  while (Date.now() < deadline) {
    const timeout = Math.max(1, Math.min(12_000, deadline - Date.now()));
    lastResult = await routeStatus(request, 'GET', '/readyz', timeout)
      .catch(() => 'transport error');
    if (lastResult === 200) return;
    await delay(Math.min(3000, Math.max(0, deadline - Date.now())));
  }
  throw new Error(`GET /readyz did not reach 200 within five minutes (last result: ${lastResult})`);
}
