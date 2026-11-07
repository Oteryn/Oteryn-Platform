import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// These cold public projections have no accepted source in a fresh local database.
// Each remains a 503 HTML refusal; they are never counted as successful data reads.
const coldPath = value => /^\/(pl|en)\/(online|rankings\/experience|deaths|stats\/guilds|stats\/rankings\/(yesterday|week|month|record|oldest|anniversaries|name-changes|transfers|sold)|wiki(?:\/search)?|houses|atlas|atlas\/delivery)$/.test(new URL(value,'https://localhost').pathname) || (/^\/(pl|en)\/stats\/world$/.test(new URL(value,'https://localhost').pathname) && new URL(value,'https://localhost').search.length>0);
function consoleError(message, errors, diagnostics) {
  if(message.type()!=='error')return;
  if(coldPath(message.location().url) && /^Failed to load resource: the server responded with a status of 503 \(Service Unavailable\)$/.test(message.text())) {diagnostics.push(new URL(message.location().url).pathname);return;}
  errors.push(message.text());
}

test('all public header and footer links: shell, links, keyboard, accessibility and mobile', async ({ page: homePage }, info) => {
  test.setTimeout(240_000); // Every HTML refusal receives the full accessibility and link sweep.
  const page = homePage;
  const lang = info.project.metadata.locale;
  const errors = [];
  const diagnostics = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => consoleError(message, errors, diagnostics));
  const home = await page.goto(`/${lang}/`);
  expect(home.status()).toBe(200);
  const links = await page.locator('header a[href], footer a[href]').evaluateAll(nodes => [...new Set(nodes.map(n => n.getAttribute('href')))]);
  const failures = [];
  for (const href of links) {
    if (!href.startsWith('/') || href.startsWith('//')) continue;
    // Authentication forms are public; account navigation is tested as its owner.
    if (/\/(account|password\/change)(\/|$)/.test(href) || href === '/account') continue;
    const page = await homePage.context().newPage();
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => consoleError(message, errors, diagnostics));
    try {
    await test.step(href, async () => {
      let response;
      try { response = await page.goto(href); } catch (error) { failures.push(`${href}: ${error.message}`); return; }
      const expectedStatus=coldPath(href)?503:200;
      if (response.status() !== expectedStatus) failures.push(`${href}: HTTP ${response.status()}, expected ${expectedStatus}`);
      if(expectedStatus===503) {expect(response.headers()['cache-control']).toContain('no-store');}
      if (!response.headers()['content-type']?.startsWith('text/html')) { failures.push(`${href}: non-HTML response cannot qualify as a public page`); return; }
      const targetLanguage = /^\/(?:language\/)?(pl|en)(?:\/|$)/.exec(href)?.[1] || lang;
      if (await page.locator('html').getAttribute('lang') !== targetLanguage) failures.push(`${href}: wrong page language`);
      if (await page.locator('body.op-shell > header.op-shell__header').count() !== 1) failures.push(`${href}: shared header missing`);
      if (await page.locator('body.op-shell > footer.op-shell__footer').count() !== 1) failures.push(`${href}: shared footer missing`);
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
      if (overflow) failures.push(`${href}: horizontal overflow`);
      await page.keyboard.press('Tab');
      const focus = await page.evaluate(() => {const s=getComputedStyle(document.activeElement);return document.activeElement.matches(':focus-visible') && (s.outlineStyle!=='none' || s.boxShadow!=='none');});
      if (!focus) failures.push(`${href}: focus not visible`);
      const axe = await new AxeBuilder({ page }).analyze();
      failures.push(...axe.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => `${href}: axe ${v.id}: ${v.nodes.map(n=>n.target).join(';')}`));
      const localLinks = await page.locator('main a[href]').evaluateAll(nodes => [...new Set(nodes.map(n=>n.getAttribute('href')))].filter(h=>h.startsWith('/')&&!h.startsWith('//')));
      for (const target of localLinks) {
        const linked = await page.request.get(target,{headers:{accept:'text/html'}});
        const expected=coldPath(target)?503:200;
        if (linked.status() !== expected) failures.push(`${href} -> ${target}: HTTP ${linked.status()}, expected ${expected}`);
        if(expected===503 && !linked.headers()['content-type']?.startsWith('text/html')) failures.push(`${href} -> ${target}: non-HTML refusal`);
        if(expected===503) {expect(linked.headers()['cache-control']).toContain('no-store');expect(await linked.text()).toContain('op-shell__header');}
      }
    });
    } finally { await page.close(); }
  }
  await info.attach('public-findings', { body: JSON.stringify({ links, failures, errors, expectedRefusalDiagnostics:diagnostics }, null, 2), contentType: 'application/json' });
  expect(failures).toEqual([]);
  expect(errors).toEqual([]);
});
