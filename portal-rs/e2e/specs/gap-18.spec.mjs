import { test, expect } from '../fixtures.mjs';
import { randomBytes } from 'node:crypto';
import { pool } from '../helpers.mjs';

test('a visitor finds profiles and compares observed character levels', async ({ page }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const suffix = randomBytes(5).toString('hex');
  const names = [`Aldric ${suffix}`, `Carina ${suffix}`];
  const levels = [412, 237];
  const db = pool();
  const connection = await db.connect();
  const stamp = () => Math.floor(Date.now() / 1000);
  const authority = process.env.OTERYN_LCFA_SOURCE_AUTHORITY;
  const profileAuthority = process.env.PORTAL_PUBLIC_PROJECTIONS_AUTHORITY;
  const ids = [];
  let saved;
  const assertWidth = async () => {
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
      page.viewportSize().width,
    );
  };

  try {
    const state = (await connection.query(
      'SELECT highest_epoch::text AS epoch FROM public_projection_state WHERE id=1',
    )).rows[0];
    const feed = (await connection.query('SELECT * FROM account_character_feed WHERE id=1')).rows[0];
    const watermark = (await connection.query(
      "SELECT * FROM public_snapshots WHERE family='character_profile' AND subject='' AND is_watermark",
    )).rows[0];
    saved = { state, feed, watermark };
    const epoch = state.epoch ?? '1';
    const feedEpoch = feed.highest_epoch ?? '1';
    const { world } = (await connection.query('SELECT uuidv7() AS world')).rows[0];
    const heartbeat = async (complete = stamp()) => {
      await connection.query(
        'UPDATE account_character_feed SET highest_epoch=$1,watermark_epoch=$1,source_authority=$2,complete_through=$3,observed_at=$3 WHERE id=1',
        [feedEpoch, authority, stamp()],
      );
      await connection.query(
        `INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload)
         VALUES('character_profile','',true,$1,1,$2,$3)
         ON CONFLICT(family,subject,is_watermark) DO UPDATE SET epoch=$1,watermark=$2,payload=$3`,
        [epoch, complete, { source_authority: profileAuthority }],
      );
    };

    // Seed disposable Platform projection rows; the existing worker consumes the level observations.
    await connection.query('BEGIN');
    await connection.query('UPDATE public_projection_state SET highest_epoch=$1 WHERE id=1', [epoch]);
    for (const [index, name] of names.entries()) {
      const { account, character } = (await connection.query(
        'SELECT uuidv7() AS account,uuidv7() AS character',
      )).rows[0];
      ids.push({ account, character });
      await connection.query(
        `INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at)
         VALUES($1,$2,'non-loginable-test','active',$3,now())`,
        [account, `${account}@example.invalid`, lang],
      );
      await connection.query(
        `INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest)
         VALUES($1,$2,$3,'1',$4,$5)`,
        [account, authority, feedEpoch, stamp(), Buffer.alloc(32)],
      );
      await connection.query(
        "INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,$4,'AVAILABLE')",
        [account, character, world, name],
      );
      await connection.query(
        'INSERT INTO character_profiles(account_id,character_id,hidden) VALUES($1,$2,false)',
        [account, character],
      );
      const observed = stamp();
      await connection.query(
        "INSERT INTO public_snapshots(family,subject,epoch,version,watermark,payload) VALUES('character_profile',$1,$2,1,$3,$4)",
        [character, epoch, observed, {
          contract_version: 1, operation: 'PublishCharacterProfileV1', source_authority: profileAuthority,
          character_id: character, projection_epoch: epoch, projection_revision: '1',
          source_observed_at: String(observed),
          profile: [{ world_id: world, name, level: String(levels[index]), vocation: 4, guild: [], guild_visible: false }],
        }],
      );
    }
    await heartbeat();
    await connection.query('COMMIT');

    await expect.poll(async () => {
      const result = await connection.query(
        'SELECT count(DISTINCT character_ref)::int AS total FROM stats_character_daily WHERE character_ref=ANY($1::uuid[])',
        [ids.map(({ character }) => character)],
      );
      return result.rows[0].total;
    }, { timeout: 90_000, intervals: [1000, 5000] }).toBe(2);

    await heartbeat();
    expect((await page.goto(`/${lang}/news`)).status()).toBe(200);
    if (page.viewportSize().width <= 960) await page.locator('.om-fallback-menu > summary').click();
    await page.locator('#portal-community-panel').locator('..').locator('summary').click();
    await page.locator(`#portal-community-panel a[href="/${lang}/characters"]`).click();
    await page.locator('#name').fill(names[0].toLowerCase());
    await page.locator('main form button[type=submit]').click();
    await expect(page.getByRole('heading', { name: names[0], exact: true })).toBeVisible();
    await assertWidth();

    await heartbeat();
    await page.locator(`a[href="/${lang}/search?kind=characters"]`).click();
    await page.locator('#q').fill(names[0].toLowerCase());
    await page.locator('main form button[type=submit]').click();
    await page.getByRole('link', { name: names[0], exact: true }).click();
    await page.locator('a[href*="/stats/compare?first="]').click();
    await expect(page.locator('#first')).toHaveValue(names[0]);
    await page.locator('#second').fill(names[1]);
    await page.locator('#metric').selectOption('level');
    await page.locator('main form button[type=submit]').click();
    for (const [index, name] of names.entries()) {
      const result = page.locator('main article').filter({ has: page.getByRole('heading', { name, exact: true }) });
      await expect(result.getByText(String(levels[index]), { exact: true })).toBeVisible();
    }
    await assertWidth();

    await heartbeat();
    await page.locator('#metric').selectOption('experience');
    await page.locator('main form button[type=submit]').click();
    const unavailable = lang === 'pl' ? 'Niedostępne' : 'Unavailable';
    await expect(page.locator('main article').getByText(unavailable, { exact: true })).toHaveCount(2);

    await heartbeat();
    await connection.query(
      'UPDATE character_profiles SET hidden=true WHERE account_id=$1 AND character_id=$2',
      [ids[1].account, ids[1].character],
    );
    const query = new URLSearchParams({ first: names[0], second: names[1], metric: 'level' });
    const hidden = await page.request.get(`/${lang}/stats/compare?${query}`);
    expect(hidden.status()).toBe(404);
    expect(await hidden.text()).not.toContain(names[0]);
    const search = new URLSearchParams({ q: names[1], kind: 'characters' });
    const hiddenSearch = await page.request.get(`/${lang}/search?${search}`);
    expect(hiddenSearch.status()).toBe(200);
    expect(await hiddenSearch.text()).not.toContain(`/characters/${encodeURIComponent(names[1])}`);
    expect((await page.request.get(`/${lang}/characters/${encodeURIComponent(names[1])}`)).status()).toBe(404);

    expect((await page.request.get(`/${lang}/characters?name=${'x'.repeat(65)}`)).status()).toBe(400);
    expect((await page.request.get(`/${lang}/search?kind=characters&q=${'x'.repeat(65)}`)).status()).toBe(422);
    query.set('metric', 'unknown');
    expect((await page.request.get(`/${lang}/stats/compare?${query}`)).status()).toBe(400);

    await heartbeat(stamp() - 60);
    const stale = await page.request.get(`/${lang}/characters/${encodeURIComponent(names[0])}`);
    expect(stale.status()).toBe(503);
    expect(await stale.text()).toContain(unavailable);
  } finally {
    try {
      await connection.query('ROLLBACK');
      await connection.query('BEGIN');
      await connection.query('SELECT id FROM public_projection_state WHERE id=1 FOR UPDATE');
      const characters = ids.map(({ character }) => character);
      await connection.query('DELETE FROM stats_character_daily WHERE character_ref=ANY($1::uuid[])', [characters]);
      await connection.query('DELETE FROM stats_character_events WHERE character_ref=ANY($1::uuid[])', [characters]);
      await connection.query("DELETE FROM stats_projection_cursors WHERE family='character_profile' AND subject=ANY($1::text[])", [characters]);
      await connection.query("DELETE FROM public_snapshots WHERE family='character_profile' AND subject=ANY($1::text[])", [characters]);
      await connection.query('DELETE FROM accounts WHERE id=ANY($1::uuid[])', [ids.map(({ account }) => account)]);
      if (saved) {
        const { state, feed, watermark } = saved;
        await connection.query('UPDATE public_projection_state SET highest_epoch=$1 WHERE id=1', [state.epoch]);
        await connection.query(
          'UPDATE account_character_feed SET highest_epoch=$1,watermark_epoch=$2,source_authority=$3,complete_through=$4,observed_at=$5 WHERE id=1',
          [feed.highest_epoch, feed.watermark_epoch, feed.source_authority, feed.complete_through, feed.observed_at],
        );
        await connection.query("DELETE FROM public_snapshots WHERE family='character_profile' AND subject='' AND is_watermark");
        if (watermark) {
          await connection.query(
            `INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload,previous_snapshot)
             VALUES('character_profile','',true,$1,$2,$3,$4,$5)`,
            [watermark.epoch, watermark.version, watermark.watermark, watermark.payload, watermark.previous_snapshot],
          );
        }
      }
      await connection.query('COMMIT');
    } finally {
      connection.release();
      await db.end();
    }
  }
});
