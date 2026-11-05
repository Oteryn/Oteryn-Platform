-- Account ids deliberately have no foreign key until identity owns accounts.
CREATE TABLE premium_authority (
    account_id uuid PRIMARY KEY,
    authority_revision bigint NOT NULL DEFAULT 0 CHECK (authority_revision BETWEEN 0 AND 9007199254740991),
    last_issued_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE premium_ledger (
    request_id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES premium_authority(account_id),
    actor_id uuid NOT NULL,
    event_type text NOT NULL CHECK (event_type IN ('GRANT', 'REVOKE')),
    duration_days integer,
    reason_sha256 bytea NOT NULL CHECK (octet_length(reason_sha256) = 32),
    entitlement_id uuid NOT NULL,
    lifecycle_revision bigint NOT NULL CHECK (lifecycle_revision BETWEEN 1 AND 9007199254740991),
    delta_seconds bigint NOT NULL,
    effective_from timestamptz NOT NULL,
    effective_until timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (effective_from < effective_until),
    CHECK (isfinite(effective_from) AND isfinite(effective_until)),
    CHECK (effective_from = date_trunc('second', effective_from)
        AND effective_until = date_trunc('second', effective_until)),
    CHECK ((event_type = 'GRANT' AND duration_days BETWEEN 1 AND 366)
        OR (event_type = 'REVOKE' AND duration_days IS NULL AND delta_seconds <= 0)),
    UNIQUE (account_id, lifecycle_revision)
);
CREATE TABLE premium_audit (
    request_id uuid PRIMARY KEY REFERENCES premium_ledger(request_id),
    actor_id uuid NOT NULL,
    account_id uuid NOT NULL,
    event_name text NOT NULL CHECK (event_name IN ('products.premium_granted', 'products.premium_revoked')),
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE FUNCTION premium_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'premium history is immutable';
END;
$$;
CREATE TRIGGER premium_ledger_immutable BEFORE UPDATE OR DELETE ON premium_ledger
    FOR EACH ROW EXECUTE FUNCTION premium_immutable();
CREATE TRIGGER premium_audit_immutable BEFORE UPDATE OR DELETE ON premium_audit
    FOR EACH ROW EXECUTE FUNCTION premium_immutable();
