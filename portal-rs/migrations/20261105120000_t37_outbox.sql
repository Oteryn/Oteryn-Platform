CREATE TABLE outbox (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    kind text NOT NULL CHECK (length(btrim(kind)) BETWEEN 1 AND 128),
    payload jsonb NOT NULL,
    idempotency_key text NOT NULL CHECK (length(btrim(idempotency_key)) BETWEEN 1 AND 512),
    state text NOT NULL DEFAULT 'pending' CHECK (state IN ('pending', 'running', 'done', 'dead')),
    attempts integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    max_attempts integer NOT NULL DEFAULT 5 CHECK (max_attempts BETWEEN 1 AND 100),
    run_after timestamptz NOT NULL DEFAULT now(),
    leased_until timestamptz,
    last_error text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (kind, idempotency_key),
    CHECK ((state = 'running') = (leased_until IS NOT NULL))
);
CREATE INDEX outbox_due ON outbox (run_after, id) WHERE state = 'pending';
CREATE INDEX outbox_leases ON outbox (leased_until) WHERE state = 'running';
