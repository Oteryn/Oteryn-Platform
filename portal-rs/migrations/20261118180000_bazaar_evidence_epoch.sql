ALTER TABLE bazaar_game_issuer
    ADD COLUMN evidence_epoch bigint NOT NULL DEFAULT 0 CHECK (evidence_epoch >= 0),
    ADD COLUMN evidence_transaction xid8,
    ADD COLUMN minimum_writer_version integer NOT NULL DEFAULT 0 CHECK (minimum_writer_version >= 0);

ALTER TABLE bazaar_resync_progress
    ADD COLUMN evidence_epoch bigint CHECK (evidence_epoch >= 0),
    ADD COLUMN evidence_restarts integer NOT NULL DEFAULT 0 CHECK (evidence_restarts BETWEEN 0 AND 3);

-- Unversioned binaries have no transaction setting. Check at the database
-- write boundary so their intake and checkpoint writes cannot bypass the floor.
CREATE FUNCTION bazaar_require_writer(minimum_version integer) RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    IF coalesce(nullif(current_setting('oteryn.bazaar_writer_version', true), ''), '0')::integer < minimum_version THEN
        RAISE EXCEPTION 'Bazaar writer version is below the issuer minimum' USING ERRCODE = '55000';
    END IF;
END;
$$;

CREATE FUNCTION bazaar_fence_writer() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE minimum_version integer;
BEGIN
    SELECT minimum_writer_version INTO STRICT minimum_version
        FROM bazaar_game_issuer WHERE singleton FOR UPDATE;
    PERFORM bazaar_require_writer(minimum_version);
    RETURN NULL;
END;
$$;

CREATE FUNCTION bazaar_advance_evidence_epoch() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE writer_transaction xid8 := pg_current_xact_id();
BEGIN
    -- Every evidence writer holds the issuer lock. This also coalesces a receipt,
    -- conflict and decision audit admitted within one transaction.
    UPDATE bazaar_game_issuer
        SET evidence_epoch = evidence_epoch + 1, evidence_transaction = writer_transaction
        WHERE singleton AND evidence_transaction IS DISTINCT FROM writer_transaction;
    RETURN NULL;
END;
$$;

-- Reopen updates the outbox in the same transaction as the issuer fence,
-- including when there are zero old commands to reopen.
CREATE TRIGGER bazaar_writer_fence BEFORE UPDATE OF state ON bazaar_game_outbox
    FOR EACH STATEMENT EXECUTE FUNCTION bazaar_fence_writer();

DO $$
DECLARE relation text;
BEGIN
    FOREACH relation IN ARRAY ARRAY[
        'bazaar_authority_receipts', 'bazaar_authority_decisions', 'bazaar_authority_conflicts',
        'bazaar_authority_dispatch', 'bazaar_authority_compensations',
        'bazaar_game_quarantine'
    ] LOOP
        EXECUTE format('CREATE TRIGGER bazaar_writer_fence BEFORE INSERT ON %I
            FOR EACH STATEMENT EXECUTE FUNCTION bazaar_fence_writer()', relation);
        EXECUTE format('CREATE TRIGGER bazaar_evidence_epoch AFTER INSERT ON %I
            FOR EACH ROW EXECUTE FUNCTION bazaar_advance_evidence_epoch()', relation);
    END LOOP;

    FOREACH relation IN ARRAY ARRAY['bazaar_resync_progress', 'bazaar_authority_resync'] LOOP
        EXECUTE format('CREATE TRIGGER bazaar_writer_fence BEFORE INSERT OR UPDATE OR DELETE ON %I
            FOR EACH STATEMENT EXECUTE FUNCTION bazaar_fence_writer()', relation);
    END LOOP;
END;
$$;

CREATE TRIGGER bazaar_writer_fence BEFORE INSERT OR UPDATE ON bazaar_game_projections
    FOR EACH STATEMENT EXECUTE FUNCTION bazaar_fence_writer();
-- Watermarks advance on their own cadence and do not change scan evidence.
CREATE TRIGGER bazaar_evidence_epoch AFTER INSERT OR UPDATE ON bazaar_game_projections
    FOR EACH ROW WHEN (NEW.kind IN ('STATUS', 'POLICY'))
    EXECUTE FUNCTION bazaar_advance_evidence_epoch();

-- A certified legacy worker may only write recheck work or a retry audit,
-- without touching either checkpoint table. Fence those writes as well.
CREATE TRIGGER bazaar_writer_fence_insert AFTER INSERT OR UPDATE ON bazaar_work_cursors
    FOR EACH ROW WHEN (NEW.queue LIKE 'delivery-dirty:%' OR NEW.queue LIKE 'resync-listing-dirty:%'
        OR NEW.queue IN ('delivery-evidence-scan', 'resync-listing-scan'))
    EXECUTE FUNCTION bazaar_fence_writer();
CREATE TRIGGER bazaar_writer_fence_delete AFTER DELETE ON bazaar_work_cursors
    FOR EACH ROW WHEN (OLD.queue LIKE 'delivery-dirty:%' OR OLD.queue LIKE 'resync-listing-dirty:%'
        OR OLD.queue IN ('delivery-evidence-scan', 'resync-listing-scan'))
    EXECUTE FUNCTION bazaar_fence_writer();
CREATE TRIGGER bazaar_delivery_audit_writer_fence AFTER INSERT ON bazaar_audit
    FOR EACH ROW WHEN (NEW.relation_name = 'bazaar_delivery_evidence')
    EXECUTE FUNCTION bazaar_fence_writer();
