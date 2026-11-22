import { readFileSync, writeFileSync } from 'node:fs';
import { setTimeout as delay } from 'node:timers/promises';
import { diagnosticURL, publicURL } from './crawl-url.mjs';

export function requestGate(origins, language, userAgent, budgetPath) {
  let tail = Promise.resolve();
  const cache = new Map();
  return (url, method = 'GET') => {
    if (!['GET', 'HEAD'].includes(method) || !publicURL(url, origins)) {
      throw new Error(`read-only gate rejected ${method} ${diagnosticURL(url)}`);
    }
    const key = `${method} ${url.href}`;
    if (cache.has(key)) return cache.get(key);
    const response = tail.then(async () => {
      const [count, lastStart] = JSON.parse(readFileSync(budgetPath, 'utf8'));
      if (count >= 2000) throw new Error('shared 2000-request budget exhausted');
      await delay(Math.max(0, lastStart + 350 - Date.now()));
      writeFileSync(budgetPath, JSON.stringify([count + 1, Date.now()]));
      const result = await fetch(url, {
        method,
        redirect: 'manual',
        signal: AbortSignal.timeout(15_000),
        headers: { 'user-agent': userAgent, 'accept-language': language },
      });
      const { headers, status } = result;
      return { headers, status, body: Buffer.from(await result.arrayBuffer()) };
    });
    tail = response.catch(() => {});
    cache.set(key, response);
    return response;
  };
}
