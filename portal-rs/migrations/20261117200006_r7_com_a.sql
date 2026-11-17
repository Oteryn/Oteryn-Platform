-- Keep existing V2 page and statistics readers on the guild key.
ALTER TABLE public_snapshots DROP CONSTRAINT public_snapshots_family_check;
ALTER TABLE public_snapshots ADD CHECK (family IN
    ('highscores','character_profile','character_deaths','guild','guild_v1','house','world_online'));

UPDATE public_snapshots SET family='guild_v1'
WHERE family='guild' AND payload->>'contract_version' IS DISTINCT FROM '2';

-- A V1 write could have displaced a V2 snapshot into the replay history.
INSERT INTO public_snapshots (family, subject, is_watermark, epoch, version, watermark, payload)
SELECT 'guild', subject, false, (previous_snapshot->>'epoch')::numeric,
       (previous_snapshot->>'version')::numeric, (previous_snapshot->>'watermark')::bigint,
       previous_snapshot->'payload'
FROM public_snapshots
WHERE family='guild_v1' AND NOT is_watermark
  AND previous_snapshot->'payload'->>'contract_version'='2';

UPDATE public_snapshots SET previous_snapshot=NULL
WHERE family='guild_v1' AND previous_snapshot->'payload'->>'contract_version'='2';

-- Historical conflicts carry no contract version. Preserve both fences.
INSERT INTO public_projection_conflicts (family, subject, epoch, revision)
SELECT 'guild_v1', subject, epoch, revision FROM public_projection_conflicts WHERE family='guild';
