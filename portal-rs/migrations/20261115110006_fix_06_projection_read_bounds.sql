-- Public catalogue cuts use these bounded subject scans and exact detail lookups.
CREATE INDEX public_snapshot_epoch_subject ON public_snapshots (family, epoch, subject)
    WHERE NOT is_watermark;
CREATE INDEX public_guild_name_subject ON public_snapshots
    (epoch, (payload->'guild'->0->>'name'), subject)
    WHERE family='guild' AND NOT is_watermark;
CREATE INDEX public_house_world_key_subject ON public_houses_v2
    (epoch, source_authority, (payload->>'world_id'), (payload->>'house_key'), subject);
CREATE INDEX public_world_name_id ON world_status (name COLLATE "C", id);
-- Older rows have no retained observation evidence; never invent a historical time.
ALTER TABLE public_deaths_v2 ADD COLUMN observed bigint CHECK (observed >= 0);
