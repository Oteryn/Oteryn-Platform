-- Run only after migrations and account seeds on the disposable staging DB.
-- All identities and values are synthetic. Dates are relative to the deploy.
BEGIN;
SET LOCAL TIME ZONE 'UTC';

DO $$
BEGIN
    IF current_database() <> 'portal_e2e_staging' THEN
        RAISE EXCEPTION 'staging sample data requires the disposable staging database';
    END IF;
END $$;

CREATE TEMP TABLE sample_worlds ON COMMIT DROP AS
SELECT n AS world_number, ('019a0000-0000-7001-8000-' || lpad(n::text, 12, '0'))::uuid AS id,
       name, pvp, location,
       ('019a0000-0000-7003-8000-' || lpad(n::text, 12, '0'))::uuid AS guild_id,
       guild_name
FROM (VALUES
    (1, 'Ashen Coast', 'open', 'Europe', 'Lantern Watch'),
    (2, 'Silverfen', 'optional', 'North America', 'Silverfen Wardens'),
    (3, 'Cinderpeak', 'hardcore', 'Europe', 'Ember Company')
) AS w(n, name, pvp, location, guild_name);

INSERT INTO world_status (id, name, pvp_type, location)
SELECT id, name, pvp, location FROM sample_worlds;

CREATE TEMP TABLE sample_characters ON COMMIT DROP AS
SELECT n, ('019a0000-0000-7002-8000-' || lpad(n::text, 12, '0'))::uuid AS id,
       a.id AS owner, w.id AS world, w.guild_id, w.guild_name,
       CASE WHEN n IN (7, 29, 30) THEN 1 ELSE 3 END AS guild_rank,
       first_names[(n - 1) % 10 + 1] || ' ' || surnames[(n - 1) / 10 + 1] AS name,
       1 + (n - 1) % 5 AS vocation, 55 + n * 4 AS level,
       (55 + n * 4)::bigint ^ 3 * 14 AS experience
FROM generate_series(1, 50) AS n
CROSS JOIN (VALUES (
    ARRAY['Aster', 'Bryn', 'Corin', 'Dara', 'Edda', 'Fenn', 'Galen', 'Iona', 'Kestrel', 'Liora'],
    ARRAY['Ashbrook', 'Mossvale', 'Reedwater', 'Copperleaf', 'Wintermere']
)) AS names(first_names, surnames)
JOIN sample_worlds w ON w.world_number = CASE WHEN n <= 12 THEN 1 ELSE (n - 13) % 3 + 1 END
JOIN accounts a ON a.email = CASE (n - 1) / 2
    WHEN 0 THEN 'player-basic' WHEN 1 THEN 'player-mfa' WHEN 2 THEN 'player-premium'
    WHEN 3 THEN 'player-guild-leader' WHEN 4 THEN 'player-new' WHEN 5 THEN 'player-restricted'
    ELSE 'founder' END || '@staging.invalid';

DO $$
BEGIN
    IF (SELECT count(*) FROM sample_characters) <> 50 THEN
        RAISE EXCEPTION 'staging accounts were not seeded';
    END IF;
END $$;

INSERT INTO account_character_snapshots
    (account_id, source_authority, projection_epoch, projection_revision, source_observed_at, content_digest)
SELECT owner, 'staging-sample', '1', '1', floor(extract(epoch FROM now())),
       sha256(convert_to(string_agg(id::text, ',' ORDER BY id), 'UTF8'))
FROM sample_characters GROUP BY owner;

INSERT INTO account_characters (account_id, character_id, world_id, name, availability)
SELECT owner, id, world, name, 'AVAILABLE' FROM sample_characters;

UPDATE accounts SET public_status_visible = true
WHERE email_verified_at IS NOT NULL AND id IN (SELECT owner FROM sample_characters);

INSERT INTO character_profiles
    (account_id, character_id, hidden, show_guild, show_skills, show_deaths, show_status, show_house, show_online_time)
SELECT owner, id, false, true, true, true, true, true, true FROM sample_characters;

INSERT INTO sanctions (id, account_id, category, status, public_reason, effective_at, expires_at, created_by)
SELECT '019a0000-0000-7006-8000-000000000001', player.id, 'restriction', 'active',
       'Sample restriction for testing the recruitment board.',
       now() - interval '1 day', now() + interval '30 days', founder.id
FROM accounts player CROSS JOIN accounts founder
WHERE player.email = 'player-restricted@staging.invalid' AND founder.email = 'founder@staging.invalid';

