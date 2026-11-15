-- An audited terminal compensation is distinct from a successful sale. Preserve
-- the sale, rejected delivery, committed return and financial records unchanged.
CREATE TABLE bazaar_authority_compensations (
    listing_id uuid PRIMARY KEY REFERENCES bazaar_listings(id),
    return_operation_id uuid NOT NULL REFERENCES bazaar_game_outbox(operation_id),
    recovery_generation numeric(20,0) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TRIGGER bazaar_compensations_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
ON bazaar_authority_compensations FOR EACH STATEMENT EXECUTE FUNCTION bazaar_append_only();

CREATE OR REPLACE FUNCTION bazaar_state_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.state <> OLD.state AND NOT (
        (OLD.state = 'draft' AND NEW.state IN ('active', 'cancelled')) OR
        (OLD.state = 'active' AND NEW.state IN ('matched', 'awaiting_delivery', 'cancelled', 'expired')) OR
        (OLD.state = 'matched' AND NEW.state = 'awaiting_delivery') OR
        (OLD.state = 'awaiting_delivery' AND NEW.state = 'completed') OR
        (OLD.state = 'awaiting_delivery' AND NEW.state = 'cancelled'
            AND EXISTS (SELECT 1 FROM bazaar_authority_compensations c WHERE c.listing_id=OLD.id))
    ) THEN
        RAISE EXCEPTION 'illegal bazaar transition';
    END IF;
    RETURN NEW;
END;
$$;
