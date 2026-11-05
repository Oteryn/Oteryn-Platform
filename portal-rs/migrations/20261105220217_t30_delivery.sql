-- Shared lock contract: restore owners lock row 1 exclusively before intents;
-- receipt publishers lock it shared. Missing rows deny exchange mutations.
CREATE TABLE character_restore_locks (id integer PRIMARY KEY CHECK (id=1));
INSERT INTO character_restore_locks(id) VALUES(1);
CREATE TABLE store_delivery_fence (
    id integer PRIMARY KEY CHECK(id=1),
    new_recovery_generation numeric(20,0) NOT NULL DEFAULT 0 CHECK(new_recovery_generation BETWEEN 0 AND 18446744073709551615),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
INSERT INTO store_delivery_fence(id) VALUES(1);
CREATE TABLE store_receipts (
    receipt_id uuid PRIMARY KEY,
    issuer_authority text NOT NULL CHECK(issuer_authority='OTERYN_PLATFORM_STORE_AUTHORITY'),
    operation_id uuid NOT NULL REFERENCES store_operations,
    operation_kind text NOT NULL CHECK(operation_kind IN ('DELIVER','CLAIM','REVOKE_UNCLAIMED','RECLAIM','FLAG')),
    receipt_bytes bytea NOT NULL CHECK(octet_length(receipt_bytes) BETWEEN 1 AND 1024),
    receipt_digest text NOT NULL CHECK(receipt_digest ~ '^[0-9a-f]{64}$'),
    binding_digest text NOT NULL CHECK(binding_digest ~ '^[0-9a-f]{64}$'),
    outcome text NOT NULL CHECK(outcome IN ('COMMITTED','REJECTED')),
    result_code text NOT NULL,
    recovery_generation numeric(20,0) NOT NULL CHECK(recovery_generation BETWEEN 1 AND 18446744073709551615),
    decided_at numeric(20,0) NOT NULL CHECK(decided_at BETWEEN 1 AND 18446744073709551615),
    superseded_by_restore_event_id uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX store_receipts_current ON store_receipts(issuer_authority,operation_id) WHERE superseded_by_restore_event_id IS NULL;
CREATE TRIGGER store_receipts_binding_immutable BEFORE UPDATE OF receipt_id,issuer_authority,operation_id,operation_kind,receipt_bytes,receipt_digest,binding_digest,outcome,result_code,recovery_generation,decided_at
    ON store_receipts FOR EACH STATEMENT EXECUTE FUNCTION wallet_reject_rewrite();
CREATE TRIGGER store_receipts_no_delete BEFORE DELETE OR TRUNCATE ON store_receipts FOR EACH STATEMENT EXECUTE FUNCTION wallet_reject_rewrite();
CREATE TABLE store_reconcile_state (
    restore_event_id uuid NOT NULL,
    operation_id uuid NOT NULL REFERENCES store_operations,
    new_recovery_generation numeric(20,0) NOT NULL CHECK(new_recovery_generation BETWEEN 1 AND 18446744073709551615),
    resolved_receipt_id uuid REFERENCES store_receipts,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(restore_event_id,operation_id)
);
