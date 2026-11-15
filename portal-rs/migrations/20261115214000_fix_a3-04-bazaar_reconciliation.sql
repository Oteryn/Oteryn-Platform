-- Scope moderation to authenticated, audited item custody, never browser input.
CREATE FUNCTION bazaar_item_restricted(listing uuid, value bigint) RETURNS boolean
LANGUAGE plpgsql STABLE AS $$
DECLARE item jsonb; policy record;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM bazaar_item_restrictions) THEN RETURN false; END IF;
    SELECT r.receipt::jsonb->'result_data'->'item' INTO item
    FROM bazaar_game_outbox o
    JOIN bazaar_authority_receipts r USING(operation_id)
    JOIN bazaar_game_issuer issuer ON issuer.singleton
    JOIN bazaar_item_details detail ON detail.id=o.listing_id
    WHERE o.listing_id=listing AND o.command='EscrowBazaarItem'
      AND r.outcome='COMMITTED' AND r.recovery_generation=issuer.recovery_generation
      AND NOT EXISTS(SELECT 1 FROM bazaar_authority_conflicts c WHERE c.operation_id=o.operation_id)
      AND EXISTS(SELECT 1 FROM bazaar_authority_decisions d WHERE d.operation_id=o.operation_id AND d.recovery_generation=r.recovery_generation)
      AND r.receipt::jsonb->'result_data'->'item'->>'world_id'=detail.world_id::text
      AND r.receipt::jsonb->'result_data'->'item'->'definition'->>'family'='item'
      AND r.receipt::jsonb->'result_data'->'item'->'definition'->>'production_key'=detail.definition_key
      AND r.receipt::jsonb->'result_data'->'item'->'definition'->>'revision_ref'=detail.definition_revision
      AND r.receipt::jsonb->'result_data'->'item'->>'item_instance_id'=detail.item_instance_id::text
      AND r.receipt::jsonb->'result_data'->'item'->>'quantity'=detail.quantity::text
    ORDER BY o.source_revision DESC LIMIT 1;
    IF item IS NULL THEN RETURN true; END IF;
    SELECT denied,minimum_value INTO policy FROM bazaar_item_restrictions
    WHERE world_id=(item->>'world_id')::uuid AND family=item->'definition'->>'family'
      AND production_key=item->'definition'->>'production_key'
      AND revision_ref=item->'definition'->>'revision_ref'
    ORDER BY created_at DESC,id DESC LIMIT 1;
    RETURN FOUND AND (policy.denied OR value < policy.minimum_value);
END;
$$;

CREATE OR REPLACE FUNCTION bazaar_moderation_fence() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE listing uuid; item_active boolean; value bigint;
BEGIN
    PERFORM pg_advisory_xact_lock_shared(64002);
    IF TG_TABLE_NAME='bazaar_listings' THEN
        listing := NEW.id;
        -- A returned asset must be able to finish cancellation. Restrictions
        -- fence trading, rather than the transition out of the market.
        item_active := NEW.kind='item' AND NEW.state IN ('active','matched','awaiting_delivery');
        value := NEW.price;
    ELSE
        listing := NEW.listing_id;
        SELECT kind='item' AND state='active',price INTO item_active,value
        FROM bazaar_listings WHERE id=listing FOR UPDATE;
    END IF;
    IF item_active AND bazaar_item_restricted(listing,value) THEN
        RAISE EXCEPTION 'item policy requires verified binding' USING ERRCODE='23514';
    END IF;
    IF EXISTS(SELECT 1 FROM bazaar_moderation_holds WHERE listing_id=listing) THEN
        RAISE EXCEPTION 'listing suspended' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;

