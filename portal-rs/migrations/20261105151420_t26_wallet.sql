-- Account identifiers deliberately have no identity foreign key until accounts exist.
CREATE TABLE wallet_accounts (
    account_id uuid PRIMARY KEY,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

-- The operation key is global across all wallet mutations. Payload equality is
-- checked on replay, so a key cannot be reused for a different effect.
CREATE TABLE wallet_operations (
    id uuid PRIMARY KEY,
    kind text NOT NULL CHECK (kind IN ('transfer', 'adjustment', 'reserve', 'release', 'capture')),
    account_id uuid NOT NULL REFERENCES wallet_accounts(account_id),
    counterparty_id uuid REFERENCES wallet_accounts(account_id),
    amount bigint NOT NULL,
    reference_id uuid,
    reason text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE wallet_transfers (
    id uuid PRIMARY KEY REFERENCES wallet_operations(id),
    sender_id uuid NOT NULL REFERENCES wallet_accounts(account_id),
    recipient_id uuid NOT NULL REFERENCES wallet_accounts(account_id),
    amount bigint NOT NULL CHECK (amount > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (sender_id <> recipient_id)
);

CREATE TABLE wallet_adjustments (
    id uuid PRIMARY KEY REFERENCES wallet_operations(id),
    account_id uuid NOT NULL REFERENCES wallet_accounts(account_id),
    actor_id uuid NOT NULL,
    amount bigint NOT NULL CHECK (amount <> 0),
    reason text NOT NULL CHECK (length(btrim(reason)) BETWEEN 1 AND 1000),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE wallet_reservations (
    id uuid PRIMARY KEY REFERENCES wallet_operations(id),
    account_id uuid NOT NULL REFERENCES wallet_accounts(account_id),
    amount bigint NOT NULL CHECK (amount > 0),
    status text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'released', 'captured')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE wallet_entries (
    id uuid PRIMARY KEY,
    operation_id uuid NOT NULL REFERENCES wallet_operations(id),
    account_id uuid NOT NULL REFERENCES wallet_accounts(account_id),
    available_delta bigint NOT NULL,
    reserved_delta bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (operation_id, account_id),
    CHECK (available_delta <> 0 OR reserved_delta <> 0)
);
CREATE INDEX wallet_entries_history ON wallet_entries(account_id, id DESC);

CREATE FUNCTION wallet_reject_rewrite() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'wallet ledger records are immutable' USING ERRCODE = '55000';
END;
$$;
CREATE TRIGGER wallet_entries_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON wallet_entries FOR EACH STATEMENT EXECUTE FUNCTION wallet_reject_rewrite();
CREATE TRIGGER wallet_operations_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON wallet_operations FOR EACH STATEMENT EXECUTE FUNCTION wallet_reject_rewrite();
CREATE TRIGGER wallet_transfers_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON wallet_transfers FOR EACH STATEMENT EXECUTE FUNCTION wallet_reject_rewrite();
CREATE TRIGGER wallet_adjustments_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON wallet_adjustments FOR EACH STATEMENT EXECUTE FUNCTION wallet_reject_rewrite();
