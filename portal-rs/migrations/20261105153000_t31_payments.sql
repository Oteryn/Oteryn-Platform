-- Account identities remain UUIDs until the accounts contract exists.
CREATE TABLE payment_intents (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL CHECK (account_id <> '00000000-0000-0000-0000-000000000000'),
    provider text NOT NULL CHECK (length(provider) BETWEEN 1 AND 128),
    provider_reference text NOT NULL CHECK (length(provider_reference) BETWEEN 1 AND 128),
    amount bigint NOT NULL CHECK (amount > 0),
    currency text NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    state text NOT NULL CHECK (state IN ('created','pending','succeeded','failed','refunded')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (provider, provider_reference)
);
CREATE INDEX payment_intents_account_idx ON payment_intents (account_id, created_at);

CREATE TABLE payment_events (
    id uuid PRIMARY KEY,
    provider text NOT NULL CHECK (length(provider) BETWEEN 1 AND 128),
    provider_event_id text NOT NULL CHECK (length(provider_event_id) BETWEEN 1 AND 128),
    intent_id uuid NOT NULL REFERENCES payment_intents(id),
    state text NOT NULL CHECK (state IN ('pending','succeeded','failed','refunded')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (provider, provider_event_id)
);
CREATE TABLE invoices (
    id uuid PRIMARY KEY,
    intent_id uuid NOT NULL UNIQUE REFERENCES payment_intents(id),
    account_id uuid NOT NULL,
    amount bigint NOT NULL CHECK (amount > 0),
    currency text NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
