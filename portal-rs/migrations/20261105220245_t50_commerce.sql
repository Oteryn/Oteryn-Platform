-- Staff intent and approval history; domain ledgers remain the financial authority.
CREATE TABLE commerce_operations (
    id uuid PRIMARY KEY,
    actor_id uuid NOT NULL,
    target_id uuid NOT NULL,
    command jsonb NOT NULL,
    reason_sha256 text NOT NULL CHECK (length(reason_sha256) = 64),
    requires_approval boolean NOT NULL,
    approved_by uuid CHECK (approved_by <> actor_id),
    review jsonb,
    result jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (result IS NULL OR NOT requires_approval OR approved_by IS NOT NULL)
);
CREATE INDEX commerce_operations_pending ON commerce_operations (created_at, id)
    WHERE result IS NULL;
-- A durable queue, never a claim that a provider has actually refunded money.
CREATE TABLE commerce_refunds (
    operation_id uuid PRIMARY KEY REFERENCES commerce_operations(id),
    intent_id uuid NOT NULL UNIQUE,
    account_id uuid NOT NULL,
    amount bigint NOT NULL CHECK (amount > 0),
    currency text NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    provider text NOT NULL,
    provider_reference text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
