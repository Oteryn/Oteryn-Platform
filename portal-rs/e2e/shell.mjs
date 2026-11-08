import {expect} from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

export async function assertShell(page, {accessibility=true,status=200}={}) {
  await expect(page.locator('body.op-shell > .hx > .hx-head')).toHaveCount(1);
  await expect(page.locator('.hx-head .hx-nav')).toHaveCount(1);
  await expect(page.locator('body.op-shell > .hx > .hx-foot')).toHaveCount(1);
  await expect(page.locator('h1')).toHaveCount(1);
  const normalize=value=>new URL(value,page.url()).pathname.replace(/^\/(pl|en)(?=\/|$)/,'').replace(/\/$/,'');
  const current=normalize(page.url());
  const destinations=await page.locator('.hx-nav__menu a').evaluateAll(nodes=>nodes.map(n=>n.getAttribute('href')));
  const active=destinations.filter(href=>{const prefix=normalize(href);return prefix&&(current===prefix||current.startsWith(prefix+'/'));}).sort((a,b)=>normalize(b).length-normalize(a).length);
  await expect(page.locator('.hx-nav__menu a[aria-current=page]')).toHaveCount(active.length?1:0);
  if(active.length)await expect(page.locator(`.hx-nav__menu a[href="${active[0]}"]`)).toHaveAttribute('aria-current','page');
  if(!current)await expect(page.locator('.hx-nav__list > li:first-child a')).toHaveAttribute('aria-current','page');
  await expect(page.locator('.hx > :is(.op-shell__page, main.op-main, main.hx-main) main, .hx > main')).toHaveCount(1);
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'horizontal overflow').toBe(true);
  await page.keyboard.press('Tab');
  await page.locator('.op-skip').focus();
  const focus=await page.evaluate(()=>{const s=getComputedStyle(document.activeElement);return document.activeElement.matches(':focus-visible')&&(s.outlineStyle!=='none'||s.boxShadow!=='none');});
  expect(focus,'visible keyboard focus').toBe(true);
  await page.keyboard.press('Enter');
  await expect(page.locator('#main'),'skip link reaches the framed page content').toBeFocused();
  for(const form of await page.locator('form[method=post i]').evaluateAll(nodes=>nodes.map(f=>({action:f.getAttribute('action')||location.pathname,fields:Object.fromEntries(new FormData(f))})))) {
    const {csrfRejected}=await import('./helpers.mjs');
    await csrfRejected(page,form.action,form.fields);
  }
  const links=await page.locator('main a[href], .hx-account a[hreflang], .hx-foot a[hreflang]').evaluateAll(nodes=>nodes.map(n=>({href:n.getAttribute('href'),language:n.hasAttribute('hreflang')})).filter(n=>n.href.startsWith('/')&&!n.href.startsWith('//')));
  for(const link of links) {
    const response=await page.request.get(link.href,{headers:{accept:'text/html'}});
    expect(link.language?[200,503,status]:[200,503],`linked destination ${link.href}`).toContain(response.status());
    if(response.status()===503)expect(response.headers()['cache-control']).toContain('no-store');
  }
  if(accessibility) {
    const axe=await new AxeBuilder({page}).analyze();
    expect(axe.violations.filter(v=>['serious','critical'].includes(v.impact)).map(v=>({id:v.id,nodes:v.nodes.map(n=>({target:n.target,summary:n.failureSummary}))}))).toEqual([]);
  }
}
