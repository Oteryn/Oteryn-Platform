-- Preserve the accepted stream fence independently of the published watermark's
-- generation, which remains evidence about that exact watermark only.
ALTER TABLE pd_streams ADD COLUMN recovery_high_water numeric(20,0);
UPDATE pd_streams SET recovery_high_water=watermark_recovery;

-- Durable handoff from authoritative lifecycle writers to journal erasure.
-- Triggers enqueue only: they never invert the journal's account/authority locks.
CREATE TABLE pd_erasure_jobs (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    account_id text NOT NULL,
    character_id text,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX pd_erasure_jobs_scope ON pd_erasure_jobs(account_id,character_id);
CREATE FUNCTION pd_queue_account_erasure() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        INSERT INTO pd_erasure_jobs(account_id) VALUES(OLD.id::text);
        RETURN OLD;
    END IF;
    IF NEW.status = 'closed' AND (TG_OP = 'INSERT' OR OLD.status IS DISTINCT FROM NEW.status) THEN
        INSERT INTO pd_erasure_jobs(account_id) VALUES(NEW.id::text);
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER pd_account_erasure AFTER INSERT OR UPDATE OF status OR DELETE ON accounts
    FOR EACH ROW EXECUTE FUNCTION pd_queue_account_erasure();
CREATE FUNCTION pd_queue_transfer_erasure() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.command = 'TransferCharacterOwnership' AND NEW.state = 'TERMINAL'
       AND NEW.receipt_json::jsonb->>'outcome' = 'COMMITTED'
       AND (TG_OP = 'INSERT' OR OLD.receipt_json IS NULL) THEN
        INSERT INTO pd_erasure_jobs(account_id,character_id)
        VALUES(NEW.receipt_json::jsonb->>'from_account_id',NEW.receipt_json::jsonb->>'character_id');
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER pd_transfer_erasure AFTER INSERT OR UPDATE OF receipt_json ON character_commands
    FOR EACH ROW EXECUTE FUNCTION pd_queue_transfer_erasure();
CREATE FUNCTION pd_queue_character_erasure() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT NEW.invalid AND NEW.snapshot_json::jsonb->>'lifecycle' IN ('DELETED','REMOVED_FROM_ACCOUNT') THEN
        INSERT INTO pd_erasure_jobs(account_id,character_id) VALUES(NEW.account_id::text,NEW.character_id::text);
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER pd_character_erasure AFTER INSERT OR UPDATE ON character_service_states
    FOR EACH ROW EXECUTE FUNCTION pd_queue_character_erasure();
-- Cover lifecycle transitions accepted before this migration, too.
INSERT INTO pd_erasure_jobs(account_id) SELECT id::text FROM accounts WHERE status='closed';
INSERT INTO pd_erasure_jobs(account_id,character_id)
SELECT receipt_json::jsonb->>'from_account_id',receipt_json::jsonb->>'character_id'
FROM character_commands WHERE command='TransferCharacterOwnership' AND state='TERMINAL'
AND receipt_json::jsonb->>'outcome'='COMMITTED';
INSERT INTO pd_erasure_jobs(account_id,character_id)
SELECT account_id::text,character_id::text FROM character_service_states
WHERE NOT invalid AND snapshot_json::jsonb->>'lifecycle' IN ('DELETED','REMOVED_FROM_ACCOUNT');
