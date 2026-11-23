import { setTimeout as delay } from 'node:timers/promises';
import { expect } from './fixtures.mjs';
import { mailList, requestTo, stagingOrigin, stagingURL } from './transport.mjs';

export async function mailLink(request, email, subject, lang, purpose) {
  const deadline = Date.now() + 60_000;
  const inspected = new Set();
  do {
    const list = await getJSON(request, mailList, deadline);
    expect(Array.isArray(list.messages), 'Mailpit response contains a message list').toBe(true);
    for (const message of list.messages) {
      if (message.Subject !== subject || !message.To?.some(({ Address }) => Address === email) ||
          inspected.has(message.ID)) continue;
      inspected.add(message.ID);
      const detail = await getJSON(request, `/__mailpit/api/v1/message/${encodeURIComponent(message.ID)}`, deadline);
      for (const candidate of `${detail.Text ?? ''}\n${detail.HTML ?? ''}`.matchAll(/https?:\/\/[^\s<>"']+/g)) {
        let url;
        try {
          url = new URL(candidate[0].replaceAll('&amp;', '&'));
          if (['localhost', '127.0.0.1'].includes(url.hostname) && !url.username && !url.password) {
            url = new URL(url.pathname + url.search, stagingOrigin);
          }
          url = stagingURL(url.href);
        } catch {
          continue;
        }
        if (purpose === 'verify' && url.pathname === `/${lang}/verify-email` &&
            /^[a-f0-9]{64}$/i.test(url.searchParams.get('token') ?? '')) return url.href;
        if (purpose === 'reset' &&
            new RegExp(`^/${lang}/reset-password/[a-f0-9]{64}$`, 'i').test(url.pathname)) return url.href;
      }
    }
    await delay(Math.min(2_000, Math.max(0, deadline - Date.now())));
  } while (Date.now() < deadline);
  throw new Error(`No ${purpose} email with a staging link arrived for the test address within 60 seconds.`);
}

async function getJSON(request, path, deadline) {
  const response = await requestTo(request, 'GET', path, {
    timeout: Math.max(1, Math.min(12_000, deadline - Date.now())),
  });
  try {
    expect(response.status(), `GET ${path.replace(/\/message\/[^/]+$/, '/message/[id]')}`).toBe(200);
    return await response.json();
  } finally {
    await response.dispose();
  }
}
