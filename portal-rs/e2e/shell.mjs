import fs from 'node:fs';
import {expect, request} from '@playwright/test';
import {waitForWikiBudget} from './helpers.mjs';
import AxeBuilder from '@axe-core/playwright';

export function isHome(page) {
  return /^\/(?:pl|en)\/?$/.test(new URL(page.url()).pathname);
}

const registry = JSON.parse(fs.readFileSync(new URL('./inventory.json', import.meta.url), 'utf8')).sections;
const shellLinks = 'header#portal-header a, #podglad-konta a, footer.foot a';
const pathKey = path => path.replace(/^\/(pl|en)(?=\/|$)/, '').replace(/\/$/, '');

async function assertModuleNavigation(page) {
  await expect(page.locator('header#portal-header.om-interior')).toHaveCount(1);
  await expect(page.locator('footer.foot-interior')).toHaveCount(1);
  const active = await page.locator(shellLinks).evaluateAll(nodes => nodes
    .filter(n => n.getAttribute('aria-current') === 'page' && !n.hasAttribute('hreflang'))
    .map(n => n.pathname));
  const destinations = await page.locator(shellLinks).evaluateAll(nodes => nodes.map(n => n.pathname));
  const available = new Set(destinations.map(pathKey));
  const current = pathKey(new URL(page.url()).pathname);
  const matches = registry.map(s => pathKey('/' + s.path.replace(/^\//, '')))
    .filter(path => available.has(path) && (current === path || current.startsWith(path + '/')))
    .sort((a, b) => b.length - a.length);
  if (matches.length) {
    expect([...new Set(active.map(pathKey))], 'current registry destination').toEqual([matches[0]]);
  }
}

export async function assertShell(page, {accessibility=true,status=200}={}) {
  const home = isHome(page);
  if (home) {
    await expect(page.locator('header a.om-brand')).toHaveAttribute('aria-current','page');
  } else {
    await assertModuleNavigation(page);
  }
  await expect(page.locator('header#portal-header')).toHaveCount(1);
  await expect(page.locator('header #portal-navigation')).toHaveCount(1);
  await expect(page.locator('footer.foot')).toHaveCount(1);
  await expect(page.getByRole('main')).toHaveCount(1);
  await expect(page.locator('h1')).toHaveCount(1);
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'horizontal overflow').toBe(true);
  await page.keyboard.press('Tab');
  await page.locator('a[href="#main"]').first().focus();
  const focus=await page.evaluate(()=>{const s=getComputedStyle(document.activeElement);return document.activeElement.matches(':focus-visible')&&(s.outlineStyle!=='none'||s.boxShadow!=='none');});
  expect(focus,'visible keyboard focus').toBe(true);
  await page.keyboard.press('Enter');
  await expect(page.locator('#main'),'skip link reaches the framed page content').toBeFocused();
  for(const form of await page.locator('form[method=post i]').evaluateAll(nodes=>nodes.map(f=>({action:f.getAttribute('action')||location.pathname,fields:Object.fromEntries(new FormData(f))})))) {
    const {csrfRejected}=await import('./helpers.mjs');
    await csrfRejected(page,form.action,form.fields);
  }
  const links=await page.locator('main a[href], footer a[hreflang]').evaluateAll(nodes=>nodes.map(n=>({href:n.getAttribute('href'),language:n.hasAttribute('hreflang')})).filter(n=>n.href.startsWith('/')&&!n.href.startsWith('//')));
  // A language link updates the preference cookie. Follow it with an
  // isolated copy of the actor's session so the probe cannot steer other tabs.
  const languageProbe = links.some(link => link.href.startsWith('/language/'))
    ? await request.newContext({
      baseURL: new URL(page.url()).origin,
      ignoreHTTPSErrors: true,
      storageState: await page.context().storageState(),
    }) : null;
  try {
    for(const link of links) {
      await waitForWikiBudget(link.href);
      const client = link.href.startsWith('/language/') ? languageProbe : page.request;
      const response=await client.get(link.href,{headers:{accept:'text/html'}});
      expect(link.language?[200,503,status]:[200,503],`linked destination ${link.href}`).toContain(response.status());
      if(response.status()===503)expect(response.headers()['cache-control']).toContain('no-store');
    }
  } finally {
    await languageProbe?.dispose();
  }
  if(accessibility) {
    const axe=await new AxeBuilder({page}).analyze();
    expect(axe.violations.filter(v=>['serious','critical'].includes(v.impact)).map(v=>({id:v.id,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))}))).toEqual([]);
  }
}
