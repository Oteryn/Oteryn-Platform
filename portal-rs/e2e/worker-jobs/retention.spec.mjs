import crypto from 'node:crypto';
import { seededAccount } from '../outbox-helpers.mjs';
import { enqueue, expect, query, status, test, waitForJob } from './worker.mjs';

// Seed retained storage directly. These rows represent already-admitted data
// with timestamps outside the real retention windows.
test('concurrent retention preserves fresh rows and applies historical changes once', async ({ jobs }, info) => {
  const account = await seededAccount(info.project.metadata.locale);
  const tag = crypto.randomUUID();
  const cases = [
    {
      kind: 'cleanup.rate-limits',
      seed: `INSERT INTO web_rate_limits(limiter_name,key_hash,window_start,count)
             SELECT $1,sha256(convert_to(mark,'UTF8')),clock_timestamp()-age,1
             FROM (VALUES ('expired',interval '61 minutes'),('fresh',interval '59 minutes')) AS t(mark,age)`,
      read: `SELECT encode(key_hash,'hex') AS id FROM web_rate_limits
             WHERE limiter_name=$1 ORDER BY window_start`,
      args: [tag],
    },
    {
      kind: 'cleanup.game-ticket-budgets',
      seed: `INSERT INTO game_ticket_budgets(credential_hash,window_start,requests)
             SELECT sha256(convert_to($1 || mark,'UTF8')),clock_timestamp()-age,1
             FROM (VALUES ('expired',interval '11 minutes'),('fresh',interval '9 minutes')) AS t(mark,age)`,
      read: `SELECT encode(credential_hash,'hex') AS id FROM game_ticket_budgets
             WHERE credential_hash IN (sha256(convert_to($1 || 'expired','UTF8')),sha256(convert_to($1 || 'fresh','UTF8')))
             ORDER BY window_start`,
      args: [tag],
    },
    {
      kind: 'cleanup.passkeys',
      seed: `INSERT INTO passkey_ceremonies(token_hash,session_hash,account_id,purpose,state,expires_at)
             SELECT sha256(gen_random_uuid()::text::bytea),sha256(gen_random_uuid()::text::bytea),$1,'register','{}',now()+delta
             FROM (VALUES (interval '-1 minute'),(interval '1 hour')) AS t(delta)`,
      read: `SELECT encode(token_hash,'hex') AS id FROM passkey_ceremonies
             WHERE account_id=$1 ORDER BY expires_at`,
      args: [account.id],
    },
    {
      kind: 'cleanup.mfa',
      seed: `INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at)
             SELECT sha256(gen_random_uuid()::text::bytea),$1,uuidv7(),sha256(gen_random_uuid()::text::bytea),'trusted',now()+delta
             FROM (VALUES (interval '-1 minute'),(interval '1 hour')) AS t(delta)`,
      read: `SELECT encode(token_hash,'hex') AS id FROM account_mfa_tokens
             WHERE account_id=$1 ORDER BY expires_at`,
      args: [account.id],
    },
    {
      kind: 'cleanup.tokens',
      seed: `INSERT INTO account_email_tokens(token_hash,account_id,purpose,expires_at,created_at,updated_at)
             SELECT sha256(gen_random_uuid()::text::bytea),$1,'verify_email',now()+delta,now()-interval '1 day',now()-interval '1 day'
             FROM (VALUES (interval '-1 minute'),(interval '1 hour')) AS t(delta)`,
      read: `SELECT encode(token_hash,'hex') AS id FROM account_email_tokens
             WHERE account_id=$1 ORDER BY expires_at`,
      args: [account.id],
    },
    {
      kind: 'cleanup.game-logins',
      seed: `INSERT INTO pd_game_logins(account_id,login_event_id,projection_epoch,recovery_generation,source_revision,entered_at)
             SELECT $1,uuidv7(),1,1,rev,(extract(epoch FROM now()-age)*1000)::bigint
             FROM (VALUES (1,interval '31 days'),(2,interval '29 days')) AS t(rev,age)`,
      read: `SELECT login_event_id AS id FROM pd_game_logins
             WHERE account_id=$1 AND source_revision IN (1,2) ORDER BY entered_at`,
      args: [account.id],
    },
    {
      kind: 'cleanup.hunting-places',
      seed: `INSERT INTO pd_places_replay_fences(source_authority,session_id,expires_at)
             SELECT $1,uuidv7(),now()+delta
             FROM (VALUES (interval '-1 minute'),(interval '1 hour')) AS t(delta)`,
      read: `SELECT session_id AS id FROM pd_places_replay_fences
             WHERE source_authority=$1 ORDER BY expires_at`,
      args: [tag],
    },
    {
      kind: 'cleanup.world-kills',
      seed: `INSERT INTO stats_world_kills_v2(source_authority,world_id,day_start,epoch,revision,content_digest,source_observed_at,payload)
             SELECT $1,uuidv7(),(floor(extract(epoch FROM now()-age)/86400)*86400)::bigint,1,1,
                    sha256(gen_random_uuid()::text::bytea),extract(epoch FROM now())::bigint,'[]'
             FROM (VALUES (interval '402 days'),(interval '399 days')) AS t(age)`,
      read: `SELECT world_id AS id FROM stats_world_kills_v2
             WHERE source_authority=$1 AND NOT retired AND payload IS NOT NULL ORDER BY day_start`,
      args: [tag],
    },
    {
      kind: 'cleanup.deaths',
      seed: `INSERT INTO public_deaths_v2(subject,authority,epoch,revision,digest,deaths)
             SELECT $1,'fixture',COALESCE(highest_epoch,1),1,sha256('fixture'::bytea),
               jsonb_build_array(
                 jsonb_build_object('occurred_at',extract(epoch FROM now()-interval '31 days')::bigint),
                 jsonb_build_object('occurred_at',extract(epoch FROM now()-interval '29 days')::bigint))
             FROM public_projection_state WHERE id=1`,
      read: `SELECT death->>'occurred_at' AS id FROM public_deaths_v2
             CROSS JOIN LATERAL jsonb_array_elements(deaths) AS death
             WHERE subject=$1 ORDER BY (death->>'occurred_at')::bigint`,
      args: [tag],
    },
    {
      kind: 'community.today.retention',
      seed: `WITH worlds AS (
               INSERT INTO world_status(id,name)
               SELECT uuidv7(),'Worker fixture ' || n || ' ' || $1 FROM generate_series(1,2) AS n
               RETURNING id,name
             )
             INSERT INTO today_snapshots(authority,world_id,epoch,revision,observed,content,fingerprint,received_at)
             SELECT $1,id,1,1,extract(epoch FROM now())::bigint,'fixture',sha256('fixture'::bytea),
               now()-CASE WHEN name LIKE 'Worker fixture 1 %' THEN interval '31 days' ELSE interval '29 days' END
             FROM worlds`,
      read: `SELECT world_id AS id FROM today_snapshots
             WHERE authority=$1 AND content IS NOT NULL ORDER BY received_at`,
      args: [tag],
    },
    {
      kind: 'league.retention',
      seed: `INSERT INTO league_subjects(authority,family,world_id,subject_id,epoch,revision,recovery_generation,
                 wire,content_hash,record_hash,final_state,finalized_at)
             SELECT $1,'world_event_outcome',uuidv7(),uuidv7(),1,1,1,'fixture'::bytea,
               repeat('0',64),repeat('0',64),true,extract(epoch FROM now()-age)::bigint
             FROM (VALUES (interval '91 days'),(interval '89 days')) AS t(age)`,
      read: `SELECT subject_id AS id FROM league_subjects
             WHERE authority=$1 AND octet_length(wire)>0 ORDER BY finalized_at`,
      args: [tag],
    },
    {
      kind: 'stats.v2-retention',
      seed: `INSERT INTO stats_v2_subjects(source_authority,family,subject,epoch,revision,observed_at,content_digest,payload,expires_at)
             SELECT $1,'character_online_sessions',mark,1,1,2,sha256(gen_random_uuid()::text::bytea),'[]',
                    extract(epoch FROM now()+delta)::bigint
             FROM (VALUES ('expired',interval '-1 minute'),('fresh',interval '1 hour')) AS t(mark,delta)`,
      read: `SELECT subject AS id FROM stats_v2_subjects
             WHERE source_authority=$1 AND NOT erased AND payload IS NOT NULL ORDER BY expires_at`,
      args: [tag],
    },
    {
      kind: 'stats.death-retention',
      seed: `INSERT INTO stats_character_events(character_ref,day,kind,event_key,details)
             VALUES ($1,current_date-32,'death','expired','{}'),($1,current_date-29,'death','fresh','{}')`,
      read: 'SELECT event_key AS id FROM stats_character_events WHERE character_ref=$1 ORDER BY day',
      args: [tag],
    },
    {
      kind: 'identity.account-closure-audit',
      seed: `WITH termination AS (
               INSERT INTO account_terminations(account_id,scheduled_for,cancelled_at)
               VALUES ($1,now()+interval '2 days',now()) RETURNING id,account_id
             ), work AS (
               INSERT INTO plat3_closure_work(termination_id,account_id,generation)
               SELECT id,account_id,1 FROM termination RETURNING termination_id
             ), commands AS (
               INSERT INTO plat3_closure_commands(operation_id,termination_id,source_revision,command,intent,intent_sha256)
               SELECT uuidv7(),termination_id,nextval('plat3_closure_source_revision'),cmd,'fixture'::bytea,sha256('fixture'::bytea)
               FROM work CROSS JOIN (VALUES ('CloseAccount'),('ResolveAccountClosure')) AS t(cmd)
               RETURNING operation_id,command
             ), receipts AS (
               INSERT INTO plat3_closure_receipts(operation_id,receipt,signature,receipt_sha256,recovery_generation)
               SELECT operation_id,convert_to(jsonb_build_object('decided_at',
                      extract(epoch FROM now()-CASE WHEN command='CloseAccount' THEN interval '91 days'
                                                   ELSE interval '89 days' END))::text,'UTF8'),
                      decode(repeat('00',64),'hex'),sha256('fixture'::bytea),'1' FROM commands
             )
             INSERT INTO plat3_closure_audit_outbox(operation_id,payload)
             SELECT operation_id,'fixture'::bytea FROM commands`,
      read: `SELECT a.operation_id AS id FROM plat3_closure_audit_outbox a
             JOIN plat3_closure_commands c USING(operation_id)
             JOIN plat3_closure_work w USING(termination_id) WHERE w.account_id=$1
             ORDER BY c.command`,
      args: [account.id],
    },
  ];

  const survivors = new Map();
  for (const { kind, seed, read, args } of cases) {
    await query(seed, args);
    const rows = await query(read, args);
    expect(rows, kind).toHaveLength(2);
    const [expired, fresh] = rows;
    expect(expired.id, kind).not.toBe(fresh.id);
    survivors.set(kind, fresh);
  }
  await query(
    `INSERT INTO stats_world_samples(world,sampled_at,players_online)
     VALUES ($1,'2025-01-01 12:00+00',3),($1,'2025-01-01 12:05+00',7)`,
    [tag],
  );
  const [freshSample] = await query(
    `INSERT INTO stats_world_samples(world,sampled_at,players_online)
     VALUES ($1,clock_timestamp(),11) RETURNING sampled_at,players_online`, [tag],
  );
  const kinds = [...cases.map(testCase => testCase.kind), 'stats.world-sample-retention'];
  const dispatches = await Promise.all(kinds.flatMap(kind => [1, 2].map(async () => ({
    kind, id: await enqueue(kind),
  }))));
  // The poll handler also prunes copied deaths when enabled. Isolate the
  // dedicated retention handler from that separate production path.
  const workers = [jobs.start({ PORTAL_STATS_ENABLED: 'false' }), jobs.start({ PORTAL_STATS_ENABLED: 'false' })];
  await Promise.all(workers.map(worker => worker.ready));
  for (const { kind, id } of dispatches) {
    if (kind === 'league.retention') {
      // Distinct one-second slots can contend for the league advisory lock.
      // Require successful acknowledgement within the same durable retry budget.
      await expect.poll(async () => (await status(id))?.state,
        { timeout: 35_000 }).toBe('done');
      const row = await status(id);
      expect(row).toMatchObject({ released: true, failed: false });
      expect(row.attempts).toBeGreaterThanOrEqual(1);
      expect(row.attempts).toBeLessThanOrEqual(5);
    } else {
      await waitForJob(id, 'done', 1);
    }
  }

  for (const { kind, read, args } of cases) {
    expect(await query(read, args), kind).toEqual([survivors.get(kind)]);
  }
  expect(await query(
    `SELECT retired,payload IS NULL AS erased FROM stats_world_kills_v2
     WHERE source_authority=$1 ORDER BY day_start`, [tag],
  )).toEqual([{ retired: true, erased: true }, { retired: false, erased: false }]);
  expect(await query(
    `SELECT erased,payload IS NULL AS empty,observed_at=1 AND expires_at=1 AS fenced
     FROM stats_v2_subjects WHERE source_authority=$1 ORDER BY subject`, [tag],
  )).toEqual([{ erased: true, empty: true, fenced: true }, { erased: false, empty: false, fenced: false }]);
  expect(await query('SELECT sampled_at,players_online FROM stats_world_samples WHERE world=$1', [tag]))
    .toEqual([freshSample]);
  expect(await query(
    `SELECT samples::int,players_sum::int,players_min,players_max
     FROM stats_world_daily WHERE world=$1 AND day='2025-01-01'`, [tag],
  )).toEqual([{ samples: 2, players_sum: 10, players_min: 3, players_max: 7 }]);
  expect(await query('SELECT event_key FROM stats_character_events WHERE character_ref=$1', [tag]))
    .toEqual([{ event_key: 'fresh' }]);
  expect(await query(
    `SELECT c.command,a.operation_id IS NOT NULL AS retained FROM plat3_closure_receipts r
     JOIN plat3_closure_commands c USING(operation_id) JOIN plat3_closure_work w USING(termination_id)
     LEFT JOIN plat3_closure_audit_outbox a USING(operation_id) WHERE w.account_id=$1 ORDER BY c.command`, [account.id],
  )).toEqual([
    { command: 'CloseAccount', retained: false },
    { command: 'ResolveAccountClosure', retained: true },
  ]);
});