-- Retry progress is independent of the immutable command/audit history.
CREATE TABLE bazaar_cleanup_attempts (
    listing_id uuid NOT NULL REFERENCES bazaar_listings(id),
    command text NOT NULL,
    root_step uuid NOT NULL,
    current_step uuid NOT NULL,
    PRIMARY KEY(listing_id,command,root_step)
);
-- The small live set, indexed independently from lifetime dispatch history.
-- A nonzero generation is written only after full retained receipt verification.
CREATE TABLE bazaar_disposition_fences (
    operation_id uuid PRIMARY KEY REFERENCES bazaar_game_outbox(operation_id),
    listing_id uuid NOT NULL,
    command text NOT NULL,
    no_effect_generation numeric(20,0) NOT NULL DEFAULT 0,
    terminal_committed boolean NOT NULL DEFAULT false,
    quarantined boolean NOT NULL DEFAULT false
);
CREATE INDEX bazaar_disposition_command ON bazaar_disposition_fences(listing_id,command,operation_id);
CREATE INDEX bazaar_disposition_live ON bazaar_disposition_fences(listing_id,no_effect_generation,command,operation_id);
CREATE INDEX bazaar_disposition_reconcile ON bazaar_disposition_fences(no_effect_generation,operation_id) WHERE NOT terminal_committed AND NOT quarantined;
INSERT INTO bazaar_disposition_fences(operation_id,listing_id,command)
SELECT o.operation_id,o.listing_id,o.command FROM bazaar_authority_dispatch d
JOIN bazaar_game_outbox o USING(operation_id)
WHERE o.command IN ('DeliverBazaarItem','ReturnBazaarItem','PayoutBazaarGold');
-- Publication ordering needs only the dispatch head, not a lifetime join.
CREATE TABLE bazaar_authority_dispatch_head (
    singleton boolean PRIMARY KEY DEFAULT true CHECK(singleton),
    source_revision numeric(20,0) NOT NULL
);
INSERT INTO bazaar_authority_dispatch_head(source_revision)
SELECT coalesce(max(o.source_revision),0) FROM bazaar_authority_dispatch d
JOIN bazaar_game_outbox o USING(operation_id);
CREATE FUNCTION bazaar_disposition_dispatch() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO bazaar_disposition_fences(operation_id,listing_id,command)
    SELECT operation_id,listing_id,command FROM bazaar_game_outbox
    WHERE operation_id=NEW.operation_id AND command IN ('DeliverBazaarItem','ReturnBazaarItem','PayoutBazaarGold');
    UPDATE bazaar_authority_dispatch_head SET source_revision=greatest(source_revision,
        (SELECT o.source_revision FROM bazaar_game_outbox o WHERE o.operation_id=NEW.operation_id))
    WHERE singleton;
    RETURN NEW;
END;
$$;
CREATE TRIGGER bazaar_disposition_dispatch AFTER INSERT ON bazaar_authority_dispatch FOR EACH ROW EXECUTE FUNCTION bazaar_disposition_dispatch();
CREATE FUNCTION bazaar_disposition_conflict() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    UPDATE bazaar_disposition_fences SET no_effect_generation=0,quarantined=true WHERE operation_id=NEW.operation_id;
    RETURN NEW;
END;
$$;
CREATE TRIGGER bazaar_disposition_conflict AFTER INSERT ON bazaar_authority_conflicts FOR EACH ROW EXECUTE FUNCTION bazaar_disposition_conflict();
CREATE INDEX IF NOT EXISTS bazaar_conflict_operation ON bazaar_authority_conflicts(operation_id);
CREATE INDEX bazaar_decision_operation_generation ON bazaar_authority_decisions(operation_id,recovery_generation,source_authority);

-- Admission pressure is a projection of receipt presence, not custody evidence.
-- Indexed generation ranges keep the pending-count check independent of history.
CREATE TABLE bazaar_command_receipt_heads (
    operation_id uuid PRIMARY KEY REFERENCES bazaar_game_outbox(operation_id),
    receipt_generation numeric(20,0) NOT NULL DEFAULT 0
);
CREATE INDEX bazaar_command_receipt_generation ON bazaar_command_receipt_heads(receipt_generation,operation_id);
INSERT INTO bazaar_command_receipt_heads(operation_id,receipt_generation)
SELECT o.operation_id,coalesce(max(r.recovery_generation),0)
FROM bazaar_game_outbox o LEFT JOIN bazaar_authority_receipts r USING(operation_id)
GROUP BY o.operation_id;
CREATE FUNCTION bazaar_command_receipt_head() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_TABLE_NAME='bazaar_game_outbox' THEN
        INSERT INTO bazaar_command_receipt_heads(operation_id) VALUES(NEW.operation_id);
    ELSE
        UPDATE bazaar_command_receipt_heads SET receipt_generation=greatest(receipt_generation,NEW.recovery_generation)
        WHERE operation_id=NEW.operation_id;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER bazaar_command_receipt_head AFTER INSERT ON bazaar_game_outbox FOR EACH ROW EXECUTE FUNCTION bazaar_command_receipt_head();
CREATE TRIGGER bazaar_command_receipt_head AFTER INSERT ON bazaar_authority_receipts FOR EACH ROW EXECUTE FUNCTION bazaar_command_receipt_head();
CREATE INDEX bazaar_operation_actor_created ON bazaar_game_outbox(actor_id,created_at);
CREATE INDEX bazaar_receipt_digest_generation ON bazaar_authority_receipts(receipt_digest,recovery_generation,source_authority);
