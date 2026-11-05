// Run after exporting the rendered PostgreSQL HTTP fixture with
// PORTAL_COMMERCE_RENDER_PATH. Uses the existing browser test installation.
const { chromium } = require('playwright');
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const rendered = process.env.PORTAL_COMMERCE_RENDER_PATH;
assert.ok(rendered, 'Set PORTAL_COMMERCE_RENDER_PATH to the rendered test fixture');
const staticRoot = path.resolve(__dirname, '../../../../static/css');
const server = http.createServer((request, response) => {
  const styles = { '/static/css/tokens.css': 'tokens.css', '/static/css/components.css': 'components.css' };
  const style = styles[request.url];
  response.setHeader('Content-Type', style ? 'text/css' : 'text/html; charset=utf-8');
  response.end(fs.readFileSync(style ? path.join(staticRoot, style) : rendered));
});
(async () => {
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  let browser;
  try {
    browser = await chromium.launch({ headless: true, executablePath: process.env.PORTAL_CHROMIUM_PATH });
    const page = await browser.newPage({ viewport: { width: 360, height: 800 } });
    await page.goto(`http://127.0.0.1:${server.address().port}/admin/commerce`);
    assert.equal(await page.locator('h1').innerText(), 'Commerce');
    const result = await page.evaluate(() => {
      const controls = [...document.querySelectorAll('input:not([type=hidden]),textarea,button')];
      function luminance(color) {
        const rgb = color.match(/[\d.]+/g).slice(0,3).map(Number).map(v => {
          const n=v/255; return n<=0.04045 ? n/12.92 : ((n+0.055)/1.055)**2.4;
        });
        return rgb[0]*0.2126+rgb[1]*0.7152+rgb[2]*0.0722;
      }
      const contrast = [...document.querySelectorAll('label,h1,h2,p,button,input:not([type=hidden]),textarea')].every(e => {
        const style=getComputedStyle(e);
        let parent=e;
        while (parent && ['rgba(0, 0, 0, 0)','transparent'].includes(getComputedStyle(parent).backgroundColor)) parent=parent.parentElement;
        const bg=parent ? getComputedStyle(parent).backgroundColor : 'rgb(255, 255, 255)';
        const a=luminance(style.color),b=luminance(bg);
        const ratio=(Math.max(a,b)+0.05)/(Math.min(a,b)+0.05);
        return ratio >= (parseFloat(style.fontSize)>=24 ? 3 : 4.5);
      });
      return {
        contrast,
        fits: document.documentElement.scrollWidth <= innerWidth,
        labels: controls.every(e => e.tagName === 'BUTTON' || e.labels?.length || e.getAttribute('aria-label')),
        targets: controls.every(e => e.getBoundingClientRect().height >= 24),
      };
    });
    assert.deepEqual(result, { contrast: true, fits: true, labels: true, targets: true });
    await page.keyboard.press('Tab');
    assert.equal(await page.locator(':focus').getAttribute('name'), 'target');
    assert.equal(await page.locator(':focus').evaluate(e => getComputedStyle(e).outlineStyle), 'solid');
    console.log('Commerce browser: 360 px overflow, text contrast, labels, target sizes and keyboard focus passed');
  } finally {
    if (browser) await browser.close();
    await new Promise(resolve => server.close(resolve));
  }
})().catch(error => { console.error(error.message); process.exitCode = 1; });
