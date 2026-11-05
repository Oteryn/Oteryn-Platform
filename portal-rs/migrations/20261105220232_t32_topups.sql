-- Browser retries claim their operation before calling the payment provider.
-- An interrupted creation stays unresolved and must not create a second payment.
CREATE TABLE store_web_topups (
    operation_id uuid PRIMARY KEY,
    account_id uuid NOT NULL,
    package_key text NOT NULL CHECK (length(package_key) BETWEEN 1 AND 64),
    coins bigint NOT NULL CHECK (coins > 0),
    minor_units bigint NOT NULL CHECK (minor_units > 0),
    currency text NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    payment_intent_id uuid UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX store_web_topups_account ON store_web_topups(account_id, created_at DESC);
