import { test, expect } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { runBinary } from './helpers.mjs';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const output = resolve(root, 'artifacts/acceptance/portal-review');
const viewports = [
  ['desktop', { width: 1440, height: 1000 }],
  ['phone', { width: 390, height: 844 }],
  ['tablet', { width: 820, height: 1180 }],
  ['wide', { width: 1920, height: 1080 }],
];

test.describe.configure({ retries: 0 });

test.beforeAll(() => {
  runBinary('php', ['scripts/acceptance/seed-homepage-navigation-seo.php']);
  mkdirSync(output, { recursive: true });
  // Baseline-only transport for the network-isolated design workspace. Remove
  // before final readiness. git archive excludes .git, .env and runtime state.
  execFileSync('git', ['archive', '--format=tar', `--output=${resolve(output, 'tracked-source.tar')}`, 'HEAD'], {
    cwd: root, timeout: 30_000, stdio: 'pipe',
  });
});

test('@smoke @portal-review sanitized public renders at four viewport sizes', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/en');
  const destinations = await page.locator('a[href]').evaluateAll((links) => [...new Set(links
    .map((link) => new URL(link.href, location.href))
    .filter((url) => url.origin === location.origin && /^\/en\//u.test(url.pathname)
      && /\/(news|online|highscores|servers|download|wiki|guilds|support)$/u.test(url.pathname))
    .map((url) => url.pathname))]);
  const records = [];
  const exactHead = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8', timeout: 5000 }).trim();

  for (const [viewportName, viewport] of viewports) {
    await page.setViewportSize(viewport);
    const paths = ['desktop', 'phone'].includes(viewportName)
      ? ['/en', '/pl', '/login', '/register', '/en/today', ...destinations]
      : ['/en', '/pl'];
    for (const path of paths) {
      const response = await page.goto(path);
      await expect(page.locator('main')).toBeVisible();
      await page.evaluate(() => document.fonts.ready);
      const name = `${viewportName}-${path.replace(/^\//u, '').replaceAll('/', '-')}`;
      await page.screenshot({ path: resolve(output, `${name}.png`), fullPage: true, animations: 'disabled' });
      // Never export form/session/CSRF values. This is a guest-only, synthetic
      // fixture snapshot; auth enrollment, recovery and token routes are excluded.
      const html = await page.evaluate(() => {
        const clone = document.documentElement.cloneNode(true);
        for (const input of clone.querySelectorAll('input')) input.removeAttribute('value');
        for (const node of clone.querySelectorAll('meta[name="csrf-token"], input[name="_token"]')) node.remove();
        for (const node of clone.querySelectorAll('[src], [href], [action]')) {
          for (const attribute of ['src', 'href', 'action']) {
            const value = node.getAttribute(attribute);
            if (!value) continue;
            try {
              const url = new URL(value, location.href);
              if (url.origin === location.origin) node.setAttribute(attribute, `${url.pathname}${url.search}${url.hash}`);
            } catch { /* Keep non-URL local references unchanged. */ }
          }
        }
        return `<!DOCTYPE html>\n${clone.outerHTML}`;
      });
      writeFileSync(resolve(output, `${name}.html`), html);
      const dimensions = await page.evaluate(() => ({ width: innerWidth, scrollWidth: document.documentElement.scrollWidth }));
      records.push({ path, viewport, status: response?.status(), screenshot: `${name}.png`, html: `${name}.html`, ...dimensions });
      writeFileSync(resolve(output, 'manifest.json'), JSON.stringify({ exactHead, fixture: 'seed-homepage-navigation-seo.php', records }, null, 2));
      expect(response?.status(), path).toBe(200);
    }
  }
});
