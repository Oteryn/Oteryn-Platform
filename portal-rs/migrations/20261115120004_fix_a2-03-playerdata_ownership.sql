-- A transfer closes an ownership period, not every future period of that owner.
ALTER TABLE pd_erasure_jobs ADD COLUMN closed_revision numeric(20,0);
ALTER TABLE pd_deleted_scopes ADD COLUMN closed_revision numeric(20,0);
CREATE OR REPLACE FUNCTION pd_queue_transfer_erasure() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.command = 'TransferCharacterOwnership' AND NEW.state = 'TERMINAL'
       AND NEW.receipt_json::jsonb->>'outcome' = 'COMMITTED'
       AND (TG_OP = 'INSERT' OR OLD.receipt_json IS NULL) THEN
        INSERT INTO pd_erasure_jobs(account_id,character_id,closed_revision)
        VALUES(NEW.receipt_json::jsonb->>'from_account_id',NEW.receipt_json::jsonb->>'character_id',
               (NEW.receipt_json::jsonb->>'character_revision')::numeric);
    END IF;
    RETURN NEW;
END $$;
-- Recover the closed cut for pre-upgrade transfer handoffs/tombstones. Actual
-- deletion evidence always wins; an ambiguous legacy tombstone stays closed.
WITH cuts AS (
 SELECT receipt_json::jsonb->>'from_account_id' AS account,
        receipt_json::jsonb->>'character_id' AS character,
        max((receipt_json::jsonb->>'character_revision')::numeric) AS revision
 FROM character_commands WHERE command='TransferCharacterOwnership' AND state='TERMINAL'
 AND receipt_json::jsonb->>'outcome'='COMMITTED' GROUP BY 1,2
), transfers AS (
 SELECT * FROM cuts c WHERE NOT EXISTS (
 SELECT 1 FROM character_service_states s WHERE s.account_id::text=c.account
 AND s.character_id::text=c.character AND NOT s.invalid
 AND s.snapshot_json::jsonb->>'lifecycle' = 'DELETED')
)
UPDATE pd_deleted_scopes d SET closed_revision=c.revision FROM transfers c
WHERE d.scope_digest=sha256(convert_to('13:deleted-scope|'||length(c.account)||':'||c.account||'|'||length(c.character)||':'||c.character,'UTF8'));
WITH cuts AS (
 SELECT receipt_json::jsonb->>'from_account_id' AS account,
        receipt_json::jsonb->>'character_id' AS character,
        max((receipt_json::jsonb->>'character_revision')::numeric) AS revision
 FROM character_commands WHERE command='TransferCharacterOwnership' AND state='TERMINAL'
 AND receipt_json::jsonb->>'outcome'='COMMITTED' GROUP BY 1,2
), transfers AS (
 SELECT * FROM cuts c WHERE NOT EXISTS (
 SELECT 1 FROM character_service_states s WHERE s.account_id::text=c.account
 AND s.character_id::text=c.character AND NOT s.invalid
 AND s.snapshot_json::jsonb->>'lifecycle' = 'DELETED')
)
UPDATE pd_erasure_jobs j SET closed_revision=c.revision FROM transfers c
WHERE j.account_id=c.account AND j.character_id=c.character;

CREATE OR REPLACE FUNCTION pd_queue_character_erasure() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT NEW.invalid AND NEW.snapshot_json::jsonb->>'lifecycle' IN ('DELETED','REMOVED_FROM_ACCOUNT') THEN
        INSERT INTO pd_erasure_jobs(account_id,character_id,closed_revision)
        VALUES(NEW.account_id::text,NEW.character_id::text,
            CASE WHEN NEW.snapshot_json::jsonb->>'lifecycle'='REMOVED_FROM_ACCOUNT'
                 THEN NEW.character_revision::numeric END);
    END IF;
    RETURN NEW;
END $$;

-- Repair acknowledged pre-upgrade purges, including projections the old worker
-- omitted. Recover scopes only from existing rows and durable deletion evidence.
INSERT INTO pd_erasure_jobs(account_id,character_id,closed_revision)
SELECT DISTINCT subjects.account,subjects.character,d.closed_revision FROM (
 SELECT s.account_id AS account,r.character_id AS character FROM pd_records r JOIN pd_streams s USING(stream_key) WHERE r.character_id IS NOT NULL
 UNION SELECT account_id::text,character_id::text FROM pd_game_logins WHERE character_id IS NOT NULL
 UNION SELECT account_id::text,character_id::text FROM pd_t76_streams WHERE character_id IS NOT NULL
 UNION SELECT account_id::text,character_id::text FROM pd_t78_records
) subjects JOIN pd_deleted_scopes d
ON d.scope_digest=sha256(convert_to('13:deleted-scope|'||length(subjects.account)||':'||subjects.account||'|'||length(subjects.character)||':'||subjects.character,'UTF8'))
WHERE NOT EXISTS(SELECT 1 FROM pd_erasure_jobs j WHERE j.account_id=subjects.account AND j.character_id=subjects.character);
INSERT INTO pd_erasure_jobs(account_id)
SELECT DISTINCT s.account_id FROM pd_streams s WHERE s.erased
AND NOT EXISTS(SELECT 1 FROM pd_erasure_jobs j WHERE j.account_id=s.account_id AND j.character_id IS NULL);
