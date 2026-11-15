-- Short-lived staff attestations, bound to the authoritative ticket owner.
CREATE TABLE support_mfa_reset_evidence (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    ticket_id uuid NOT NULL,
    account_id uuid NOT NULL,
    verified_by uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    expires_at timestamptz NOT NULL DEFAULT clock_timestamp() + interval '1 hour',
    consumed_at timestamptz,
    CHECK (account_id <> verified_by)
);
CREATE INDEX support_mfa_reset_evidence_owner ON support_mfa_reset_evidence(account_id);
