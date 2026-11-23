import { seededAccount, seededPlayer } from '../outbox-helpers.mjs';
import { enqueue, expect, query, test, waitForJob } from './worker.mjs';

test('statistics polling composes one sample and advances the source cursor once', async ({ jobs }) => {
  const [fixture] = await query(
    `WITH world AS (SELECT uuidv7()::text AS subject,extract(epoch FROM now())::bigint AS observed),
          epoch AS (UPDATE public_projection_state SET highest_epoch=COALESCE(highest_epoch,1)
                    WHERE id=1 RETURNING highest_epoch)
     INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload)
     SELECT 'world_online',subject,false,highest_epoch,1,observed,'{"characters":[]}'::jsonb FROM world,epoch
     RETURNING subject,watermark::text,epoch::text`,
  );
  await query(
    `INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload)
     VALUES ('world_online','worker-watermark',true,$1,1,$2,'{}')
     ON CONFLICT (family,subject,is_watermark) DO UPDATE SET watermark=EXCLUDED.watermark`,
    [fixture.epoch, fixture.watermark],
  );
  const first = await enqueue('stats.poll');
  const replay = await enqueue('stats.poll');
  const workers = [jobs.start(), jobs.start()];
  await Promise.all(workers.map(worker => worker.ready));
  await waitForJob(first, 'done', 1);
  await waitForJob(replay, 'done', 1);
  expect(await query(
    `SELECT epoch::text,version::text,watermark::text FROM stats_projection_cursors
     WHERE family='world_online' AND subject=$1`,
    [fixture.subject],
  )).toEqual([{ epoch: fixture.epoch, version: '1', watermark: fixture.watermark }]);
  expect(await query(
    `SELECT players_online, sampled_at=to_timestamp(($2::bigint/300)*300) AS source_bucket
     FROM stats_world_samples WHERE world=$1`,
    [fixture.subject, fixture.watermark],
  )).toEqual([{ players_online: 0, source_bucket: true }]);
});

test('termination stays pending when Game closure authority is absent', async ({ page, jobs }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seededPlayer(page, lang);
  const [pending] = await query(
    `INSERT INTO account_terminations(account_id,scheduled_for,created_at,updated_at)
     VALUES ($1,now()-interval '1 day',now()-interval '15 days',now()-interval '15 days')
     RETURNING id`,
    [account.id],
  );
  const url = `/${lang}/account/privacy/delete`;
  expect((await page.goto(url)).status()).toBe(200);
  const cancel = page.locator('form[action$="/cancel"]');
  await expect(cancel).toBeVisible();

  const id = await enqueue('identity.finalize-terminations');
  const worker = jobs.start();
  await worker.ready;
  await waitForJob(id, 'done', 1);
  expect(await query(
    `SELECT a.status,t.cancelled_at IS NULL AND t.finalized_at IS NULL AS pending,
            t.updated_at > t.created_at AS visited
     FROM account_terminations t JOIN accounts a ON a.id=t.account_id WHERE t.id=$1`,
    [pending.id],
  )).toEqual([{ status: 'active', pending: true, visited: true }]);
  expect(await query(
    'SELECT id FROM account_termination_events WHERE termination_id=$1 AND kind=$2',
    [pending.id, 'finalized'],
  )).toEqual([]);
  expect((await page.reload()).status()).toBe(200);
  await expect(cancel).toBeVisible();
  await query('UPDATE account_terminations SET cancelled_at=clock_timestamp() WHERE id=$1', [pending.id]);
});

test('journal lifecycle erasure consumes a closed-account handoff once', async ({ jobs }, info) => {
  const closed = await seededAccount(info.project.metadata.locale);
  const active = await seededAccount(info.project.metadata.locale);
  await query(
    `INSERT INTO pd_game_logins(account_id,login_event_id,projection_epoch,recovery_generation,source_revision,entered_at)
     SELECT id,uuidv7(),1,1,1,(extract(epoch FROM now())*1000)::bigint
     FROM accounts WHERE id=ANY($1::uuid[])`,
    [[closed.id, active.id]],
  );
  // Exercise the existing database lifecycle handoff, without pretending to
  // supply the Game receipts required by the closure HTTP workflow.
  await query("UPDATE accounts SET status='closed' WHERE id=$1", [closed.id]);
  const handoff = () => query('SELECT id FROM pd_erasure_jobs WHERE account_id=$1', [closed.id]);
  expect(await handoff()).toHaveLength(1);
  const id = await enqueue('cleanup.journal-payloads');
  const replay = await enqueue('cleanup.journal-payloads');
  const workers = [jobs.start(), jobs.start()];
  await Promise.all(workers.map(worker => worker.ready));
  await waitForJob(id, 'done', 1);
  await waitForJob(replay, 'done', 1);
  expect(await handoff()).toEqual([]);
  expect(await query(
    'SELECT account_id FROM pd_game_logins WHERE account_id=ANY($1::uuid[])', [[closed.id, active.id]],
  )).toEqual([{ account_id: active.id }]);
});
