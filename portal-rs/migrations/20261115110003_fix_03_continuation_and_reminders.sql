-- Leave the old reminder table and its conflict target available to older
-- binaries. New scheduling uses the scoped table; existing intent IDs and
-- delivery state are preserved and synchronized throughout rolling upgrades.
CREATE TABLE pd_scoped_reminders (LIKE pd_reminders INCLUDING DEFAULTS INCLUDING CONSTRAINTS);
ALTER TABLE pd_scoped_reminders ADD PRIMARY KEY (id);
ALTER TABLE pd_scoped_reminders ADD CONSTRAINT pd_scoped_reminders_identity
    UNIQUE NULLS NOT DISTINCT (account_id, kind, entity_key, channel, due_at_ms, character_id, cut);
CREATE INDEX pd_scoped_reminders_pending ON pd_scoped_reminders(due_at_ms) WHERE state='pending';
INSERT INTO pd_scoped_reminders SELECT * FROM pd_reminders;
CREATE FUNCTION pd_reminder_legacy_sync() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF pg_trigger_depth() > 1 THEN RETURN NEW; END IF;
    INSERT INTO pd_scoped_reminders SELECT NEW.*
    ON CONFLICT(id) DO UPDATE SET state=EXCLUDED.state,updated_at=EXCLUDED.updated_at;
    RETURN NEW;
END $$;
CREATE TRIGGER pd_reminder_legacy_sync AFTER INSERT OR UPDATE ON pd_reminders
    FOR EACH ROW EXECUTE FUNCTION pd_reminder_legacy_sync();
CREATE FUNCTION pd_reminder_scoped_sync() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF pg_trigger_depth() > 1 THEN RETURN NEW; END IF;
    UPDATE pd_reminders SET state=NEW.state,updated_at=NEW.updated_at WHERE id=NEW.id;
    RETURN NEW;
END $$;
CREATE TRIGGER pd_reminder_scoped_sync AFTER UPDATE ON pd_scoped_reminders
    FOR EACH ROW EXECUTE FUNCTION pd_reminder_scoped_sync();

-- Ledger continuation is independent of presentation and replay retention.
CREATE TABLE pd_bank_continuations (
    stream_key text PRIMARY KEY REFERENCES pd_streams(stream_key),
    entry_id text NOT NULL,
    balance_after text NOT NULL,
    coverage_ms bigint NOT NULL
);
INSERT INTO pd_bank_continuations(stream_key,entry_id,balance_after,coverage_ms)
SELECT stream_key,entity_key,convert_from(payload,'UTF8')::jsonb->>'balance_after',coverage_ms
FROM (
    SELECT DISTINCT ON (r.stream_key) r.stream_key,r.entity_key,r.payload,r.coverage_ms
    FROM pd_records r JOIN pd_streams s USING(stream_key)
    WHERE s.family='BANK'
    ORDER BY r.stream_key,r.source_revision DESC
) latest WHERE payload IS NOT NULL;
