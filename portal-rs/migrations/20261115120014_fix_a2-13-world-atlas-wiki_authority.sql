-- Authority history survives pointer clearing and process restarts. Ordinary refresh
-- cannot delete retirement records; recovery requires a separately audited operator transition.
ALTER TABLE world_catalog_publication ADD COLUMN authority_epoch text, ADD COLUMN configured_snapshot_id text;
UPDATE world_catalog_publication p SET authority_epoch=c.authority_epoch
FROM world_catalog_candidates c WHERE c.snapshot_id=p.snapshot_id;
CREATE TABLE world_catalog_retired_epochs (
    authority_epoch text PRIMARY KEY,
    retired_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE world_catalog_retired_snapshots (
    snapshot_id text PRIMARY KEY,
    retired_at timestamptz NOT NULL DEFAULT now()
);

-- Preserve alias rows for old readers while explicitly copying canonical identities.
-- Conflicting records are withheld pending reimport; conflicting approvals deny.
CREATE FUNCTION world_canonical_kind(value text) RETURNS text LANGUAGE sql IMMUTABLE AS $$
SELECT CASE value WHEN 'items' THEN 'Item' WHEN 'creatures' THEN 'Creature'
WHEN 'npcs' THEN 'NPC' WHEN 'spells' THEN 'Spell' WHEN 'quests' THEN 'Quest'
WHEN 'loot' THEN 'Loot' WHEN 'achievements' THEN 'Achievement' WHEN 'areas' THEN 'Area'
WHEN 'spawns' THEN 'Spawn.Source' ELSE value END
$$;
INSERT INTO world_entities(version_id,kind,key,data,publishable)
SELECT version_id,world_canonical_kind(kind),key,data,publishable FROM world_entities
WHERE kind<>world_canonical_kind(kind)
ON CONFLICT(version_id,kind,key) DO UPDATE SET publishable=
    world_entities.publishable AND EXCLUDED.publishable AND world_entities.data=EXCLUDED.data;
INSERT INTO world_visibility(kind,key,allowed)
SELECT world_canonical_kind(kind),key,allowed FROM world_visibility WHERE kind<>world_canonical_kind(kind)
ON CONFLICT(kind,key) DO UPDATE SET allowed=world_visibility.allowed AND EXCLUDED.allowed;
INSERT INTO world_admin_names(kind,key,pl,en)
SELECT world_canonical_kind(kind),key,pl,en FROM world_admin_names WHERE kind<>world_canonical_kind(kind)
ON CONFLICT DO NOTHING;

-- Seed the fence from projections accepted before this migration. If the old
-- pointer was cleared, no retained projection is evidence of current authority.
INSERT INTO world_catalog_retired_snapshots(snapshot_id)
SELECT v.snapshot_id FROM world_catalog_versions v CROSS JOIN world_catalog_publication p
WHERE v.snapshot_id IS DISTINCT FROM p.snapshot_id ON CONFLICT DO NOTHING;
INSERT INTO world_catalog_retired_epochs(authority_epoch)
SELECT DISTINCT c.authority_epoch FROM world_catalog_versions v
JOIN world_catalog_candidates c USING(snapshot_id) CROSS JOIN world_catalog_publication p
WHERE c.authority_epoch IS DISTINCT FROM p.authority_epoch ON CONFLICT DO NOTHING;

-- Operator-only recovery seam: no browser or automatic refresh calls this.
-- A verified inactive target, actor, exact profile scope, authority evidence and
-- rollback plan are mandatory. The next ordinary refresh must match this target.
CREATE TABLE world_catalog_authority_recoveries (
    recovery_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    actor uuid NOT NULL CHECK (actor<>'00000000-0000-0000-0000-000000000000'),
    -- Audit provenance must survive catalogue rollback or candidate retention cleanup.
    snapshot_id text NOT NULL CHECK (snapshot_id ~ '^sha256:[0-9a-f]{64}$'),
    authority_epoch text NOT NULL,
    ruleset_id text NOT NULL,
    content_profile_id text NOT NULL,
    authority_evidence text NOT NULL CHECK (length(btrim(authority_evidence)) BETWEEN 1 AND 4096),
    rollback_plan text NOT NULL CHECK (length(btrim(rollback_plan)) BETWEEN 1 AND 4096),
    recovered_at timestamptz NOT NULL DEFAULT now()
);
CREATE FUNCTION world_catalog_recover_authority(
    recovery_actor uuid, target_snapshot text, target_ruleset text,
    target_profile text, evidence text, recovery_plan text
) RETURNS bigint LANGUAGE plpgsql AS $$
DECLARE
    target_epoch text;
    previous_epoch text;
    previous_snapshot text;
    recovery bigint;
BEGIN
    SELECT authority_epoch,COALESCE(configured_snapshot_id,snapshot_id)
    INTO previous_epoch,previous_snapshot FROM world_catalog_publication WHERE singleton FOR UPDATE;
    SELECT authority_epoch INTO STRICT target_epoch FROM world_catalog_candidates
    WHERE snapshot_id=target_snapshot AND ruleset_id=target_ruleset AND content_profile_id=target_profile;
    INSERT INTO world_catalog_authority_recoveries(actor,snapshot_id,authority_epoch,
        ruleset_id,content_profile_id,authority_evidence,rollback_plan)
    VALUES(recovery_actor,target_snapshot,target_epoch,target_ruleset,target_profile,evidence,recovery_plan)
    RETURNING recovery_id INTO recovery;
    IF previous_epoch IS NOT NULL AND previous_epoch<>target_epoch THEN
        INSERT INTO world_catalog_retired_epochs(authority_epoch) VALUES(previous_epoch) ON CONFLICT DO NOTHING;
    END IF;
    IF previous_snapshot IS NOT NULL AND previous_snapshot<>target_snapshot THEN
        INSERT INTO world_catalog_retired_snapshots(snapshot_id) VALUES(previous_snapshot) ON CONFLICT DO NOTHING;
    END IF;
    DELETE FROM world_catalog_retired_epochs WHERE authority_epoch=target_epoch;
    DELETE FROM world_catalog_retired_snapshots WHERE snapshot_id=target_snapshot;
    UPDATE world_catalog_publication SET authority_epoch=target_epoch,
        configured_snapshot_id=target_snapshot,snapshot_id=NULL WHERE singleton;
    RETURN recovery;
END;
$$;
CREATE FUNCTION world_catalog_recovery_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'Catalogue authority recovery history is immutable';
END;
$$;
CREATE TRIGGER world_catalog_recovery_immutable BEFORE UPDATE OR DELETE ON world_catalog_authority_recoveries
FOR EACH ROW EXECUTE FUNCTION world_catalog_recovery_immutable();
CREATE TRIGGER world_catalog_recovery_no_truncate BEFORE TRUNCATE ON world_catalog_authority_recoveries
FOR EACH STATEMENT EXECUTE FUNCTION world_catalog_recovery_immutable();
