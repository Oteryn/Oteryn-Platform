-- T-coin-native phase A: native coin purchases (sandbox) per Game contract
-- OTERYN_COIN_PURCHASE_V1. Additive only; existing OAuth grants keep game:ticket.
ALTER TABLE oauth_authorization_codes ADD COLUMN scope text NOT NULL DEFAULT 'game:ticket'
    CHECK (scope IN ('game:ticket', 'game:account-services'));
ALTER TABLE oauth_token_families ADD COLUMN scope text NOT NULL DEFAULT 'game:ticket'
    CHECK (scope IN ('game:ticket', 'game:account-services'));

-- A quote binds payer, optional character, package, exact coins, price,
-- environment and expiry. An order consumes it at most once.
CREATE TABLE coin_purchase_quotes (
    id uuid PRIMARY KEY,
    payer_account_id uuid NOT NULL CHECK (payer_account_id <> '00000000-0000-0000-0000-000000000000'),
    character_id uuid,
    package_id uuid NOT NULL,
    coins bigint NOT NULL CHECK (coins BETWEEN 1 AND 999999999999),
    minor_units bigint NOT NULL CHECK (minor_units BETWEEN 1 AND 9999999999),
    currency text NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    exponent smallint NOT NULL CHECK (exponent BETWEEN 0 AND 3),
    environment text NOT NULL CHECK (length(environment) BETWEEN 1 AND 32),
    expires_at timestamptz NOT NULL,
    consumed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX coin_purchase_quotes_payer ON coin_purchase_quotes (payer_account_id, created_at);

-- Request bindings are immutable and unique per payer. The payment intent ID is
-- reserved with the order and used as the provider creation key afterwards.
CREATE TABLE coin_purchase_orders (
    id uuid PRIMARY KEY,
    payer_account_id uuid NOT NULL CHECK (payer_account_id <> '00000000-0000-0000-0000-000000000000'),
    request_id uuid NOT NULL,
    quote_id uuid NOT NULL UNIQUE REFERENCES coin_purchase_quotes(id),
    package_id uuid NOT NULL,
    character_id uuid,
    coins bigint NOT NULL CHECK (coins BETWEEN 1 AND 999999999999),
    minor_units bigint NOT NULL CHECK (minor_units BETWEEN 1 AND 9999999999),
    currency text NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    exponent smallint NOT NULL CHECK (exponent BETWEEN 0 AND 3),
    environment text NOT NULL CHECK (length(environment) BETWEEN 1 AND 32),
    payment_intent_id uuid NOT NULL UNIQUE,
    revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0),
    state text NOT NULL CHECK (state IN ('awaiting_payment', 'processing', 'paid', 'failed', 'refunded', 'chargeback')),
    checkout_url text CHECK (length(checkout_url) BETWEEN 1 AND 2048),
    receipt_id uuid UNIQUE REFERENCES invoices(id),
    review_required boolean NOT NULL DEFAULT false,
    code text NOT NULL CHECK (char_length(code) BETWEEN 1 AND 96),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (payer_account_id, request_id),
    CHECK (
        (state = 'awaiting_payment' AND checkout_url IS NOT NULL AND receipt_id IS NULL)
        OR (state IN ('processing', 'failed') AND checkout_url IS NULL AND receipt_id IS NULL)
        OR (state IN ('paid', 'refunded', 'chargeback') AND checkout_url IS NULL AND receipt_id IS NOT NULL)
    )
);
CREATE INDEX coin_purchase_orders_payer ON coin_purchase_orders (payer_account_id, created_at DESC, id DESC);
