-- Additive, dormant PLAT3 candidate state. Rollback disables the adapter and retains
-- authoritative command/replay evidence; it must never reopen a Game account.
CREATE TABLE plat3_closure_lifecycle (
    account_id uuid PRIMARY KEY REFERENCES accounts(id),
    generation bigint NOT NULL CHECK (generation > 0)
);
CREATE TABLE plat3_closure_work (
    termination_id uuid PRIMARY KEY REFERENCES account_terminations(id),
    account_id uuid NOT NULL REFERENCES accounts(id),
    generation bigint NOT NULL CHECK (generation > 0),
    close_operation_id uuid UNIQUE,
    invalid boolean NOT NULL DEFAULT false,
    resolution text CHECK (resolution IN ('CANCEL','FINALIZE')),
    UNIQUE(account_id, generation)
);
CREATE SEQUENCE plat3_closure_source_revision AS bigint MINVALUE 1 NO CYCLE;
CREATE TABLE plat3_closure_commands (
    operation_id uuid PRIMARY KEY,
    termination_id uuid NOT NULL REFERENCES plat3_closure_work(termination_id),
    source_revision bigint NOT NULL UNIQUE CHECK(source_revision>0),
    command text NOT NULL CHECK (command IN ('CloseAccount','ResolveAccountClosure')),
    intent bytea NOT NULL CHECK (octet_length(intent) BETWEEN 1 AND 4096),
    intent_sha256 bytea NOT NULL CHECK (octet_length(intent_sha256)=32),
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX plat3_closure_one_resolution ON plat3_closure_commands(termination_id) WHERE command='ResolveAccountClosure';
CREATE TABLE plat3_closure_receipts (
    operation_id uuid PRIMARY KEY REFERENCES plat3_closure_commands(operation_id),
    receipt bytea NOT NULL CHECK (octet_length(receipt) BETWEEN 1 AND 2048),
    signature bytea NOT NULL CHECK (octet_length(signature)=64),
    receipt_sha256 bytea NOT NULL CHECK (octet_length(receipt_sha256)=32),
    recovery_generation text NOT NULL CHECK (recovery_generation ~ '^[1-9][0-9]{0,19}$' AND recovery_generation::numeric<=18446744073709551615),
    received_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE plat3_closure_feed (
    id smallint PRIMARY KEY CHECK (id=1),
    highest_epoch text NOT NULL DEFAULT '0',
    watermark bytea,
    signature bytea CHECK (octet_length(signature)=64),
    invalid boolean NOT NULL DEFAULT false
);
INSERT INTO plat3_closure_feed(id) VALUES(1);
CREATE TABLE plat3_closure_snapshots (
    account_id uuid PRIMARY KEY REFERENCES accounts(id),
    snapshot bytea NOT NULL CHECK (octet_length(snapshot) BETWEEN 1 AND 2048),
    signature bytea NOT NULL CHECK (octet_length(signature)=64),
    epoch text NOT NULL,
    revision text NOT NULL,
    invalid boolean NOT NULL DEFAULT false
);
-- Event type 7, schema revision 1, PLAT3_AUTHORITY_AUDIT_RETENTION_V1 (P90D).
-- Payload only, awaiting the existing foundation envelope publisher; no new envelope.
-- This finite audit is separate from permanent operation/generation replay state.
CREATE TABLE plat3_closure_audit_outbox (
    operation_id uuid PRIMARY KEY REFERENCES plat3_closure_commands(operation_id),
    event_type integer NOT NULL DEFAULT 7 CHECK(event_type=7),
    schema_revision integer NOT NULL DEFAULT 1 CHECK(schema_revision=1),
    payload bytea NOT NULL,
    expires_at timestamptz NOT NULL DEFAULT now()+interval '90 days'
);
CREATE FUNCTION plat3_closure_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'immutable account closure evidence'; END;
$$;
CREATE TRIGGER plat3_closure_command_immutable BEFORE UPDATE OR DELETE ON plat3_closure_commands
    FOR EACH ROW EXECUTE FUNCTION plat3_closure_immutable();
CREATE TRIGGER plat3_closure_receipt_immutable BEFORE UPDATE OR DELETE ON plat3_closure_receipts
    FOR EACH ROW EXECUTE FUNCTION plat3_closure_immutable();