INSERT INTO premium_authority (account_id, authority_revision, last_issued_at)
SELECT id, 1, date_trunc('second', now()) FROM accounts WHERE email = 'player-premium@staging.invalid';

INSERT INTO premium_ledger
    (request_id, account_id, actor_id, event_type, duration_days, reason_sha256, entitlement_id,
     lifecycle_revision, delta_seconds, effective_from, effective_until)
SELECT '019a0000-0000-7006-8000-000000000002', player.account_id, founder.id, 'GRANT', 30,
       sha256('Staging purchase-free premium grant'), '019a0000-0000-7006-8000-000000000003',
       1, 2592000, date_trunc('second', now()), date_trunc('second', now()) + interval '30 days'
FROM premium_authority player CROSS JOIN accounts founder WHERE founder.email = 'founder@staging.invalid';

INSERT INTO premium_audit (request_id, actor_id, account_id, event_name)
SELECT request_id, actor_id, account_id, 'products.premium_granted' FROM premium_ledger;

UPDATE public_projection_state SET highest_epoch = 1 WHERE id = 1;

INSERT INTO public_snapshots (family, subject, epoch, version, watermark, payload)
SELECT 'character_profile', id::text, 1, 1, floor(extract(epoch FROM now())),
       jsonb_build_object('character_id', id, 'profile', jsonb_build_array(jsonb_build_object(
           'world_id', world, 'name', name, 'vocation', vocation,
           'level', level::text, 'experience', experience::bigint::text,
           'created_at', floor(extract(epoch FROM now() - make_interval(days => 100 + n)))::text,
           'guild_visible', true,
           'guild', jsonb_build_array(jsonb_build_object('guild_id', guild_id, 'rank_level', guild_rank)))))
FROM sample_characters;

INSERT INTO public_snapshots (family, subject, epoch, version, watermark, payload)
SELECT 'world_online', world::text, 1, 1, floor(extract(epoch FROM now())),
       jsonb_build_object('source_authority', 'staging-sample', 'world_id', world, 'characters', jsonb_agg(jsonb_build_object(
           'name', name, 'level', level::text, 'vocation', vocation) ORDER BY name))
FROM sample_characters WHERE n % 4 <> 0
GROUP BY world;

-- Include each supported category and vocation filter with stable rankings.
WITH scores AS (
    SELECT c.*, category, filter,
           CASE category WHEN 1 THEN experience::bigint
                         WHEN 2 THEN 12 + n / 3
                         ELSE 25 + n + category END AS score
    FROM sample_characters c
    CROSS JOIN generate_series(1, 9) AS category
    CROSS JOIN LATERAL (VALUES (0), (c.vocation)) AS f(filter)
), ranked AS (
    SELECT *, row_number() OVER (PARTITION BY world, category, filter ORDER BY score DESC, id) AS rank
    FROM scores
)
INSERT INTO public_snapshots (family, subject, epoch, version, watermark, payload)
SELECT 'highscores', concat_ws(':', world, category, filter), 1, 1, floor(extract(epoch FROM now())),
       jsonb_build_object('world_id', world, 'category', category, 'filter', filter,
           'rows', jsonb_agg(jsonb_build_object('rank', rank::text, 'character_id', id, 'name', name,
                                               'vocation', vocation, 'level', level::text, 'value', score::text)
                             ORDER BY rank))
FROM ranked GROUP BY world, category, filter;

INSERT INTO public_snapshots (family, subject, epoch, version, watermark, payload)
SELECT 'guild', guild_id::text, 1, 1, floor(extract(epoch FROM now())),
       jsonb_build_object('contract_version', 2, 'source_authority', 'staging-sample', 'guild_id', guild_id,
           'guild', jsonb_build_array(jsonb_build_object(
               'world_id', world, 'name', guild_name, 'state', 'ACTIVE',
               'founded_at', floor(extract(epoch FROM now() - interval '90 days'))::text,
               'ranks', '[{"level":1,"name":"Leader"},{"level":2,"name":"Officer"},{"level":3,"name":"Member"}]'::jsonb,
               'members', jsonb_agg(jsonb_build_object(
                   'character_id', id, 'name', name, 'rank_level', guild_rank,
                   'joined_at', floor(extract(epoch FROM now() - interval '60 days'))::text)
                   ORDER BY guild_rank, id))))
FROM sample_characters GROUP BY guild_id, world, guild_name;

