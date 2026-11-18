-- Keep the generation boundary on the cursor and the two BANK identities on
-- their replay record. Old writers also populate them during rolling upgrades.
ALTER TABLE pd_streams ADD COLUMN history_coverage_ms bigint;
ALTER TABLE pd_records ADD COLUMN bank_event_id text, ADD COLUMN bank_operation_id text;

UPDATE pd_streams s SET history_coverage_ms = latest.coverage_ms
FROM (SELECT DISTINCT ON (stream_key) stream_key, coverage_ms
      FROM pd_records WHERE kind IN ('MARKET_EVENT','PLAYER_TRADE_EVENT') AND scope_key <> ''
      ORDER BY stream_key,source_revision DESC) latest
WHERE s.stream_key = latest.stream_key AND NOT s.erased;

UPDATE pd_records SET bank_event_id = convert_from(payload,'UTF8')::jsonb->>'event_id',
    bank_operation_id = convert_from(payload,'UTF8')::jsonb->>'operation_occurrence_id'
WHERE kind='BANK_ENTRY' AND payload IS NOT NULL;
CREATE INDEX pd_bank_event_lookup ON pd_records(authority,bank_event_id) WHERE bank_event_id IS NOT NULL;
CREATE INDEX pd_bank_operation_lookup ON pd_records(authority,bank_operation_id) WHERE bank_operation_id IS NOT NULL;

CREATE FUNCTION pd_capture_history_evidence() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.kind='BANK_ENTRY' AND NEW.payload IS NOT NULL THEN
        NEW.bank_event_id := convert_from(NEW.payload,'UTF8')::jsonb->>'event_id';
        NEW.bank_operation_id := convert_from(NEW.payload,'UTF8')::jsonb->>'operation_occurrence_id';
    ELSIF NEW.kind IN ('MARKET_EVENT','PLAYER_TRADE_EVENT') THEN
        UPDATE pd_streams SET history_coverage_ms=NEW.coverage_ms
        WHERE stream_key=NEW.stream_key AND history_coverage_ms IS NULL AND stored_through=0;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER pd_capture_history_evidence BEFORE INSERT ON pd_records
FOR EACH ROW EXECUTE FUNCTION pd_capture_history_evidence();
