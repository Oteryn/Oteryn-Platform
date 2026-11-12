-- T-coin-native phase B: refunds, chargebacks and reversal of native coin
-- purchases (OTERYN_COIN_PURCHASE_V1, Platform prerequisites 3 and 4).
-- Additive: two new tables; the intent and event state checks only widen.

-- A provider chargeback is its own intent and event state (succeeded or
-- refunded -> chargeback). Every existing row keeps satisfying the new check.
ALTER TABLE payment_intents DROP CONSTRAINT payment_intents_state_check;
ALTER TABLE payment_intents ADD CONSTRAINT payment_intents_state_check
    CHECK (state IN ('created', 'pending', 'succeeded', 'failed', 'refunded', 'chargeback'));
ALTER TABLE payment_events DROP CONSTRAINT payment_events_state_check;
ALTER TABLE payment_events ADD CONSTRAINT payment_events_state_check
    CHECK (state IN ('pending', 'succeeded', 'failed', 'refunded', 'chargeback'));

-- At most one full reversal per order (the order row lock serializes it). Its
-- id is the wallet operation key of the debit. `debited` reverses the whole
-- credit, `held` debits nothing because the unreserved balance no longer covers
-- it (review + purchase hold), `fenced` arrived before any credit and blocks a
-- later one.
CREATE TABLE coin_purchase_reversals (
    id uuid PRIMARY KEY,
    order_id uuid NOT NULL UNIQUE REFERENCES coin_purchase_orders(id),
    payer_account_id uuid NOT NULL CHECK (payer_account_id <> '00000000-0000-0000-0000-000000000000'),
    kind text NOT NULL CHECK (kind IN ('refund', 'chargeback')),
    provider text NOT NULL CHECK (length(provider) BETWEEN 1 AND 128),
    provider_event_id text NOT NULL CHECK (length(provider_event_id) BETWEEN 1 AND 128),
    coins bigint NOT NULL CHECK (coins BETWEEN 1 AND 999999999999),
    debited bigint NOT NULL CHECK (debited = 0 OR debited = coins),
    outcome text NOT NULL CHECK (outcome IN ('debited', 'held', 'fenced')),
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK ((outcome = 'debited') = (debited > 0)),
    UNIQUE (provider, provider_event_id)
);
CREATE INDEX coin_purchase_reversals_payer ON coin_purchase_reversals (payer_account_id, created_at);

-- Insufficient unreserved balance at a reversal freezes new purchasing for the
-- payer until staff resolve it. One row per account; the first cause is kept.
CREATE TABLE coin_purchase_holds (
    account_id uuid PRIMARY KEY CHECK (account_id <> '00000000-0000-0000-0000-000000000000'),
    order_id uuid NOT NULL REFERENCES coin_purchase_orders(id),
    reason text NOT NULL CHECK (reason IN ('reversal_insufficient_balance')),
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX coin_purchase_holds_order ON coin_purchase_holds (order_id);
