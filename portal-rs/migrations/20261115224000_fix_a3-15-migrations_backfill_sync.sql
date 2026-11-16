-- Drain in-flight legacy writes before taking either repair snapshot. These
-- transaction-scoped locks survive the copies and trigger installation. Lock
-- records before continuations, matching the journal writer's lock order.
LOCK TABLE pd_records IN SHARE ROW EXCLUSIVE MODE;
LOCK TABLE pd_reminders, pd_scoped_reminders IN SHARE ROW EXCLUSIVE MODE;

-- A missed legacy intent may already have been scheduled by a new binary with
-- another ID. Restore the legacy ID so both schedulers address the same intent;
-- preserve a terminal scoped state rather than resurrecting delivery.
UPDATE pd_scoped_reminders s SET id=l.id,
    state=CASE WHEN s.state='pending' THEN l.state ELSE s.state END,
    updated_at=GREATEST(s.updated_at,l.updated_at)
FROM pd_reminders l
WHERE (s.account_id,s.kind,s.entity_key,s.channel,s.due_at_ms) =
      (l.account_id,l.kind,l.entity_key,l.channel,l.due_at_ms)
  AND s.character_id IS NOT DISTINCT FROM l.character_id
  AND s.cut IS NOT DISTINCT FROM l.cut AND s.id<>l.id;
INSERT INTO pd_scoped_reminders SELECT * FROM pd_reminders
ON CONFLICT(id) DO UPDATE SET
    state=CASE WHEN pd_scoped_reminders.state='pending' THEN EXCLUDED.state
        ELSE pd_scoped_reminders.state END,
    updated_at=GREATEST(pd_scoped_reminders.updated_at,EXCLUDED.updated_at);
UPDATE pd_reminders l SET state=s.state,updated_at=s.updated_at
FROM pd_scoped_reminders s WHERE l.id=s.id AND l.state IS DISTINCT FROM s.state;

-- Old journal binaries only append pd_records. Keep their accepted BANK chain
-- in the durable continuation even after presentation/replay retention expires.
-- Ignore content-free refusal rows and never roll back to an older retained row.
CREATE FUNCTION pd_bank_continuation_sync() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.kind='BANK_ENTRY' AND NEW.payload IS NOT NULL
        AND EXISTS (SELECT 1 FROM pd_streams WHERE stream_key=NEW.stream_key AND family='BANK')
        AND NOT EXISTS (SELECT 1 FROM pd_records WHERE stream_key=NEW.stream_key
            AND source_revision>NEW.source_revision) THEN
        INSERT INTO pd_bank_continuations(stream_key,entry_id,balance_after,coverage_ms)
        VALUES(NEW.stream_key,NEW.entity_key,convert_from(NEW.payload,'UTF8')::jsonb->>'balance_after',NEW.coverage_ms)
        ON CONFLICT(stream_key) DO UPDATE SET entry_id=EXCLUDED.entry_id,
            balance_after=EXCLUDED.balance_after,coverage_ms=EXCLUDED.coverage_ms;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER pd_bank_continuation_sync AFTER INSERT ON pd_records
    FOR EACH ROW EXECUTE FUNCTION pd_bank_continuation_sync();
INSERT INTO pd_bank_continuations(stream_key,entry_id,balance_after,coverage_ms)
SELECT stream_key,entity_key,convert_from(payload,'UTF8')::jsonb->>'balance_after',coverage_ms
FROM (
    SELECT DISTINCT ON (r.stream_key) r.*
    FROM pd_records r JOIN pd_streams s USING(stream_key)
    WHERE s.family='BANK'
    ORDER BY r.stream_key,r.source_revision DESC
) latest WHERE kind='BANK_ENTRY' AND payload IS NOT NULL
ON CONFLICT(stream_key) DO UPDATE SET entry_id=EXCLUDED.entry_id,
    balance_after=EXCLUDED.balance_after,coverage_ms=EXCLUDED.coverage_ms;