INSERT INTO public_snapshots (family, subject, is_watermark, epoch, version, watermark, payload)
SELECT family, subject, true, 1, 1, 0, '{"contract_version":2,"source_authority":"staging-sample"}'
FROM (VALUES ('world_online', ''), ('character_profile', ''), ('guild', 'v2')) AS w(family, subject);

CREATE TEMP TABLE sample_deaths ON COMMIT DROP AS
SELECT id AS character, ('019a0000-0000-7004-8000-' || lpad(n::text, 12, '0'))::uuid AS occurrence,
       now() - make_interval(hours => n) AS occurred, level,
       jsonb_build_object('kind', 'creature', 'creature_key', 'mire-wolf', 'name', 'Mire Wolf') AS killer
FROM sample_characters WHERE n % 4 = 1;

INSERT INTO public_deaths_v2 (subject, authority, epoch, revision, digest, deaths)
SELECT c.id::text, 'staging-sample', 1, 1, sha256(convert_to(c.id::text, 'UTF8')),
       CASE WHEN d.occurrence IS NULL THEN '[]'::jsonb ELSE jsonb_build_array(jsonb_build_object(
           'death_occurrence_id', d.occurrence, 'occurred_at', floor(extract(epoch FROM d.occurred))::text,
           'level', d.level::text, 'pvp', false, 'killers', jsonb_build_array(d.killer))) END
FROM sample_characters c LEFT JOIN sample_deaths d ON d.character = c.id;
INSERT INTO public_deaths_v2_watermark VALUES (1, 'staging-sample', 1, 0);

WITH houses AS (
    SELECT w.id AS world, 'oteryn:content.house.' || h.key AS key, h.name, h.kind,
           lower(replace(w.name, ' ', '_')) AS town, h.status, h.size, h.beds, h.rent
    FROM sample_worlds w CROSS JOIN (VALUES
        ('harbor_cottage', 'Harbor Cottage', 'private_house', 'VACANT', '54', '2', '1200'),
        ('east_gate', 'East Gate Residence', 'private_house', 'AUCTION', '88', '4', '2200'),
        ('council_hall', 'Council Hall', 'guildhall', 'VACANT', '210', '12', '8500')
    ) AS h(key, name, kind, status, size, beds, rent)
)
INSERT INTO public_houses_v2 (subject, source_authority, epoch, revision, observed, payload)
SELECT world::text || ':' || key, 'staging-sample', 1, 1, floor(extract(epoch FROM now())),
       jsonb_build_object('contract_version', 2, 'operation', 'PublishHouseV2',
           'source_authority', 'staging-sample', 'projection_epoch', '1', 'projection_revision', '1',
           'source_observed_at', floor(extract(epoch FROM now()))::text, 'world_id', world,
           'house_key', key, 'house', jsonb_build_array(
               jsonb_build_object('name', name, 'kind', kind, 'town', town, 'status', status,
                   'size_sqm', size, 'beds', beds, 'rent_gold', rent) ||
               CASE WHEN status = 'AUCTION' THEN jsonb_build_object(
                   'current_bid', '1800',
                   'auction_ends_at', floor(extract(epoch FROM now() + interval '3 days'))::text)
               ELSE '{}'::jsonb END))
FROM houses;
INSERT INTO public_houses_v2_watermark VALUES (1, 'staging-sample', 1, 0);

INSERT INTO pd_party_posts
    (id, account_id, character_id, character_name, world_id, purpose, min_level, max_level,
     profession, note, starts_at, expires_at)
SELECT ('019a0000-0000-7005-8000-' || lpad(n::text, 12, '0'))::uuid,
       owner, id, name, world, purpose, level - 15, level + 40, profession, note,
       now() + interval '1 hour', now() + interval '3 days'
FROM sample_characters JOIN (VALUES
    (1, 'hunt', 'any', 'Looking for company in the Saltwind Caverns. Meet at the depot.'),
    (3, 'boss', 'druid', 'Need a healer for the Glass Warden. Supplies are ready.'),
    (7, 'quest', 'any', 'Lantern Watch is running the East Gate quest chain. First-timers welcome.'),
    (17, 'hunt', 'paladin', 'Two spots open for an evening run through the marsh.')
) AS post(n, purpose, profession, note) USING (n);

DO $$
DECLARE
    month_start date;
BEGIN
    FOR month_start IN
        SELECT generate_series(date_trunc('month', current_date - 45),
                               date_trunc('month', current_date), interval '1 month')::date
    LOOP
        EXECUTE format('CREATE TABLE IF NOT EXISTS %I PARTITION OF stats_character_daily FOR VALUES FROM (%L) TO (%L)',
                       'stats_character_daily_' || to_char(month_start, 'YYYYMM'),
                       month_start, (month_start + interval '1 month')::date);
    END LOOP;
