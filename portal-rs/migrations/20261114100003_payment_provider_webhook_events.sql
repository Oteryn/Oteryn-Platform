-- OTERYN_PAYMENT_PROVIDER_WEBHOOK_V1: every verified provider event, keyed by
-- the provider's event ID, with the SHA-256 of its exact body and the outcome
-- it was given. A replay with the same body changes nothing; one with another
-- body is refused and audited. Additive: one new table, no existing row read.
CREATE TABLE payment_provider_webhook_events (
    event_id text PRIMARY KEY CHECK (length(event_id) BETWEEN 1 AND 128),
    body_sha256 bytea NOT NULL CHECK (length(body_sha256) = 32),
    event_type text NOT NULL CHECK (length(event_type) BETWEEN 1 AND 64),
    order_reference text CHECK (length(order_reference) BETWEEN 1 AND 128),
    outcome text NOT NULL CHECK (outcome IN ('applied', 'ignored', 'unknown_order', 'rejected', 'illegal_transition')),
    received_at timestamptz NOT NULL DEFAULT now()
);
