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

  const control = process.env.PORTAL_E2E_LEAGUE_CONTROL;
  expect(control, 'fixture publisher must be running').toBeTruthy();
  const db = pool();
  try {
    const hold = await page.request.post(`${control}/hold-watermarks`);
    expect(hold.status()).toBe(200);
    const { watermark } = await hold.json();
    expect(watermark).toMatch(/^\d+$/);
    const watermarks = async () => {
      const { rows } = await db.query(`
        SELECT ready, event_complete::text, season_complete::text
        FROM league_feed_state WHERE authority = $1
      `, ['oteryn:game:primary']);
      return rows;
    };
    const held = [{ ready: true, event_complete: watermark, season_complete: watermark }];
    await expect.poll(watermarks, { message: 'the worker must install both held watermarks' }).toEqual(held);

    const routes = ['league', 'collection', 'puzzles'];
    const statuses = await Promise.all(Array.from({ length: 12 }, () =>
      Promise.all(routes.map(async (route) => {
        const response = await page.request.get(`/${lang}/${route}`, { maxRedirects: 0 });
        return response.status();
      })),
    ));
    for (const row of statuses) expect(row, routes.join(', ')).toEqual([200, 200, 200]);

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
    }
    expect((await page.request.get(league, { maxRedirects: 0 })).status()).toBe(200);
    expect(await watermarks()).toEqual(held);
  } finally {
    await db.end();
    expect((await page.request.post(`${control}/resume`)).status()).toBe(200);
  }
});

test('League SQL refusals release connections and preserve the localized response', async ({ page, request }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seedAccount(lang);
  await login(page, lang, account);
  const league = `/${lang}/league`;
  const db = pool();
  const blocker = await db.connect();
  try {
    expect((await request.get('/readyz')).status()).toBe(200);
    await blocker.query('BEGIN');
    await blocker.query("SET LOCAL statement_timeout = '3s'");
    await blocker.query('LOCK TABLE web_rate_limits IN ACCESS EXCLUSIVE MODE');
    const quotaWaiters = async () => {
      const { rows } = await db.query(`
        SELECT count(*)::int AS waiting FROM pg_locks l JOIN pg_stat_activity a USING (pid)
        WHERE l.relation = 'web_rate_limits'::regclass AND NOT l.granted
          AND a.datname = current_database() AND a.query LIKE 'WITH clock AS MATERIALIZED%'
      `);
      return rows[0].waiting;
    };
    for (let wave = 0; wave < 4; wave++) {
      const responses = Promise.allSettled(Array.from({ length: 4 }, async () =>
        (await page.request.get(league, { maxRedirects: 0 })).status(),
      ));
      try {
        if (wave === 0) {
          await expect.poll(quotaWaiters, { message: 'requests must reach the locked quota table' }).toBeGreaterThan(0);
        }
      } finally {
        expect(await responses).toEqual(Array.from({ length: 4 }, () => ({ status: 'fulfilled', value: 503 })));
      }
      expect(await quotaWaiters()).toBe(0);
      expect((await request.get('/readyz', { timeout: 2000 })).status()).toBe(200);
    }
    await blocker.query('ROLLBACK');

    await blocker.query('BEGIN');
    await blocker.query("SET LOCAL statement_timeout = '3s'");
    await blocker.query('LOCK TABLE web_sessions IN ACCESS EXCLUSIVE MODE');
    const response = await page.goto(league);
    expect(response.status()).toBe(503);
    expect(response.headers()['cache-control']).toContain('no-store');
    expect((await page.request.get('/static/thais/shell.css', { timeout: 2000 })).status()).toBe(200);
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    await expect(page.getByRole('heading', {
      level: 1,
      name: lang === 'pl' ? 'Usługa chwilowo niedostępna' : 'Temporarily unavailable',
    })).toBeVisible();
    expect((await request.get('/readyz', { timeout: 2000 })).status()).toBe(200);
  } finally {
    await blocker.query('ROLLBACK').finally(() => blocker.release());
    await db.end();
  }
});
