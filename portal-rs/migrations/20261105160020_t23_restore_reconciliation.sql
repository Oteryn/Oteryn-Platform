-- Receipts remain immutable history; account identifiers have no cross-domain FK.
CREATE TABLE character_restore_locks (id smallint PRIMARY KEY CHECK (id = 1));
INSERT INTO character_restore_locks VALUES (1);
CREATE TABLE character_restore_notices (
    restore_event_id uuid PRIMARY KEY,
    notice_json text NOT NULL,
    new_recovery_generation text COLLATE "C" NOT NULL CHECK (new_recovery_generation ~ '^[0-9]{20}$'),
    reopened_count bigint NOT NULL DEFAULT 0 CHECK (reopened_count >= 0),
    received_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    acknowledged_at timestamptz
);
CREATE TABLE character_restore_receipts (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    operation_id uuid NOT NULL REFERENCES character_commands(id),
    receipt_json text NOT NULL,
    recovery_generation text COLLATE "C",
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    superseded_at timestamptz
);
CREATE UNIQUE INDEX character_restore_current_receipt ON character_restore_receipts(operation_id) WHERE superseded_at IS NULL;
CREATE INDEX character_restore_generation ON character_restore_receipts(recovery_generation) WHERE superseded_at IS NULL;
CREATE TABLE character_restore_reopens (
    restore_event_id uuid NOT NULL REFERENCES character_restore_notices(restore_event_id),
    operation_id uuid NOT NULL REFERENCES character_commands(id),
    superseded_receipt_id bigint NOT NULL REFERENCES character_restore_receipts(id),
    new_receipt_id bigint REFERENCES character_restore_receipts(id),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (restore_event_id, operation_id)
);
CREATE INDEX character_restore_open ON character_restore_reopens(operation_id) WHERE new_receipt_id IS NULL;
CREATE TABLE character_restore_queue (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    restore_event_id uuid,
    operation_id uuid,
    account_id uuid,
    reason text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    resolved_at timestamptz,
    UNIQUE NULLS NOT DISTINCT (restore_event_id, operation_id, reason)
);

CREATE FUNCTION character_restore_generation(raw text) RETURNS text LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE value jsonb; generation text;
BEGIN
    value := raw::jsonb;
    IF jsonb_typeof(value->'recovery_generation') <> 'string' THEN RETURN NULL; END IF;
    generation := value->>'recovery_generation';
    IF generation !~ '^[1-9][0-9]{0,19}$' OR generation::numeric > 18446744073709551615 THEN RETURN NULL; END IF;
    RETURN lpad(generation, 20, '0');
EXCEPTION WHEN OTHERS THEN RETURN NULL;
END;
$$;
INSERT INTO character_restore_receipts(operation_id, receipt_json, recovery_generation)
SELECT id, receipt_json, character_restore_generation(receipt_json) FROM character_commands WHERE receipt_json IS NOT NULL;

-- Receipt publication enters through the composed Rust exchange, which holds
-- the shared restore lock before T22 takes an intent lock. This trigger adds
-- history and answers reopens in T22's transaction, without a second lock order.
CREATE FUNCTION character_restore_receipt_history() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE generation text; receipt_id bigint;
BEGIN
    IF OLD.receipt_json IS NOT DISTINCT FROM NEW.receipt_json THEN RETURN NEW; END IF;
    IF OLD.receipt_json IS NOT NULL THEN
        IF NEW.receipt_json IS NOT NULL THEN RAISE EXCEPTION 'current receipt is immutable'; END IF;
        UPDATE character_restore_receipts SET superseded_at=clock_timestamp()
        WHERE operation_id=OLD.id AND superseded_at IS NULL;
        RETURN NEW;
    END IF;
    IF NEW.receipt_json IS NULL THEN RETURN NEW; END IF;
    generation := character_restore_generation(NEW.receipt_json);
    IF generation IS NULL OR EXISTS (
        SELECT 1 FROM character_restore_notices WHERE new_recovery_generation > generation
    ) THEN RAISE EXCEPTION 'receipt lost to restore'; END IF;
    INSERT INTO character_restore_receipts(operation_id,receipt_json,recovery_generation)
    VALUES (NEW.id,NEW.receipt_json,generation) RETURNING id INTO receipt_id;
    INSERT INTO character_restore_queue(restore_event_id,operation_id,account_id,reason)
    SELECT reopen.restore_event_id,NEW.id,NEW.account_id,'character_authority.restore_outcome_reversed'
    FROM character_restore_reopens reopen JOIN character_restore_receipts prior_receipt ON prior_receipt.id=reopen.superseded_receipt_id
    WHERE reopen.operation_id=NEW.id AND reopen.new_receipt_id IS NULL
      AND prior_receipt.receipt_json::jsonb->>'outcome'='COMMITTED' AND NEW.receipt_json::jsonb->>'outcome'='REJECTED'
    ON CONFLICT DO NOTHING;
    UPDATE character_restore_reopens SET new_receipt_id=receipt_id
    WHERE operation_id=NEW.id AND new_receipt_id IS NULL;
    RETURN NEW;
END;
$$;
CREATE TRIGGER character_restore_receipt_history BEFORE UPDATE OF receipt_json ON character_commands
FOR EACH ROW EXECUTE FUNCTION character_restore_receipt_history();
