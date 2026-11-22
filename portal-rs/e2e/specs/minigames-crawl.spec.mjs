import { test, expect } from '../fixtures.mjs';
import { login, register } from '../helpers.mjs';

test('league, collection and puzzles serve a concurrent signed-in crawl', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await login(page, lang, account);

  const league = `/${lang}/league`;
  await expect.poll(async () => (await page.request.get(league)).status(), {
    timeout: 45_000,
    message: 'the harness league publisher must deliver a current watermark',
  }).toBe(200);

  const routes = ['league', 'collection', 'puzzles'];
  const statuses = await Promise.all(Array.from({ length: 12 }, () =>
    Promise.all(routes.map(async (route) => {
      const response = await page.request.get(`/${lang}/${route}`, { maxRedirects: 0 });
      return response.status();
    })),
  ));
  for (const row of statuses) expect(row, routes.join(', ')).toEqual([200, 200, 200]);
});
