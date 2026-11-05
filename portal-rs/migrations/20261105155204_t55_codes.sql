CREATE TABLE code_batches (
    id uuid PRIMARY KEY,
    kind text NOT NULL CHECK (kind IN ('coins','premium_days','item_grant')),
    amount bigint NOT NULL CHECK (amount > 0),
    max_per_code integer NOT NULL CHECK (max_per_code > 0),
    max_per_account integer NOT NULL CHECK (max_per_account > 0),
    valid_from timestamptz NOT NULL,
    valid_until timestamptz NOT NULL,
    created_by uuid NOT NULL,
    note text NOT NULL CHECK (length(note) BETWEEN 1 AND 500),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (isfinite(valid_from) AND isfinite(valid_until) AND valid_from < valid_until),
    CHECK (kind <> 'premium_days' OR amount <= 366)
);
CREATE TABLE gift_codes (
    id uuid PRIMARY KEY,
    batch_id uuid NOT NULL REFERENCES code_batches(id),
    code_hash bytea NOT NULL UNIQUE CHECK (octet_length(code_hash) = 32),
    support_prefix varchar(4) NOT NULL CHECK (length(support_prefix) = 4),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE code_redemptions (
    id uuid PRIMARY KEY,
    code_id uuid NOT NULL REFERENCES gift_codes(id),
    account_id uuid NOT NULL,
    character_ref uuid,
    idempotency_key uuid NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX code_redemptions_limits ON code_redemptions(code_id, account_id);
CREATE TABLE code_game_commands (
    redemption_id uuid PRIMARY KEY REFERENCES code_redemptions(id),
    character_ref uuid NOT NULL,
    amount bigint NOT NULL CHECK (amount > 0),
    status text NOT NULL DEFAULT 'pending' CHECK (status = 'pending'),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE FUNCTION code_history_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'code history is immutable';
END;
$$;
CREATE TRIGGER code_redemptions_immutable BEFORE UPDATE OR DELETE ON code_redemptions
    FOR EACH ROW EXECUTE FUNCTION code_history_immutable();
CREATE TRIGGER gift_codes_immutable BEFORE UPDATE OR DELETE ON gift_codes
    FOR EACH ROW EXECUTE FUNCTION code_history_immutable();
CREATE TRIGGER code_batches_immutable BEFORE UPDATE OR DELETE ON code_batches
    FOR EACH ROW EXECUTE FUNCTION code_history_immutable();
