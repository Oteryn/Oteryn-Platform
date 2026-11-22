import { once } from 'node:events';
import { expect } from '@playwright/test';

export const budgets = Object.freeze({
  lcpMs: 2500, cls: 0.1, javascriptBytes: 180_000, cssBytes: 120_000, requestBytes: 1_000_000,
});

export async function installVitals(page) {
  await page.addInitScript(() => {
    if (window !== window.top) return;
    let lcp = null;
    let lcpAsset;
    let candidate;
    const pathname = url => {
      const parsed = new URL(url, location.href);
      return parsed.origin === location.origin ? parsed.pathname : undefined;
    };
    const shifts = [];
    let windowStart, previousShift, windowValue = 0, cls = 0;
    const consumeLcp = entries => {
      for (const entry of entries) {
        lcp = entry.startTime;
        lcpAsset = entry.url ? pathname(entry.url) : undefined;
        const element = entry.element;
        candidate = {
          element: element && {
            tag: element.localName, id: element.id || undefined,
            classes: [...element.classList],
          },
          render: entry.renderTime,
          load: entry.loadTime,
          area: entry.size,
        };
      }
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
      const navigation = performance.getEntriesByType('navigation')[0];
      const resources = performance.getEntriesByType('resource')
        .filter(resource => pathname(resource.name) &&
          resource.startTime <= lcp)
        .map(resource => ({
          path: pathname(resource.name), type: resource.initiatorType,
          start: resource.startTime, request: resource.requestStart,
          firstByte: resource.responseStart, end: resource.responseEnd,
          blocking: resource.renderBlockingStatus,
        }));
      return { lcp, lcpAsset, cls, shifts, candidate, timing: {
        firstByte: navigation.responseStart, responseEnd: navigation.responseEnd,
        domInteractive: navigation.domInteractive, resources,
      } };
    };
  });
}

export async function vitals(page) {
  await page.evaluate(async () => {
    await document.fonts.ready;
    await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  });
  await expect.poll(() => page.evaluate(() => window.portalQualityVitals().lcp), {
    message: 'Chromium reports a painted LCP candidate',
  }).not.toBeNull();
  return page.evaluate(() => window.portalQualityVitals());
}

// Keep only timing and candidate events. Raw Chromium traces can contain URLs
// with tokens and authenticated page data.
export async function startPaintTrace(page) {
  if (process.env.PORTAL_QUALITY_PAINT_TRACE !== '1') return;
  const session = await page.context().newCDPSession(page);
  const candidates = [];
  const tasks = [];
  const renderEvents = [];
  const phases = new Set([
    'UpdateLayoutTree', 'Layout', 'PrePaint', 'Paint', 'Layerize', 'Commit', 'RasterTask',
    'Decode Image', 'ImageDecodeTask', 'EvaluateScript', 'FunctionCall',
  ]);
  session.on('Tracing.dataCollected', ({ value }) => {
    for (const event of value) {
      if (event.name === 'largestContentfulPaint::Candidate') {
        candidates.push({
          ts: event.ts, pid: event.pid, tid: event.tid,
          kind: event.args?.data?.type, node: event.args?.data?.nodeId,
          size: event.args?.data?.size, mainFrame: event.args?.data?.isMainFrame,
        });
      }
      if (event.name === 'RunTask' && event.dur >= 50_000) {
        tasks.push({ ts: event.ts, pid: event.pid, tid: event.tid, durationMs: event.dur / 1000,
          cpuMs: event.tdur == null ? undefined : event.tdur / 1000 });
      }
      if (phases.has(event.name) && event.dur >= 5_000) {
        renderEvents.push({ name: event.name, ts: event.ts, pid: event.pid, tid: event.tid,
          durationMs: event.dur / 1000, cpuMs: event.tdur == null ? undefined : event.tdur / 1000 });
      }
    }
  });
  await session.send('Tracing.start', {
    categories: 'loading,devtools.timeline,disabled-by-default-devtools.timeline',
  });
  return async () => {
    const complete = once(session, 'Tracing.tracingComplete');
    await session.send('Tracing.end');
    await complete;
    await session.detach();
    const main = candidates.filter(candidate => candidate.mainFrame !== false);
    const last = main.at(-1);
    return {
      candidates: main,
      longTasks: last ? tasks.filter(task =>
        task.pid === last.pid && task.tid === last.tid && task.ts < last.ts) : [],
      renderEvents: last ? renderEvents.filter(event =>
        event.pid === last.pid && event.ts < last.ts) : [],
    };
  };
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
