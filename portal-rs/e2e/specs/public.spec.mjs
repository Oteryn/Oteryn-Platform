import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test('all public header and footer links: shell, links, keyboard, accessibility and mobile', async ({ page: homePage }, info) => {
  const page = homePage;
  const lang = info.project.metadata.locale;
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
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
    page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
    try {
    await test.step(href, async () => {
      let response;
      try { response = await page.goto(href); } catch (error) { failures.push(`${href}: ${error.message}`); return; }
      if (response.status() !== 200) failures.push(`${href}: HTTP ${response.status()}`);
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
        const linked = await page.request.get(target);
        if (linked.status() >= 400) failures.push(`${href} -> ${target}: HTTP ${linked.status()}`);
      }
    });
    } finally { await page.close(); }
  }
  await info.attach('public-findings', { body: JSON.stringify({ links, failures, errors }, null, 2), contentType: 'application/json' });
  expect(failures).toEqual([]);
  expect(errors).toEqual([]);
});