END $$;

INSERT INTO stats_character_daily
    (character_ref, day, level, experience, skills, minutes_online, name, world, guild, observed_at)
SELECT id, current_date - age, level - age / 4, experience::bigint - age * (3200 + n * 160),
       jsonb_build_object('magic_level', 25 + n / 2, 'sword_level', 60 + n - age / 5,
                          'distance_level', 50 + n / 2),
       24 + (n * 7 + age * 11) % 190, name, world, guild_id,
       floor(extract(epoch FROM now())) - age * 86400
FROM sample_characters CROSS JOIN generate_series(0, 45) AS age;

INSERT INTO stats_character_events (character_ref, day, kind, event_key, details)
SELECT character, occurred::date, 'death', occurrence::text,
       jsonb_build_object('death_occurrence_id', occurrence, 'occurred_at', floor(extract(epoch FROM occurred))::text,
                          'level', level)
FROM sample_deaths
UNION ALL
SELECT id, current_date - 4, 'level_up', 'sample-level-up', jsonb_build_object('level', level - 1)
FROM sample_characters;

INSERT INTO stats_world_daily (world, day, samples, players_sum, players_min, players_max)
SELECT id, current_date - age, 24, 24 * (7 + world_number + age % 4), 2 + world_number, 12 + world_number + age % 5
FROM sample_worlds CROSS JOIN generate_series(0, 45) AS age;

WITH days AS (
    SELECT id, world_number, age,
           extract(epoch FROM (current_date - age)::timestamp)::bigint AS day_start
    FROM sample_worlds CROSS JOIN generate_series(1, 45) AS age
), totals AS (
    SELECT *, jsonb_build_array(jsonb_build_object(
        'day_end', (day_start + 86400)::text, 'monsters_killed', (850 + world_number * 290 + age % 7 * 45)::text,
        'bosses_killed', (4 + world_number + age % 4)::text, 'complete', true)) AS payload
    FROM days
)
INSERT INTO stats_world_kills_v2
    (source_authority, world_id, day_start, epoch, revision, content_digest, source_observed_at, payload)
SELECT 'staging-sample', id, day_start, 1, 1,
       sha256(convert_to(payload::text, 'UTF8')), floor(extract(epoch FROM now())), payload
FROM totals;

-- T14 catalogue consumed by the wiki, calculators, atlas, daily boosts and collection.
INSERT INTO world_versions (id, source_commit, status, document_count)
VALUES ('019a0000-0000-7007-8000-000000000001', repeat('a', 40), 'ready', 12);
UPDATE world_active_version SET version_id = '019a0000-0000-7007-8000-000000000001';

WITH creatures(key, name, health, experience) AS (VALUES
    ('mire-wolf', 'Mire Wolf', 460, 310),
    ('glass-warden', 'Glass Warden', 6800, 4100),
    ('saltwind-crab', 'Saltwind Crab', 290, 170)
), entities AS (
    SELECT 'Creature' AS kind, key, jsonb_build_object(
        'definition', jsonb_build_object('identity', jsonb_build_object('family', 'Creature', 'key', key),
                                         'client_projection', 'ClientSafe'),
        'authoring', jsonb_build_object('profile', jsonb_build_object('details', jsonb_build_object(
            'display_name', name, 'health', health, 'experience', experience,
            'bestiary', jsonb_build_object('race_key', key))))) AS data
    FROM creatures
    UNION ALL
    SELECT * FROM (VALUES
        ('Item', 'lantern-shield', '{"names":{"en":"Lantern Shield","pl":"Tarcza latarnika"},"defense":22}'::jsonb),
        ('NPC', 'harbor-keeper', '{"names":{"en":"Harbor Keeper","pl":"Strażnik portu"}}'),
        ('Raid', 'saltwind-siege', '{}'),
        ('npc-placements', 'harbor-depot', '{"declaration":{"npc":"harbor-keeper","cell":{"x":124,"y":136,"floor":7}}}'),
        ('Spawn.Source', 'saltwind-pack', '{"declaration":{"points":[{"cell":{"x":193,"y":185,"floor":7},"creature":"mire-wolf","respawn_ms":90000},{"cell":{"x":211,"y":163,"floor":7},"creature":"saltwind-crab","respawn_ms":45000}]}}'),
        ('HuntPlace', 'saltwind-caverns', '{"title":{"en":"Saltwind Caverns","pl":"Groty Słonego Wiatru"},"purpose":"hunt","level":80,"party_size":2,"summary":{"en":"Follow the coast below the depot. Bring supplies for the lower tunnels.","pl":"Idź brzegiem poniżej depozytu. Zabierz zapasy do dolnych tuneli."}}'),
        ('HuntPlace', 'glass-warden-vault', '{"title":{"en":"Glass Warden Vault","pl":"Skarbiec Szklanego Strażnika"},"purpose":"boss","level":120,"party_size":4,"summary":{"en":"Gather at the eastern bridge before entering the vault.","pl":"Zbierz drużynę przy wschodnim moście przed wejściem do skarbca."}}'),
        ('StagingPolicy', 'training', '{"schema_version":1,"enabled":true,"exercise_gains":[600,7,7,7,7,7,7,0],"percent_scale":100,"dummies":{"oteryn:item.tibia.i28558":100,"oteryn:item.tibia.i28559":110}}')
    ) AS other(kind, key, data)
    UNION ALL
    SELECT 'StagingPolicy', 'experience', jsonb_build_object(
        'schema', 'OTERYN_GAME_CHARACTER_EXPERIENCE_TABLE/v1',
        'levels', jsonb_agg(jsonb_build_object('level', level, 'minimum_experience', ((level - 1) * 100)::text)
                           ORDER BY level),
        'terminal_exclusive_experience', '200000')
    FROM generate_series(1, 2000) AS level
)
INSERT INTO world_entities (version_id, kind, key, data, publishable)
SELECT '019a0000-0000-7007-8000-000000000001', kind, key, data, true FROM entities;

