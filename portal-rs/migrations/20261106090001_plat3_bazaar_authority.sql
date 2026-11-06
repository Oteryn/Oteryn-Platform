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
    settlement_id uuid PRIMARY KEY REFERENCES bazaar_settlements(id) DEFERRABLE INITIALLY DEFERRED,
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
-- Runtime orchestration metadata; these are requests, never Game ownership truth.
CREATE TABLE bazaar_authority_assets (
    listing_id uuid PRIMARY KEY REFERENCES bazaar_listings(id),
    request_id uuid NOT NULL,
    world_id uuid NOT NULL,
    seller_character_id uuid NOT NULL,
    asset_id uuid NOT NULL,
    expected_revision numeric(20,0) NOT NULL CHECK(expected_revision BETWEEN 1 AND 18446744073709551615),
    session_hash bytea NOT NULL CHECK(octet_length(session_hash)=32),
    definition_key text,
    definition_revision text,
    quantity bigint NOT NULL CHECK(quantity BETWEEN 1 AND 4294967295),
    publication_consent boolean NOT NULL CHECK(publication_consent),
    input_binding text NOT NULL,
    refusal_code text,
    cancel_requested boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(request_id)
);
CREATE TABLE bazaar_authority_destinations (
    listing_id uuid NOT NULL REFERENCES bazaar_listings(id),
    buyer uuid NOT NULL,
    character_id uuid NOT NULL,
    PRIMARY KEY(listing_id,buyer)
);
CREATE TABLE bazaar_authority_transfers (
    operation_id uuid PRIMARY KEY,
    listing_id uuid NOT NULL REFERENCES bazaar_listings(id),
    purpose text NOT NULL CHECK(purpose IN ('BAZAAR_LISTING_ESCROW','BAZAAR_SETTLEMENT','BAZAAR_ESCROW_RETURN')),
    decision_id uuid NOT NULL,
    source_revision bigint NOT NULL CHECK(source_revision>0),
    intent text NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 4096),
    intent_digest text NOT NULL CHECK(intent_digest ~ '^[0-9a-f]{64}$'),
    binding text NOT NULL,
    projection_epoch numeric(20,0) NOT NULL,
    recovery_generation numeric(20,0) NOT NULL,
    UNIQUE(listing_id,purpose)
);
CREATE TRIGGER bazaar_authority_transfers_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
ON bazaar_authority_transfers FOR EACH STATEMENT EXECUTE FUNCTION bazaar_append_only();
CREATE TABLE bazaar_authority_snapshot_keys (
    kid text PRIMARY KEY,
    public_key bytea NOT NULL CHECK(octet_length(public_key)=32),
    revoked boolean NOT NULL DEFAULT false
);

CREATE TABLE bazaar_authority_bid_requests (
    request_id uuid PRIMARY KEY,
    listing_id uuid NOT NULL REFERENCES bazaar_listings(id),
    buyer uuid NOT NULL,
    character_id uuid NOT NULL,
    maximum bigint NOT NULL CHECK(maximum>0)
);
CREATE TRIGGER bazaar_authority_bid_requests_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
ON bazaar_authority_bid_requests FOR EACH STATEMENT EXECUTE FUNCTION bazaar_append_only();
-- Gold rows are commercial decisions and receipt references, never a bank balance.
CREATE TABLE bazaar_authority_gold_bids (
    request_id uuid PRIMARY KEY,
    listing_id uuid NOT NULL REFERENCES bazaar_listings(id),
    buyer uuid NOT NULL,
    character_id uuid NOT NULL,
    maximum bigint NOT NULL CHECK(maximum BETWEEN 2 AND 999999999999),
    hold_operation_id uuid NOT NULL REFERENCES bazaar_game_outbox(operation_id),
    release_operation_id uuid REFERENCES bazaar_game_outbox(operation_id),
    phase text NOT NULL DEFAULT 'pending' CHECK(phase IN ('pending','accepted','refused','releasing'))
);
CREATE UNIQUE INDEX bazaar_authority_gold_one_pending ON bazaar_authority_gold_bids(listing_id) WHERE phase IN ('pending','releasing');
CREATE TABLE bazaar_authority_gold_sales (
    id uuid PRIMARY KEY,
    listing_id uuid NOT NULL UNIQUE REFERENCES bazaar_listings(id),
    buyer uuid NOT NULL,
    seller uuid NOT NULL CHECK(seller<>buyer),
    gross_amount bigint NOT NULL CHECK(gross_amount BETWEEN 2 AND 999999999999),
    fee_amount bigint NOT NULL CHECK(fee_amount=(gross_amount*250+9999)/10000),
    seller_amount bigint NOT NULL CHECK(seller_amount=gross_amount-fee_amount AND seller_amount>0),
    hold_id uuid NOT NULL,
    hold_revision numeric(20,0) NOT NULL CHECK(hold_revision BETWEEN 1 AND 18446744073709551615),
    completed_at timestamptz
);

CREATE TABLE bazaar_authority_browser_requests (
    request_id uuid PRIMARY KEY,
    listing_id uuid NOT NULL REFERENCES bazaar_listings(id),
    actor uuid NOT NULL REFERENCES accounts(id),
    action text NOT NULL CHECK(action IN ('resume','cancel')),
    created_at timestamptz NOT NULL DEFAULT now()
);
