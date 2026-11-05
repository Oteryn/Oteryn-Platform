-- Platform lifecycle only. Game account and character disposition awaits its contract.
CREATE TABLE account_terminations (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL REFERENCES accounts(id),
    scheduled_for timestamptz NOT NULL,
    cancelled_at timestamptz,
    finalized_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (scheduled_for >= created_at + interval '1 day'),
    CHECK (scheduled_for <= created_at + interval '90 days'),
    CHECK (cancelled_at IS NULL OR finalized_at IS NULL)
);
CREATE UNIQUE INDEX account_terminations_pending ON account_terminations(account_id)
    WHERE cancelled_at IS NULL AND finalized_at IS NULL;
CREATE INDEX account_terminations_due ON account_terminations(scheduled_for, account_id)
    WHERE cancelled_at IS NULL AND finalized_at IS NULL;
CREATE TABLE account_termination_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    termination_id uuid NOT NULL REFERENCES account_terminations(id),
    kind text NOT NULL CHECK (kind IN ('requested', 'cancelled', 'finalized')),
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (termination_id, kind)
);
