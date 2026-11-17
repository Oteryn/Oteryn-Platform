-- A single replacement receipt can resolve reopens from several notices.
CREATE FUNCTION character_restore_acknowledge_resolved() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    -- Serialize receipts for different operations of the same notice before
    -- checking its remaining reopens. Take overlapping notice locks in order.
    PERFORM n.restore_event_id FROM character_restore_notices n
    JOIN resolved r USING (restore_event_id)
    WHERE r.new_receipt_id IS NOT NULL AND n.acknowledged_at IS NULL
    ORDER BY n.restore_event_id FOR NO KEY UPDATE OF n;

    UPDATE character_restore_notices n SET acknowledged_at=clock_timestamp()
    WHERE n.acknowledged_at IS NULL
      AND EXISTS (SELECT 1 FROM resolved r WHERE r.restore_event_id=n.restore_event_id AND r.new_receipt_id IS NOT NULL)
      AND NOT EXISTS (SELECT 1 FROM character_restore_reopens r
                      WHERE r.restore_event_id=n.restore_event_id AND r.new_receipt_id IS NULL);
    RETURN NULL;
END;
$$;
CREATE TRIGGER character_restore_acknowledge_resolved AFTER UPDATE ON character_restore_reopens
REFERENCING NEW TABLE AS resolved FOR EACH STATEMENT
EXECUTE FUNCTION character_restore_acknowledge_resolved();

-- Repair notices left pending by receipts accepted before this migration.
UPDATE character_restore_notices n SET acknowledged_at=clock_timestamp()
WHERE n.acknowledged_at IS NULL AND NOT EXISTS (
    SELECT 1 FROM character_restore_reopens r
    WHERE r.restore_event_id=n.restore_event_id AND r.new_receipt_id IS NULL
);