INSERT INTO world_visibility (kind, key, allowed)
SELECT kind, key, true FROM world_entities WHERE version_id = '019a0000-0000-7007-8000-000000000001';

INSERT INTO world_admin_names (kind, key, en, pl) VALUES
    ('Creature', 'mire-wolf', 'Mire Wolf', 'Wilk bagienny'),
    ('Creature', 'glass-warden', 'Glass Warden', 'Szklany Strażnik'),
    ('Creature', 'saltwind-crab', 'Saltwind Crab', 'Krab z Solnego Brzegu'),
    ('Raid', 'saltwind-siege', 'Saltwind Siege', 'Oblężenie Solnego Brzegu');

-- League terminal results are explicitly from the sample authority.
-- No Game signature or audit evidence is fabricated.
WITH results AS (
    SELECT id AS world, guild_id, guild_name,
           ('019a0000-0000-7008-8000-' || lpad(world_number::text, 12, '0'))::uuid AS season,
           ('019a0000-0000-7009-8000-' || lpad(world_number::text, 12, '0'))::uuid AS challenge,
           floor(extract(epoch FROM now()))::bigint AS stamp, (2500 - world_number * 100)::text AS score
    FROM sample_worlds
), bodies AS (
    SELECT *, jsonb_build_object('ruleset_revision', '1', 'starts_at', (stamp - 30*86400)::text,
        'ends_at', (stamp - 86400)::text, 'freeze_at', (stamp - 86400)::text, 'state', 'FROZEN',
        'challenges', jsonb_build_array(jsonb_build_object('challenge_id', challenge,
            'source_key', 'saltwind-siege', 'scoring_revision', '1', 'max_score', '3000')),
        'standings', jsonb_build_array(jsonb_build_object('guild_id', guild_id, 'guild_name', guild_name,
            'scores', jsonb_build_array(jsonb_build_object('challenge_id', challenge, 'score', score)),
            'total_score', score, 'rank', '1')),
        'computed_at', (stamp - 86400)::text, 'source_complete_through', (stamp - 86400)::text,
        'correction_of', jsonb_build_array()) AS record
    FROM results
)
INSERT INTO league_subjects
    (authority, family, world_id, subject_id, epoch, revision, recovery_generation, wire,
     content_hash, record_hash, final_state, finalized_at)
SELECT 'staging-sample', 'guild_season', world, season, 1, 1, 1,
       convert_to(jsonb_build_object('contract_version', 1, 'operation', 'PublishGuildSeasonV1',
           'source_authority', 'staging-sample', 'world_id', world, 'season_id', season,
           'projection_epoch', '1', 'projection_revision', '1', 'source_observed_at', stamp::text,
           'season', jsonb_build_array(record))::text, 'UTF8'),
       encode(sha256(convert_to(record::text, 'UTF8')), 'hex'),
       encode(sha256(convert_to(record::text, 'UTF8')), 'hex'), true, stamp - 86400
