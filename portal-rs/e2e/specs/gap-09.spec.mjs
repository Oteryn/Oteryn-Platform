import { randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { pool } from '../helpers.mjs';
import { messages, missingId, probe, seedWorld, visit } from '../public-depth-helpers.mjs';

function fixtureId() {
  const id = randomUUID();
  return `${id.slice(0, 14)}7${id.slice(15)}`;
}

test('a visitor picks a world, skill and vocation and sees the published ties', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const t = messages('crates/ui/i18n', lang);
  const world = await seedWorld();
  const db = pool();
  const account = fixtureId();
  const rows = [
    { rank: '1', name: 'Hidden Contender', value: '99' },
    { rank: '2', name: 'Zelda Contender', value: '95' },
    { rank: '2', name: 'Aldric Contender', value: '95' },
    { rank: '4', name: 'Beren Contender', value: '91' },
  ].map(row => ({ ...row, character_id: fixtureId(), vocation: 1, level: '100' }));
  const subject = `${world.id}:2:1`;
  const authority = process.env.OTERYN_LCFA_SOURCE_AUTHORITY;
  let publicEpochBefore, feedBefore, watermarksBefore;
  try {
    await page.goto(`/${lang}/`);
    const path = `/${lang}/rankings/experience`;
    const link = page.locator(`header a[href="${path}"]`).first();
    await expect(link).toHaveAttribute('href', path);
    await visit(page, await link.getAttribute('href'), 503, { lang, message: t['rankings-unavailable'] });

    const [state] = (await db.query('SELECT highest_epoch::text FROM public_projection_state WHERE id=1')).rows;
    publicEpochBefore = state.highest_epoch;
    const publicEpoch = publicEpochBefore ?? '1';
    [feedBefore] = (await db.query('SELECT * FROM account_character_feed WHERE id=1')).rows;
    const accountEpoch = feedBefore.highest_epoch ?? '1';
    watermarksBefore = (await db.query(
      "SELECT * FROM public_snapshots WHERE family IN ('highscores', 'character_profile') AND is_watermark AND subject=''",
    )).rows;

    const now = Math.floor(Date.now() / 1000);
    await db.query(
      "INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'non-loginable-test','active',$3,now())",
      [account, `${account}@example.invalid`, lang],
    );
    await db.query(
      "INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,$2,$3,'1',$4,$5)",
      [account, authority, accountEpoch, now, Buffer.alloc(32)],
    );
    await db.query(
      'UPDATE account_character_feed SET highest_epoch=$1,watermark_epoch=$1,source_authority=$2,complete_through=$3,observed_at=$3 WHERE id=1',
      [accountEpoch, authority, now],
    );
    await db.query('UPDATE public_projection_state SET highest_epoch=$1 WHERE id=1', [publicEpoch]);
    for (const [index, row] of rows.entries()) {
      await db.query(
        "INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,$4,'AVAILABLE')",
        [account, row.character_id, world.id, row.name],
      );
      await db.query('INSERT INTO character_profiles(account_id,character_id,hidden) VALUES($1,$2,$3)',
        [account, row.character_id, index === 0]);
      await db.query(
        "INSERT INTO public_snapshots(family,subject,epoch,version,watermark,payload) VALUES('character_profile',$1,$2,1,$3,$4)",
        [row.character_id, publicEpoch, now, { profile: [{ world_id: world.id, name: row.name }] }],
      );
    }
    await db.query(
      "INSERT INTO public_snapshots(family,subject,epoch,version,watermark,payload) VALUES('highscores',$1,$2,1,$3,$4)",
      [subject, publicEpoch, now, { world_id: world.id, category: 2, filter: 1, computed_at: String(now), rows }],
    );
    await db.query(
      `INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload)
       VALUES ('highscores','',true,$1,1,$2,'null'), ('character_profile','',true,$1,1,$2,'null')
       ON CONFLICT(family,subject,is_watermark) DO UPDATE SET epoch=$1, watermark=$2`,
      [publicEpoch, now],
    );

    const form = page.locator('main form');
    await form.getByLabel(t['rankings-world']).selectOption({ label: world.name });
    await form.getByLabel(t['rankings-category']).selectOption('magic');
    await form.getByLabel(t['rankings-vocation']).selectOption('1');
    const [response] = await Promise.all([
      page.waitForResponse(reply => reply.request().isNavigationRequest() && reply.url().includes('skill=magic')),
      form.getByRole('button', { name: t['rankings-apply'] }).click(),
    ]);
    expect(response.status()).toBe(200);
    expect(response.headers()['cache-control']).toContain('no-store');
    await expect(page.locator('tbody tr')).toHaveCount(3);
    await expect(page.locator('tbody th')).toHaveText(['2', '2', '4']);
    await expect(page.locator('tbody tr td:first-of-type')).toHaveText(rows.slice(1).map(row => row.name));
    await expect(page.locator('main time')).toHaveAttribute('datetime', new Date(now * 1000).toISOString().replace('.000Z', 'Z'));
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);

    const selected = `/${lang}/rankings/magic?world=${world.id}&vocation=1`;
    await probe(page, `${selected}&page=21`, 400, { lang, message: t['rankings-bad-query'] });
    await probe(page, `/${lang}/rankings/magic?world=${missingId}`, 404, { lang, message: t['rankings-not-found'] });
    await probe(page, `/${lang}/rankings/unknown`, 404, { lang, message: t['rankings-not-found'] });
    await db.query("UPDATE public_snapshots SET payload=jsonb_set(payload,'{computed_at}',to_jsonb('1'::text)) WHERE family='highscores' AND subject=$1", [subject]);
    await visit(page, selected, 503, { lang, message: t['rankings-unavailable'] });
    await expect(page.getByRole('table')).toHaveCount(0);
    await expect(form.getByLabel(t['rankings-vocation'])).toHaveValue('1');
  } finally {
    try {
      await db.query("DELETE FROM public_snapshots WHERE NOT is_watermark AND subject=ANY($1)", [[subject, ...rows.map(row => row.character_id)]]);
      if (watermarksBefore) {
        await db.query("DELETE FROM public_snapshots WHERE family IN ('highscores','character_profile') AND is_watermark AND subject=''");
        for (const w of watermarksBefore) {
          await db.query(
            'INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload,previous_snapshot) VALUES($1,$2,true,$3,$4,$5,$6,$7)',
            [w.family, w.subject, w.epoch, w.version, w.watermark, JSON.stringify(w.payload), w.previous_snapshot],
          );
        }
      }
      if (publicEpochBefore !== undefined) {
        await db.query('UPDATE public_projection_state SET highest_epoch=$1 WHERE id=1', [publicEpochBefore]);
      }
      if (feedBefore) {
        const { highest_epoch, watermark_epoch, source_authority, complete_through, observed_at } = feedBefore;
        await db.query(
          'UPDATE account_character_feed SET highest_epoch=$1,watermark_epoch=$2,source_authority=$3,complete_through=$4,observed_at=$5 WHERE id=1',
          [highest_epoch, watermark_epoch, source_authority, complete_through, observed_at],
        );
      }
      await db.query('DELETE FROM account_characters WHERE account_id=$1', [account]);
      await db.query('DELETE FROM account_character_snapshots WHERE account_id=$1', [account]);
      await db.query('DELETE FROM accounts WHERE id=$1', [account]);
      await db.query('DELETE FROM world_status WHERE id=$1', [world.id]);
    } finally {
      await db.end();
    }
  }
});
