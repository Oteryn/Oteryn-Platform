-- Game enforcement command wire (oteryn-v2-game-enforcement-wire-v1, Part A).
-- The command journal is the T34 enforcement outbox: one row per Platform
-- decision whose profile is enabled, written in the decision's transaction.
-- One transactional counter serializes publication until commit, as in N009.
CREATE TABLE support_enforcement_counter (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    position bigint NOT NULL DEFAULT 0 CHECK (position >= 0)
);
INSERT INTO support_enforcement_counter DEFAULT VALUES;

CREATE TABLE support_enforcement_commands (
    position bigint PRIMARY KEY CHECK (position > 0),
    operation_id uuid NOT NULL UNIQUE CHECK (uuid_extract_version(operation_id) = 7),
    sanction_stream_id uuid NOT NULL CHECK (uuid_extract_version(sanction_stream_id) = 7),
    decision_revision bigint NOT NULL CHECK (decision_revision > 0),
    intent text NOT NULL CHECK (intent IN ('APPLY','REPLACE','REVOKE','EXPIRE')),
    profile text NOT NULL CHECK (profile IN ('MUTE','JAIL')),
    account_id uuid NOT NULL CHECK (uuid_extract_version(account_id) = 7),
    character_id uuid NOT NULL CHECK (uuid_extract_version(character_id) = 7),
    -- Exact canonical bytes served by A1 and A2, and their SHA-256.
    command bytea NOT NULL CHECK (octet_length(command) BETWEEN 2 AND 1024),
    command_digest text NOT NULL CHECK (command_digest ~ '^[0-9a-f]{64}$'),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX support_enforcement_commands_stream
    ON support_enforcement_commands(sanction_stream_id, decision_revision DESC);

-- Game's authoritative results, keyed by (operation_id, receipt_revision).
-- The exact canonical bytes are stored so a republication is compared byte for byte.
CREATE TABLE support_enforcement_results (
    operation_id uuid NOT NULL REFERENCES support_enforcement_commands(operation_id),
    receipt_revision integer NOT NULL CHECK (receipt_revision BETWEEN 1 AND 65535),
    state text NOT NULL CHECK (state IN
        ('ACCEPTED_PENDING','APPLIED','REPLACED','REVOKED','EXPIRED','REJECTED')),
    terminal boolean NOT NULL CHECK (terminal = (state <> 'ACCEPTED_PENDING')),
    result bytea NOT NULL CHECK (octet_length(result) BETWEEN 2 AND 1024),
    result_digest text NOT NULL CHECK (result_digest ~ '^[0-9a-f]{64}$'),
    received_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (operation_id, receipt_revision)
);
-- Exactly one terminal result exists per operation.
CREATE UNIQUE INDEX support_enforcement_results_terminal
    ON support_enforcement_results(operation_id) WHERE terminal;

CREATE FUNCTION support_enforcement_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'enforcement journal is append only' USING ERRCODE = '23514';
END $$;
CREATE TRIGGER support_enforcement_commands_immutable
    BEFORE UPDATE OR DELETE ON support_enforcement_commands
    FOR EACH ROW EXECUTE FUNCTION support_enforcement_immutable();
CREATE TRIGGER support_enforcement_results_immutable
    BEFORE UPDATE OR DELETE ON support_enforcement_results
    FOR EACH ROW EXECUTE FUNCTION support_enforcement_immutable();
