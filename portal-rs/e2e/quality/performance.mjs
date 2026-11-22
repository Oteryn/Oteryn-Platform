import { expect } from '@playwright/test';

export const budgets = Object.freeze({
  lcpMs: 2500, cls: 0.1, javascriptBytes: 180_000, cssBytes: 120_000, requestBytes: 1_000_000,
});

export async function installVitals(page) {
  await page.addInitScript(() => {
    if (window !== window.top) return;
    let lcp = null;
    const shifts = [];
    let windowStart, previousShift, windowValue = 0, cls = 0;
    const consumeLcp = entries => {
      for (const entry of entries) lcp = entry.startTime;
    };
    const consumeShifts = entries => {
      for (const entry of entries) {
        if (entry.hadRecentInput) continue;
        shifts.push({
          ms: entry.startTime, score: entry.value,
          elements: entry.sources.map(({ node }) => node?.id || node?.nodeName),
        });
        if (windowStart === undefined || entry.startTime - previousShift > 1000 ||
            entry.startTime - windowStart > 5000) {
          windowStart = entry.startTime;
          windowValue = 0;
        }
        previousShift = entry.startTime;
        windowValue += entry.value;
        cls = Math.max(cls, windowValue);
      }
    };
    const lcpObserver = new PerformanceObserver(list => consumeLcp(list.getEntries()));
    const clsObserver = new PerformanceObserver(list => consumeShifts(list.getEntries()));
    lcpObserver.observe({ type: 'largest-contentful-paint', buffered: true });
    clsObserver.observe({ type: 'layout-shift', buffered: true });
    window.portalQualityVitals = () => {
      consumeLcp(lcpObserver.takeRecords());
      consumeShifts(clsObserver.takeRecords());
      return { lcp, cls, shifts };
    };
  });
}

export async function vitals(page) {
  await page.evaluate(() => document.fonts.ready.then(() => undefined));
  await expect.poll(() => page.evaluate(() => window.portalQualityVitals().lcp), {
    message: 'Chromium reports a painted LCP candidate',
  }).not.toBeNull();
  return page.evaluate(() => window.portalQualityVitals());
}

export function collectResponses(page) {
  const transfers = [];
  page.on('requestfinished', request => {
    transfers.push((async () => {
      const response = await request.response();
      const { responseBodySize: bytes } = await request.sizes();
      const pathname = new URL(request.url()).pathname;
      return {
        pathname, bytes, type: request.resourceType(), status: response.status(),
        cache: await response.headerValue('cache-control'),
      };
    })());
  });
  return async () => Promise.all(transfers);
}

export function assertBudgets(measurement, transfers, label) {
  expect.soft(measurement.lcp, `${label}: LCP in ms`).toBeLessThan(budgets.lcpMs);
  expect.soft(measurement.cls, `${label}: CLS`).toBeLessThan(budgets.cls);
  const totals = { script: 0, stylesheet: 0 };
  for (const resource of transfers) {
    const { pathname, bytes, type, status, cache } = resource;
    expect.soft(bytes, `${label}: transfer bytes for ${pathname}`).toBeLessThan(budgets.requestBytes);
    if (Object.hasOwn(totals, type)) totals[type] += bytes;
    if (status < 200 || status >= 300) continue;
    if (pathname.startsWith('/static/') || ['script', 'stylesheet', 'font', 'image'].includes(type)) {
      expect.soft(cache, `${label}: cache policy for ${pathname}`).toBeTruthy();
      if (pathname.startsWith('/static/')) {
        expect.soft(cache, `${label}: reusable static asset ${pathname}`).toMatch(/\bmax-age=[1-9]\d*/i);
      }
      if (/(?:^|[.-])[\da-f]{8,}(?=[.-]|$)/i.test(pathname.split('/').at(-1))) {
        expect.soft(cache, `${label}: content-hashed asset ${pathname}`).toMatch(/\bimmutable\b/i);
      }
    }
  }
  expect.soft(totals.script, `${label}: total JavaScript transfer`).toBeLessThan(budgets.javascriptBytes);
  expect.soft(totals.stylesheet, `${label}: total CSS transfer`).toBeLessThan(budgets.cssBytes);
  return totals;
}