FROM bodies;

INSERT INTO collection_cards (id, definition, sprite)
SELECT id, jsonb_build_object('id', id, 'world_version', '019a0000-0000-7007-8000-000000000001',
           'kind', 'Creature', 'key', key, 'race_key', key, 'race_revision', '1',
           'final_kill_threshold', kills, 'name', jsonb_build_object('en', en, 'pl', pl),
           'appearance_id', appearance),
       decode(sprite, 'base64')
FROM (VALUES
    ('019a0000-0000-7010-8000-000000000001'::uuid, 'mire-wolf', 250, 'Mire Wolf', 'Wilk bagienny', 1, 'UklGRmgAAABXRUJQVlA4TFwAAAAvH8AHEBcgEEjyxxloLgFB0WzvCQSSIAYeff4D/hWwjWS11Q0hVShWlZRC9A1typsCIvo/AdxVatZwrYUmIFlEAF/UohSseLQWilcyXyEQDQoqXpMGngFrZaqt5Q=='),
    ('019a0000-0000-7010-8000-000000000002'::uuid, 'saltwind-crab', 100, 'Saltwind Crab', 'Krab z Solnego Brzegu', 2, 'UklGRmgAAABXRUJQVlA4TFwAAAAvH8AHEBcgFkzm/2QYl0AgyZ9rrwUCSegjjjT/Ad8aGEWyGmuKY5RQqEQKlRN5Nv++CIjo/wTwu77nt7TAm5ICz9FLiyAkaRq1EeRmnGFjB+wFkKPlQFlRQLL3eg==')
) AS cards(id, key, kills, en, pl, appearance, sprite);

INSERT INTO today_snapshots (authority, world_id, epoch, revision, observed, fingerprint)
SELECT 'staging-sample', id, 1, 1, 0, sha256(convert_to(id::text, 'UTF8')) FROM sample_worlds;
INSERT INTO today_watermarks VALUES ('staging-sample', 1, 0, 0);

-- Republish just the synthetic freshness cuts while the stack is running.
-- Player edits (profile preferences, party posts, account state) are left alone.
CREATE FUNCTION staging_refresh_sample_feed() RETURNS void LANGUAGE plpgsql AS $$
DECLARE
    stamp bigint := floor(extract(epoch FROM clock_timestamp()));
    day_start bigint := stamp / 86400 * 86400;
BEGIN
    UPDATE account_character_feed
       SET highest_epoch = '1', watermark_epoch = '1', source_authority = 'staging-sample',
           complete_through = stamp, observed_at = stamp, updated_at = now()
     WHERE id = 1 AND (highest_epoch IS NULL OR (highest_epoch = '1' AND source_authority = 'staging-sample'));
    UPDATE public_snapshots SET watermark = stamp
     WHERE epoch = 1 AND payload->>'source_authority' = 'staging-sample'
       AND (is_watermark OR family = 'world_online');
    UPDATE public_deaths_v2_watermark SET complete_through = stamp WHERE authority = 'staging-sample' AND epoch = 1;
    UPDATE public_houses_v2_watermark SET complete_through = stamp WHERE source_authority = 'staging-sample' AND epoch = 1;
    UPDATE today_snapshots SET observed = stamp, received_at = now(),
           content = jsonb_build_object('world_id', world_id, 'daily', jsonb_build_array(jsonb_build_object(
               'reset_epoch', (day_start / 86400)::text,
               'effective_from', day_start::text, 'effective_until', (day_start + 86400)::text,
               'creature_key', 'mire-wolf', 'boss_key', 'glass-warden',
               'history_complete_from', (day_start - 86400)::text,
               'history', jsonb_build_array(jsonb_build_object(
                   'reset_epoch', (day_start / 86400 - 1)::text,
                   'effective_from', (day_start - 86400)::text, 'effective_until', day_start::text,
                   'creature_key', 'saltwind-crab', 'boss_key', 'glass-warden')),
               'raids', jsonb_build_array(jsonb_build_object('raid_key', 'saltwind-siege',
                   'started_at', day_start::text, 'scheduled_at', day_start::text, 'state', 'done'))
           )))::text
     WHERE authority = 'staging-sample' AND epoch = 1 AND NOT conflicted;
    UPDATE today_watermarks SET complete_through = stamp, observed = stamp
     WHERE authority = 'staging-sample' AND epoch = 1;
END $$;

SELECT staging_refresh_sample_feed();
COMMIT;
