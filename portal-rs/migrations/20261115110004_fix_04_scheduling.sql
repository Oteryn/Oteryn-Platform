-- Durable round-robin progress is independent of an individual attempt's rollback.
-- No custody, reservation, or delivery evidence is inferred from this cursor.
CREATE TABLE bazaar_work_cursors (
    queue text PRIMARY KEY,
    cursor_id uuid NOT NULL
);

-- Authority resync progresses only within one nonrollback fence and world set.
CREATE TABLE bazaar_resync_progress (
    projection_epoch numeric(20,0) NOT NULL,
    recovery_generation numeric(20,0) NOT NULL,
    worlds text NOT NULL,
    listing_cursor uuid NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    operation_cursor numeric(20,0) NOT NULL DEFAULT 0,
    PRIMARY KEY(projection_epoch,recovery_generation)
);
CREATE INDEX bazaar_operation_listing_command_revision ON bazaar_game_outbox(listing_id,command,source_revision DESC);
CREATE INDEX bazaar_pending_offer_listing_id ON bazaar_offers(listing_id,id) WHERE state='pending';

-- Moderation lock order. Statement triggers run before implicit row locks;
-- explicit listing locks acquire the same shared gate in the application.
CREATE FUNCTION bazaar_moderation_statement_gate() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_advisory_xact_lock_shared(64002);
    RETURN NULL;
END;
$$;
CREATE TRIGGER bazaar_moderation_listing_gate BEFORE INSERT OR UPDATE ON bazaar_listings FOR EACH STATEMENT EXECUTE FUNCTION bazaar_moderation_statement_gate();
CREATE TRIGGER bazaar_moderation_bid_gate BEFORE INSERT ON bazaar_bids FOR EACH STATEMENT EXECUTE FUNCTION bazaar_moderation_statement_gate();
CREATE TRIGGER bazaar_moderation_offer_gate BEFORE INSERT OR UPDATE ON bazaar_offers FOR EACH STATEMENT EXECUTE FUNCTION bazaar_moderation_statement_gate();
CREATE TRIGGER bazaar_moderation_settlement_gate BEFORE INSERT OR UPDATE ON bazaar_settlements FOR EACH STATEMENT EXECUTE FUNCTION bazaar_moderation_statement_gate();
CREATE OR REPLACE FUNCTION bazaar_moderation_fence() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE listing uuid; item_active boolean;
BEGIN
    PERFORM pg_advisory_xact_lock_shared(64002);
    IF TG_TABLE_NAME = 'bazaar_listings' THEN
        listing := NEW.id;
        item_active := NEW.kind='item' AND (NEW.state='active' OR (TG_OP='UPDATE' AND OLD.state='active'));
    ELSE
        listing := NEW.listing_id;
        SELECT kind='item' AND state='active' INTO item_active FROM bazaar_listings WHERE id=listing FOR UPDATE;
    END IF;
    IF item_active AND EXISTS(SELECT 1 FROM bazaar_item_restrictions) THEN
        RAISE EXCEPTION 'item policy requires verified binding' USING ERRCODE='23514';
    END IF;
    IF EXISTS(SELECT 1 FROM bazaar_moderation_holds WHERE listing_id=listing) THEN
        RAISE EXCEPTION 'listing suspended' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER bazaar_moderation_listing_insert BEFORE INSERT ON bazaar_listings FOR EACH ROW EXECUTE FUNCTION bazaar_moderation_fence();

-- Character transfer publication and expiry reconciliation. Preserve all original
-- bytes; retirement requires an authoritative terminal no-commit receipt.
CREATE TABLE bazaar_authority_transfer_retries (
    LIKE bazaar_authority_transfers INCLUDING DEFAULTS INCLUDING CONSTRAINTS,
    PRIMARY KEY(operation_id),
    FOREIGN KEY(listing_id) REFERENCES bazaar_listings(id)
);
CREATE TRIGGER bazaar_transfer_retries_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON bazaar_authority_transfer_retries FOR EACH STATEMENT EXECUTE FUNCTION bazaar_append_only();
CREATE TABLE bazaar_authority_transfer_retirements (
    operation_id uuid PRIMARY KEY,
    evidence_operation_id uuid NOT NULL,
    recovery_generation numeric(20,0) NOT NULL,
    retired_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TRIGGER bazaar_transfer_retirements_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON bazaar_authority_transfer_retirements FOR EACH STATEMENT EXECUTE FUNCTION bazaar_append_only();
CREATE VIEW bazaar_all_transfers AS SELECT * FROM bazaar_authority_transfers UNION ALL SELECT * FROM bazaar_authority_transfer_retries;
CREATE VIEW bazaar_current_transfers AS SELECT t.* FROM bazaar_all_transfers t WHERE NOT EXISTS(SELECT 1 FROM bazaar_authority_transfer_retirements r WHERE r.operation_id=t.operation_id);
CREATE INDEX bazaar_transfer_retry_listing_purpose ON bazaar_authority_transfer_retries(listing_id,purpose,source_revision DESC);
CREATE FUNCTION bazaar_character_publication_barrier() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    -- The same fence protects every Character issuer, including service commands.
    PERFORM pg_advisory_xact_lock(22005160000);
    IF EXISTS (
        SELECT 1 FROM bazaar_current_transfers t
        WHERE t.source_revision < NEW.source_revision
          AND (t.intent::jsonb->>'expires_at_source')::bigint > extract(epoch FROM clock_timestamp())
          AND NOT EXISTS(SELECT 1 FROM character_commands c WHERE c.id=t.operation_id)
    ) THEN
        RAISE EXCEPTION 'private Character transfer awaits publication' USING ERRCODE='55P03';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER bazaar_character_publication_barrier BEFORE INSERT ON character_commands FOR EACH ROW EXECUTE FUNCTION bazaar_character_publication_barrier();

CREATE INDEX bazaar_transfer_publication_deadline ON bazaar_authority_transfers(((intent::jsonb->>'expires_at_source')::bigint),source_revision);
CREATE INDEX bazaar_transfer_retry_publication_deadline ON bazaar_authority_transfer_retries(((intent::jsonb->>'expires_at_source')::bigint),source_revision);
