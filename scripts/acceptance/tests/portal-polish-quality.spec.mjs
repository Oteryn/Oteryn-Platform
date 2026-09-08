import { test, expect } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const output = resolve(root, 'artifacts/acceptance/portal-polish');

test.describe.configure({ retries: 0 });

test('@smoke @portal-polish measure rendered asset sizes and layout quality', async ({ page }) => {
  test.setTimeout(120_000);
  mkdirSync(output, { recursive: true });
  const exactHead = execFileSync('git', ['rev-parse', 'HEAD'], {
    cwd: root, encoding: 'utf8', timeout: 5000,
  }).trim();
  const records = [];
  for (const width of [390, 820, 1440, 1920]) {
    await page.setViewportSize({ width, height: 1000 });
    for (const path of ['/en', '/pl', '/en/news', '/en/wiki', '/en/highscores', '/login']) {
      const response = await page.goto(path);
      await expect(page.locator('main')).toBeVisible();
      await page.evaluate(() => document.fonts.ready);
      for (const image of await page.locator('img[loading="lazy"]').all()) {
        if (await image.isVisible()) {
          await image.scrollIntoViewIfNeeded();
          await expect(image).toHaveJSProperty('complete', true);
        }
      }
      await page.evaluate(() => window.scrollTo(0, 0));
      // Public fixture DOM only, for offline visual iteration; never authenticated HTML.
      if (width === 1440 && path.startsWith('/en')) {
        const html = await page.evaluate(() => {
          const clone = document.documentElement.cloneNode(true);
          clone.querySelectorAll('script, meta[name="csrf-token"], input[type="hidden"]').forEach((node) => node.remove());
          clone.querySelectorAll('input').forEach((node) => node.removeAttribute('value'));
          clone.querySelectorAll('textarea').forEach((node) => { node.textContent = ''; });
          clone.querySelectorAll('[nonce]').forEach((node) => node.removeAttribute('nonce'));
          return `<!doctype html>${clone.outerHTML}`;
        });
        writeFileSync(resolve(output, `${path.replaceAll('/', '-')}.html`), html);
      }
      const metrics = await page.evaluate(() => ({
        viewport: document.documentElement.clientWidth,
        scrollWidth: document.documentElement.scrollWidth,
        height: document.documentElement.scrollHeight,
        stylesheetCount: document.styleSheets.length,
        images: [...document.images].filter((image) => image.currentSrc && image.getBoundingClientRect().width).map((image) => ({
          path: new URL(image.currentSrc, location.href).pathname,
          naturalWidth: image.naturalWidth,
          naturalHeight: image.naturalHeight,
          displayedWidth: Math.round(image.getBoundingClientRect().width),
          displayedHeight: Math.round(image.getBoundingClientRect().height),
          loading: image.loading,
        })),
        resources: performance.getEntriesByType('resource')
          .filter((resource) => ['img', 'css', 'link', 'script'].includes(resource.initiatorType))
          .map((resource) => ({
            path: new URL(resource.name, location.href).pathname,
            transferSize: resource.transferSize,
            decodedBodySize: resource.decodedBodySize,
            durationMs: Math.round(resource.duration),
          })),
      }));
      records.push({ path, width, status: response?.status(), ...metrics });
      writeFileSync(resolve(output, 'quality-current.json'), JSON.stringify({ exactHead, records }, null, 2));
      expect(response?.status(), path).toBe(200);
      for (const image of metrics.images.filter((image) => image.path.endsWith('.webp'))) {
        expect(image.naturalWidth, `${path}: loaded image ${image.path}`).toBeGreaterThan(0);
        expect(image.displayedWidth, `${path}: avoid enlarging ${image.path}`).toBeLessThanOrEqual(image.naturalWidth + 1);
      }
      expect(metrics.scrollWidth, `${path} at ${width}px`).toBeLessThanOrEqual(metrics.viewport + 1);
    }
  }
});
