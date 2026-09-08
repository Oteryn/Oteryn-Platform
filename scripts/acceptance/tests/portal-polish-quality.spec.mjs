import { test, expect } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const output = resolve(root, 'artifacts/acceptance/portal-polish');

test.describe.configure({ retries: 0 });

test('@smoke @portal-polish collect exact-source visual quality baseline', async ({ page }) => {
  test.setTimeout(120_000);
  mkdirSync(output, { recursive: true });
  const exactHead = execFileSync('git', ['rev-parse', 'HEAD'], {
    cwd: root, encoding: 'utf8', timeout: 5000,
  }).trim();
  // Temporary baseline transport for the isolated design workspace. Only tracked
  // source is exported, never .git, untracked .env, cookies or runtime/user data.
  // Remove this archive operation before this candidate is marked ready.
  execFileSync('git', ['archive', '--format=tar', `--output=${resolve(output, 'tracked-source.tar')}`, 'HEAD'], {
    cwd: root, timeout: 30_000, stdio: 'pipe',
  });

  const records = [];
  for (const width of [390, 820, 1440, 1920]) {
    await page.setViewportSize({ width, height: 1000 });
    for (const path of ['/en', '/pl', '/en/news', '/en/wiki', '/en/highscores', '/login']) {
      const response = await page.goto(path);
      await expect(page.locator('main')).toBeVisible();
      await page.evaluate(() => document.fonts.ready);
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
      writeFileSync(resolve(output, 'quality-baseline.json'), JSON.stringify({ exactHead, records }, null, 2));
      expect(response?.status(), path).toBe(200);
      expect(metrics.scrollWidth, `${path} at ${width}px`).toBeLessThanOrEqual(metrics.viewport + 1);
    }
  }
});
