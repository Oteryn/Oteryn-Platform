CREATE TABLE sanctions (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL REFERENCES accounts(id),
    category text NOT NULL CHECK (category IN ('warning', 'restriction', 'suspension')),
    status text NOT NULL CHECK (status IN ('pending', 'active', 'expired', 'revoked')),
    public_reason text NOT NULL CHECK (length(btrim(public_reason)) BETWEEN 1 AND 4000),
    moderator_notes text CHECK (length(moderator_notes) <= 8000),
    effective_at timestamptz NOT NULL,
    expires_at timestamptz CHECK (expires_at > effective_at),
    created_by uuid NOT NULL REFERENCES accounts(id),
    approved_by uuid REFERENCES accounts(id),
    acknowledged_at timestamptz,
    lock_version bigint NOT NULL DEFAULT 1 CHECK (lock_version > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (approved_by IS NULL OR approved_by <> created_by),
    CHECK (expires_at IS NOT NULL OR status <> 'active' OR approved_by IS NOT NULL)
);
CREATE INDEX sanctions_account_active ON sanctions(account_id, effective_at)
    WHERE status = 'active';
