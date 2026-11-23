import { test, expect } from '../fixtures.mjs';

function cacheDirectives(value = '') {
  const directives = [];
  let start = 0;
  let quoted = false;
  let escaped = false;
  for (let at = 0; at < value.length; at++) {
    const character = value[at];
    if (escaped) escaped = false;
    else if (quoted && character === '\\') escaped = true;
    else if (character === '"') quoted = !quoted;
    else if (!quoted && character === ',') {
      directives.push(value.slice(start, at).trim().toLowerCase());
      start = at + 1;
    }
  }
  if (!quoted) directives.push(value.slice(start).trim().toLowerCase());
  return directives;
}

test('cold Atlas views forbid caching for navigation and raw requests', async ({ page }, info) => {
  expect(cacheDirectives('private, extension="no-store", quoted="a,no-store,b"')).not.toContain('no-store');
  const lang = info.project.metadata.locale;
  for (const route of ['atlas', 'atlas/delivery']) {
    const path = `/${lang}/${route}`;
    const document = await page.goto(path);
    expect(document.status(), path).toBe(503);
    expect(cacheDirectives(document.headers()['cache-control'])).toContain('no-store');
    await expect(page.locator('header#portal-header')).toHaveCount(1);

    for (const method of ['GET', 'HEAD']) {
      const raw = await page.request.fetch(path, {
        method,
        headers: { Accept: 'application/json' },
        maxRedirects: 0,
      });
      expect(raw.status(), `${method} ${path}`).toBe(503);
      expect(cacheDirectives(raw.headers()['cache-control'])).toContain('no-store');
    }
  }
});
