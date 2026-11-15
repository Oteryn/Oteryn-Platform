-- Canonicalize in the scoped unique index before a legacy INSERT can return
-- its generated ID. The upsert also serializes simultaneous old/new writers.
CREATE FUNCTION pd_reminder_legacy_canonical() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE canonical pd_scoped_reminders%ROWTYPE;
BEGIN
    IF pg_trigger_depth() > 1 THEN RETURN NEW; END IF;
    IF TG_OP='INSERT' THEN
        -- Older schedulers refuse a different character/cut at their coarser
        -- conflict target. Serialize legacy inserts so a refused attempt cannot
        -- leave a new scoped intent behind before ON CONFLICT checks its WHERE.
        PERFORM pg_advisory_xact_lock(hashtextextended(jsonb_build_array(
            NEW.account_id,NEW.kind,NEW.entity_key,NEW.channel,NEW.due_at_ms)::text,15120016));
        IF EXISTS (SELECT 1 FROM pd_reminders WHERE account_id=NEW.account_id
            AND kind=NEW.kind AND entity_key=NEW.entity_key AND channel=NEW.channel
            AND due_at_ms=NEW.due_at_ms AND (character_id IS DISTINCT FROM NEW.character_id
                OR cut IS DISTINCT FROM NEW.cut)) THEN
            RETURN NEW;
        END IF;
    END IF;
    INSERT INTO pd_scoped_reminders SELECT NEW.*
    ON CONFLICT ON CONSTRAINT pd_scoped_reminders_identity DO UPDATE
        SET updated_at=pd_scoped_reminders.updated_at
    RETURNING * INTO canonical;
    NEW.id := canonical.id;
    IF canonical.state <> 'pending' THEN
        NEW.state := canonical.state;
        NEW.updated_at := canonical.updated_at;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER pd_reminder_legacy_canonical BEFORE INSERT OR UPDATE ON pd_reminders
    FOR EACH ROW EXECUTE FUNCTION pd_reminder_legacy_canonical();
CREATE OR REPLACE FUNCTION pd_reminder_legacy_sync() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF pg_trigger_depth() > 1 THEN RETURN NEW; END IF;
    INSERT INTO pd_scoped_reminders SELECT NEW.*
    ON CONFLICT(id) DO UPDATE SET
        state=CASE WHEN pd_scoped_reminders.state='pending' THEN EXCLUDED.state
            ELSE pd_scoped_reminders.state END,
        updated_at=EXCLUDED.updated_at;
    RETURN NEW;
END $$;

-- SANCTION EXPIRY CANDIDATES
-- The permanent journal is consulted once at upgrade, never rediscovered on a
-- foreground poll. Publication already owns the counter before this trigger.
-- Close the backfill/trigger-installation gap against concurrent publishers.
LOCK TABLE support_sanction_export_revisions IN SHARE ROW EXCLUSIVE MODE;
CREATE TABLE support_sanction_export_expiry_candidates (
    sanction_stream_id uuid PRIMARY KEY,
    position bigint NOT NULL REFERENCES support_sanction_export_revisions(position),
    expires_at timestamptz NOT NULL
);
CREATE INDEX support_sanction_export_expiry_due
    ON support_sanction_export_expiry_candidates(expires_at,sanction_stream_id);
INSERT INTO support_sanction_export_expiry_candidates
SELECT exported.sanction_stream_id,exported.position,authoritative.expires_at
FROM (
    SELECT DISTINCT ON (sanction_stream_id) *
    FROM support_sanction_export_revisions WHERE kind=5
    ORDER BY sanction_stream_id,decision_revision DESC
) exported
JOIN LATERAL (
    SELECT active,expires_at FROM support_sanction_revisions
    WHERE account_id=exported.account_id AND sanction_id=exported.sanction_stream_id
    ORDER BY revision DESC LIMIT 1
) authoritative ON true
WHERE NOT exported.revoked AND authoritative.active AND authoritative.expires_at IS NOT NULL;

CREATE FUNCTION support_sanction_export_track_expiry() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE deadline timestamptz;
BEGIN
    IF NEW.kind <> 5 THEN RETURN NEW; END IF;
    IF NOT NEW.revoked THEN
        SELECT expires_at INTO deadline FROM support_sanction_revisions
        WHERE account_id=NEW.account_id AND sanction_id=NEW.sanction_stream_id AND active
        ORDER BY revision DESC LIMIT 1;
    END IF;
    IF deadline IS NULL THEN
        DELETE FROM support_sanction_export_expiry_candidates
            WHERE sanction_stream_id=NEW.sanction_stream_id;
    ELSE
        INSERT INTO support_sanction_export_expiry_candidates VALUES
            (NEW.sanction_stream_id,NEW.position,deadline)
        ON CONFLICT(sanction_stream_id) DO UPDATE
            SET position=EXCLUDED.position,expires_at=EXCLUDED.expires_at;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER support_sanction_export_track_expiry
    AFTER INSERT ON support_sanction_export_revisions
    FOR EACH ROW EXECUTE FUNCTION support_sanction_export_track_expiry();

CREATE OR REPLACE FUNCTION support_sanction_export_expire_namelocks() RETURNS void LANGUAGE plpgsql AS $$
DECLARE
    previous support_sanction_export_revisions%ROWTYPE;
    candidate_position bigint;
BEGIN
    PERFORM 1 FROM support_sanction_export_counter WHERE singleton FOR UPDATE SKIP LOCKED;
    IF NOT FOUND THEN RETURN; END IF;
    FOR candidate_position IN
        SELECT position FROM support_sanction_export_expiry_candidates
        WHERE expires_at<=statement_timestamp()
        ORDER BY expires_at,sanction_stream_id LIMIT 500
    LOOP
        -- A primary-key lookup per candidate bounds journal work too, without
        -- allowing a hash join to scan permanent history under the counter.
        SELECT * INTO STRICT previous FROM support_sanction_export_revisions
            WHERE position=candidate_position;
        PERFORM support_sanction_export_append(previous.sanction_stream_id,previous.account_id,
            previous.character_id,previous.kind,previous.reason_code,previous.reason_text,
            previous.starts_at_ms,NULL,true);
    END LOOP;
END $$;
