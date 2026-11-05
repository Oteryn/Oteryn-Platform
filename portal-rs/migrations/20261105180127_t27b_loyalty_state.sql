-- Rebuildable projection; Premium event history remains authoritative.
CREATE TABLE loyalty_state (
    account_id uuid PRIMARY KEY REFERENCES premium_authority(account_id),
    loyalty_points integer NOT NULL CHECK (loyalty_points >= 0),
    loyalty_revision bigint NOT NULL CHECK (loyalty_revision BETWEEN 1 AND 9007199254740991),
    computed_at timestamptz NOT NULL CHECK (isfinite(computed_at)),
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
);
