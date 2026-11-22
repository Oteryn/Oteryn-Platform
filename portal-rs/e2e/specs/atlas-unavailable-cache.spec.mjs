import { test, expect } from '../fixtures.mjs';

test('cold Atlas views forbid caching for navigation and raw requests', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  for (const route of ['atlas', 'atlas/delivery']) {
    const path = `/${lang}/${route}`;
    const document = await page.goto(path);
    expect(document.status(), path).toBe(503);
    expect(document.headers()['cache-control']).toContain('no-store');
    await expect(page.locator('header#portal-header')).toHaveCount(1);

    for (const method of ['GET', 'HEAD']) {
      const raw = await page.request.fetch(path, {
        method,
        headers: { Accept: 'application/json' },
        maxRedirects: 0,
      });
      expect(raw.status(), `${method} ${path}`).toBe(503);
      expect(raw.headers()['cache-control']).toContain('no-store');
    }
  }
});
