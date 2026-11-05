-- Platform intent storage only; no game database access or production activation.
CREATE TABLE admin_control_agents (
    agent_id text PRIMARY KEY CHECK (length(btrim(agent_id)) BETWEEN 1 AND 128),
    build text NOT NULL CHECK (length(btrim(build)) BETWEEN 1 AND 256),
    catalog_digest bytea NOT NULL CHECK (octet_length(catalog_digest) = 32),
    advertised_operations jsonb NOT NULL CHECK (jsonb_typeof(advertised_operations) = 'array'),
    served_worlds text[] NOT NULL,
    served_nodes text[] NOT NULL,
    owner_epoch bigint NOT NULL CHECK (owner_epoch > 0),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
-- Connectivity is derived at read time; it is never persisted as a boolean.
CREATE TABLE admin_control_requests (
    request_id uuid PRIMARY KEY,
    admin_operation_id uuid NOT NULL UNIQUE,
    operation text NOT NULL CHECK (length(btrim(operation)) BETWEEN 1 AND 128),
    operation_version integer NOT NULL CHECK (operation_version > 0),
    kind text NOT NULL CHECK (kind IN ('command', 'query')),
    target jsonb NOT NULL CHECK (jsonb_typeof(target) = 'object'),
    params jsonb NOT NULL CHECK (jsonb_typeof(params) = 'object'),
    params_digest bytea NOT NULL CHECK (octet_length(params_digest) = 32),
    dry_run boolean NOT NULL,
    risk_class text NOT NULL CHECK (risk_class IN ('R0', 'R1', 'R2', 'R3', 'R4')),
    actor uuid NOT NULL,
    reason jsonb NOT NULL CHECK (jsonb_typeof(reason) = 'object'),
    approvals jsonb NOT NULL CHECK (jsonb_typeof(approvals) = 'array'),
    issued_at timestamptz NOT NULL,
    expires_at timestamptz NOT NULL CHECK (expires_at > issued_at),
    nonce bytea NOT NULL CHECK (octet_length(nonce) = 16),
    signer_key_id text NOT NULL CHECK (length(btrim(signer_key_id)) BETWEEN 1 AND 128),
    canonical_envelope bytea NOT NULL CHECK (octet_length(canonical_envelope) BETWEEN 1 AND 1048576),
    signature bytea NOT NULL CHECK (octet_length(signature) = 64),
    status text NOT NULL DEFAULT 'queued' CHECK (status IN (
        'queued', 'delivered', 'accepted', 'in_progress', 'succeeded',
        'failed', 'rejected', 'expired', 'cancelled'
    )),
    agent_id text REFERENCES admin_control_agents(agent_id),
    delivered_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (admin_operation_id, params_digest),
    CHECK (kind <> 'query' OR admin_operation_id = request_id),
    CHECK ((agent_id IS NULL) = (delivered_at IS NULL))
);
-- The single-column uniqueness above also refuses a different digest under the same id.
CREATE INDEX admin_control_pending_idx ON admin_control_requests (issued_at, request_id)
    WHERE status = 'queued';
CREATE INDEX admin_control_expiry_idx ON admin_control_requests (expires_at)
    WHERE status IN ('queued', 'delivered');
CREATE INDEX admin_control_console_idx ON admin_control_requests (created_at DESC, request_id);
CREATE TABLE admin_control_results (
    result_id uuid PRIMARY KEY DEFAULT uuidv7(),
    request_id uuid NOT NULL REFERENCES admin_control_requests(request_id),
    agent_id text NOT NULL REFERENCES admin_control_agents(agent_id),
    game_ledger_id text,
    error_code text CHECK (error_code IS NULL OR error_code ~ '^ADM_[A-Z0-9_]{1,100}$'),
    status text NOT NULL CHECK (status IN (
        'accepted', 'in_progress', 'succeeded', 'failed', 'rejected', 'expired', 'cancelled'
    )),
    payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    received_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX admin_control_results_request_idx ON admin_control_results (request_id, received_at, result_id);
CREATE FUNCTION reject_admin_control_result_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'admin_control_results is append-only' USING ERRCODE = '42501';
END;
$$;
CREATE TRIGGER admin_control_results_append_only
    BEFORE UPDATE OR DELETE OR TRUNCATE ON admin_control_results
    FOR EACH STATEMENT EXECUTE FUNCTION reject_admin_control_result_mutation();
CREATE TABLE admin_approvals (
    approval_id uuid PRIMARY KEY,
    request_id uuid NOT NULL REFERENCES admin_control_requests(request_id),
    approver uuid NOT NULL,
    approved_at timestamptz NOT NULL,
    preview_digest bytea NOT NULL CHECK (octet_length(preview_digest) = 32),
    decision text NOT NULL CHECK (decision IN ('approved', 'rejected')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (request_id, approver)
);
