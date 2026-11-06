-- Additive consumer records only. Game custody and balances are never stored as authority.
-- Rollback: turn off PLAT3, preserve these immutable records and reconcile outstanding work.
CREATE TABLE bazaar_authority_receipts (
    operation_id uuid NOT NULL REFERENCES bazaar_game_outbox(operation_id),
    recovery_generation numeric(20,0) NOT NULL CHECK (recovery_generation BETWEEN 1 AND 18446744073709551615),
    source_authority text NOT NULL,
    intent_digest text NOT NULL CHECK (intent_digest ~ '^[0-9a-f]{64}$'),
    receipt_digest text NOT NULL CHECK (receipt_digest ~ '^[0-9a-f]{64}$'),
    receipt text NOT NULL CHECK (octet_length(receipt) BETWEEN 1 AND 270336),
    outcome text NOT NULL CHECK (outcome IN ('COMMITTED','REJECTED')),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(operation_id,recovery_generation)
);
CREATE TRIGGER bazaar_authority_receipts_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
ON bazaar_authority_receipts FOR EACH STATEMENT EXECUTE FUNCTION bazaar_append_only();
CREATE TABLE bazaar_authority_conflicts (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    operation_id uuid NOT NULL,
    incoming_digest text NOT NULL CHECK (incoming_digest ~ '^[0-9a-f]{64}$'),
    kind text NOT NULL CHECK (kind IN ('RECEIPT','DECISION')),
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE bazaar_authority_decisions (
    event_id uuid PRIMARY KEY,
    envelope_semantics bytea NOT NULL,
    transaction_id uuid NOT NULL,
    ordinal integer NOT NULL CHECK (ordinal>0),
    event_count integer NOT NULL CHECK (event_count>=ordinal),
    operation_id uuid NOT NULL,
    recovery_generation numeric(20,0) NOT NULL,
    source_authority text NOT NULL,
    payload bytea NOT NULL CHECK (octet_length(payload) BETWEEN 1 AND 256),
    payload_digest bytea NOT NULL CHECK (octet_length(payload_digest)=32),
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(transaction_id,ordinal),
    FOREIGN KEY(operation_id,recovery_generation) REFERENCES bazaar_authority_receipts(operation_id,recovery_generation)
);
CREATE TRIGGER bazaar_authority_decisions_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
ON bazaar_authority_decisions FOR EACH STATEMENT EXECUTE FUNCTION bazaar_append_only();
-- Platform coin holds. Wallet entries remain the sole balance authority.
CREATE TABLE bazaar_coin_holds (
    id uuid NOT NULL,
    account_id uuid NOT NULL REFERENCES wallet_accounts(account_id),
    amount bigint NOT NULL CHECK (amount>=0),
    settlement_id uuid,
    completed boolean NOT NULL DEFAULT false,
    PRIMARY KEY(id,account_id)
);
CREATE TABLE bazaar_coin_dispositions (
    settlement_id uuid PRIMARY KEY REFERENCES bazaar_settlements(id),
    binding text NOT NULL,
    completed boolean NOT NULL DEFAULT false
);
CREATE TABLE bazaar_authority_resync (
    projection_epoch numeric(20,0) NOT NULL,
    recovery_generation numeric(20,0) NOT NULL,
    completed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(projection_epoch,recovery_generation)
);
CREATE TABLE bazaar_authority_dispatch (
    operation_id uuid PRIMARY KEY REFERENCES bazaar_game_outbox(operation_id),
    projection_epoch numeric(20,0) NOT NULL,
    recovery_generation numeric(20,0) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TRIGGER bazaar_authority_dispatch_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
ON bazaar_authority_dispatch FOR EACH STATEMENT EXECUTE FUNCTION bazaar_append_only();
CREATE TRIGGER bazaar_authority_conflicts_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
ON bazaar_authority_conflicts FOR EACH STATEMENT EXECUTE FUNCTION bazaar_append_only();
