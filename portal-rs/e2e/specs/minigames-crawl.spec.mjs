import { test, expect } from '../fixtures.mjs';
import { login, pool } from '../helpers.mjs';
import { seedAccount } from '../seed-helpers.mjs';

test('league, collection and puzzles serve a concurrent signed-in crawl', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seedAccount(lang);
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

  const db = pool();
  const publisher = await db.connect();
  try {
    await publisher.query('BEGIN');
    await publisher.query("SET LOCAL statement_timeout = '3s'");
    await publisher.query(
      "SELECT pg_advisory_xact_lock('league_subjects'::regclass::oid::int, 94000003)",
    );

    const leagueCrawl = Promise.allSettled(Array.from({ length: 12 }, async () =>
      (await page.request.get(league, { maxRedirects: 0 })).status(),
    ));
    try {
      await expect.poll(async () => {
        const { rows } = await publisher.query(`
          SELECT EXISTS (
            SELECT FROM pg_locks WHERE locktype = 'advisory' AND NOT granted
              AND mode = 'ShareLock' AND database = (SELECT oid FROM pg_database WHERE datname = current_database())
              AND classid = 'league_subjects'::regclass::oid AND objid = 94000003
          ) AS blocked
        `);
        return rows[0].blocked;
      }, { message: 'a League reader must be waiting on the held publication fence' }).toBe(true);

      const healthy = await Promise.all(['collection', 'puzzles'].map(async (route) =>
        (await page.request.get(`/${lang}/${route}`, { maxRedirects: 0 })).status(),
      ));
      expect(healthy).toEqual([200, 200]);
    } finally {
      const blocked = await leagueCrawl;
      expect(blocked).toEqual(Array.from({ length: 12 }, () => ({ status: 'fulfilled', value: 503 })));
    }
  } finally {
    await publisher.query('ROLLBACK').finally(() => publisher.release());
    await db.end();
  }
  expect((await page.request.get(league, { maxRedirects: 0 })).status()).toBe(200);
});
