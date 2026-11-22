import { test, expect } from '../fixtures.mjs';
import {isHome} from '../shell.mjs';
import AxeBuilder from '@axe-core/playwright';

// These cold public projections have no accepted source in a fresh local database.
// Each remains a 503 HTML refusal; they are never counted as successful data reads.
const coldPath = value => /^\/(pl|en)\/(online|rankings\/experience|deaths|guilds|stats\/guilds|stats\/rankings\/(yesterday|week|month|record|oldest|anniversaries|name-changes|transfers|sold)|wiki(?:\/(?:search|creatures))?|houses|atlas|atlas\/delivery)$/.test(new URL(value,'https://localhost').pathname) || (/^\/(pl|en)\/stats\/world$/.test(new URL(value,'https://localhost').pathname) && new URL(value,'https://localhost').search.length>0);
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
      const header = isHome(page) ? 'header#portal-header' : 'header#portal-header.om-interior';
      const footer = isHome(page) ? 'footer.foot' : 'footer.foot-interior';
      if (await page.locator(header).count() !== 1) failures.push(`${href}: shared header missing`);
      if (await page.locator(footer).count() !== 1) failures.push(`${href}: shared footer missing`);
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
        if(expected===503) {expect(linked.headers()['cache-control']).toContain('no-store');expect(await linked.text()).toContain('id="portal-header"');}
      }
    });
    } finally { await page.close(); }
  }
  await info.attach('public-findings', { body: JSON.stringify({ links, failures, errors, expectedRefusalDiagnostics:diagnostics }, null, 2), contentType: 'application/json' });
  expect(failures).toEqual([]);
  expect(errors).toEqual([]);
});

test('Thais homepage opens the world menu and selects a profession', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const failedAssets = [];
  page.on('response', response => {
    if (new URL(response.url()).pathname.startsWith('/static/thais/') && !response.ok()) {
      failedAssets.push(`${response.status()} ${response.url()}`);
    }
  });
  const archive = await page.goto(`/${lang}/news`);
  expect(archive.status()).toBe(200);
  const leadStory = await page.locator(`main a[href^="/${lang}/news/"]`).evaluateAll(links =>
    links.length ? {href: links[0].getAttribute('href'), title: links[0].textContent.trim()} : null
  );
  await page.goto(`/${lang}/`);
  await expect(page.locator('.hero-art')).toHaveJSProperty('complete', true);
  expect(await page.locator('.hero-art').evaluate(image => image.naturalWidth)).toBeGreaterThan(0);
  const news = page.locator('.hero-news');
  await expect(news.getByRole('heading', {level: 1})).toHaveText(lang === 'pl' ? 'Aktualności' : 'News');
  await expect(news.locator('.hero-news-all')).toHaveAttribute('href', `/${lang}/news`);
  const stories = news.locator('.hero-story');
  if (leadStory) {
    const count = await stories.count();
    expect(count).toBeGreaterThan(0);
    expect(count).toBeLessThanOrEqual(3);
    await expect(stories.first().locator('.hero-story-title')).toHaveText(leadStory.title);
    await expect(stories.first().locator('a')).toHaveAttribute('href', leadStory.href);
    for (const story of await stories.all()) {
      const summary = story.locator('summary');
      await expect(summary.locator('time')).toHaveAttribute('datetime', /^\d{4}-\d{2}-\d{2}$/);
      await expect(summary.locator('.hero-story-toggle')).toBeVisible();
      const link = story.getByRole('link', {includeHidden: true});
      await expect(link).toBeHidden();
      await summary.focus();
      await page.keyboard.press('Enter');
      await expect(link).toBeVisible();
      expect(await link.getAttribute('href')).toMatch(new RegExp(`^/${lang}/news/.+`));
      await summary.click();
      await expect(link).toBeHidden();
    }
  } else {
    await expect(stories).toHaveCount(0);
    await expect(news.locator('.hero-news-status')).toHaveText(lang === 'pl'
      ? 'Brak opublikowanych aktualności.' : 'No news has been published yet.');
    await expect(page.locator('#aktualnosci .actions')).toHaveCount(0);
  }
  await expect(page.locator('#portal-header aside')).toHaveCount(0);
  await expect(page.locator('#aktualnosci .hero-today')).toHaveCount(0);
  const mobileToggle = page.locator('#portal-menu-toggle');
  const mobile = info.project.use.viewport.width === 390;
  if (mobile) {
    await mobileToggle.click();
    await expect(mobileToggle).toHaveAttribute('aria-expanded', 'true');
  }
  const world = page.locator('#portal-world-button');
  const panel = page.locator('#portal-world-panel');
  await world.focus();
  await page.keyboard.press('ArrowDown');
  await expect(world).toHaveAttribute('aria-expanded', 'true');
  await expect(panel.locator('a').first()).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(panel).toBeHidden();
  await expect(world).toBeFocused();
  await world.click();
  const bounds = await panel.boundingBox();
  expect(bounds.x).toBeGreaterThanOrEqual(0);
  expect(bounds.x + bounds.width).toBeLessThanOrEqual(info.project.use.viewport.width);
  await panel.locator('a[href="#profesje"]').click();
  await expect(page.locator('#profesje')).toBeFocused();
  if (mobile) await expect(mobileToggle).toHaveAttribute('aria-expanded', 'false');
  const tabs = page.getByRole('tab');
  await expect(tabs).toHaveCount(5);
  await tabs.last().click();
  await expect(tabs.last()).toHaveAttribute('aria-selected', 'true');
  await expect(page.locator('#profil h3')).toHaveText(lang === 'pl' ? 'Mnich' : 'Monk');
  await expect(page.locator('#status a')).toHaveCount(2);
  await expect(page.locator('#status a').first()).toHaveAttribute('href', `/${lang}/worlds`);
  await expect(page.locator('#status a').last()).toHaveAttribute('href', `/${lang}/online`);
  const audit = await new AxeBuilder({page}).analyze();
  expect(audit.violations.filter(v => ['serious', 'critical'].includes(v.impact))).toEqual([]);
  expect(failedAssets).toEqual([]);
});
