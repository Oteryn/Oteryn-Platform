-- Install the wider constraint before removing the narrower one.
ALTER TABLE public_snapshots ADD CONSTRAINT public_snapshots_family_widened_check CHECK (family IN
    ('highscores','character_profile','character_deaths','guild','guild_v1','house','world_online'));
ALTER TABLE public_snapshots DROP CONSTRAINT public_snapshots_family_check;
ALTER TABLE public_snapshots RENAME CONSTRAINT public_snapshots_family_widened_check TO public_snapshots_family_check;

-- V1 stores a null watermark payload and only guild_id and guild in snapshots.
-- Leave any row outside those shapes in its original namespace.
UPDATE public_snapshots SET family='guild_v1'
WHERE family='guild' AND (
    (is_watermark AND subject='' AND payload='null'::jsonb)
    OR (NOT is_watermark
        AND jsonb_typeof(payload->'guild')='array'
        AND payload=jsonb_build_object('guild_id', subject, 'guild', payload->'guild'))
);

-- Restore identifiable V2 snapshots displaced by a V1 write. Keep the replay
-- history on the V1 row and never overwrite a current V2 row.
INSERT INTO public_snapshots (family, subject, is_watermark, epoch, version, watermark, payload)
SELECT 'guild', subject, false, (previous_snapshot->>'epoch')::numeric,
       (previous_snapshot->>'version')::numeric, (previous_snapshot->>'watermark')::bigint,
       previous_snapshot->'payload'
FROM public_snapshots
WHERE family='guild_v1' AND NOT is_watermark
  AND previous_snapshot->'payload'->'contract_version'='2'::jsonb
  AND previous_snapshot->'payload'->>'guild_id'=subject
  AND jsonb_typeof(previous_snapshot->'payload'->'guild')='array'
  AND previous_snapshot->>'epoch' ~ '^[1-9][0-9]{0,19}$'
  AND previous_snapshot->>'version' ~ '^[1-9][0-9]{0,19}$'
  AND previous_snapshot->>'watermark' ~ '^[0-9]+$'
  AND pg_input_is_valid(previous_snapshot->>'watermark', 'bigint')
ON CONFLICT (family, subject, is_watermark) DO NOTHING;

-- Historical conflicts carry no contract version. Preserve both fences.
INSERT INTO public_projection_conflicts (family, subject, epoch, revision)
SELECT 'guild_v1', subject, epoch, revision FROM public_projection_conflicts WHERE family='guild'
ON CONFLICT (family, subject) DO NOTHING;
