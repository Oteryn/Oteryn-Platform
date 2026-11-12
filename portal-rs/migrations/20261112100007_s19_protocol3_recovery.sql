-- Gateway login contract §19 (N4P-4): protocol 3 channel keys, recovery handles,
-- recovery attempts and the recovery locator read model. Testing/preproduction
-- only and default off; every addition is additive and nullable or new.

-- §19.2.2: the Registry channel key shown to players. A channel without a key is
-- never a protocol 3 candidate (fail closed); protocol 2 ignores the column.
ALTER TABLE game_channels
    ADD COLUMN channel_key text CHECK (channel_key ~ '^[a-z0-9][a-z0-9-]{0,63}$');
CREATE UNIQUE INDEX game_channels_world_key ON game_channels (world_id, channel_key)
    WHERE channel_key IS NOT NULL;

-- §19.3: only the SHA-256 of a handle is stored. Times are epoch seconds.
CREATE TABLE native_recovery_handles (
    handle_hash bytea PRIMARY KEY CHECK (octet_length(handle_hash) = 32),
    account_id uuid NOT NULL REFERENCES accounts(id),
    character_id uuid NOT NULL,
    world_id uuid NOT NULL,
    channel_id uuid NOT NULL,
    security_generation bigint NOT NULL CHECK (security_generation > 0),
    play_generation bigint NOT NULL CHECK (play_generation > 0),
    attempt_ref uuid NOT NULL,
    device_family_id uuid REFERENCES device_sessions(id),
    device_key_thumbprint text CHECK (octet_length(device_key_thumbprint) = 43),
    issued_at bigint NOT NULL,
    expires_at bigint NOT NULL CHECK (expires_at > issued_at),
    revoked_at bigint,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK ((device_family_id IS NULL) = (device_key_thumbprint IS NULL))
);
CREATE INDEX native_recovery_handles_account ON native_recovery_handles (account_id);
CREATE INDEX native_recovery_handles_attempt ON native_recovery_handles (attempt_ref);
CREATE INDEX native_recovery_handles_family ON native_recovery_handles (device_family_id)
    WHERE device_family_id IS NOT NULL;

-- §19.4: recovery attempt records, stored like native_admissions (§6.1: the
-- signing input only, never the token).
CREATE TABLE native_recoveries (
    attempt_ref uuid PRIMARY KEY,
    handle_hash bytea NOT NULL CHECK (octet_length(handle_hash) = 32),
    request_digest bytea NOT NULL CHECK (octet_length(request_digest) = 32),
    account_id uuid NOT NULL REFERENCES accounts(id),
    response_body bytea NOT NULL,
    expires_at_ms bigint NOT NULL,
    retires_at_ms bigint NOT NULL CHECK (retires_at_ms >= expires_at_ms),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX native_recoveries_account ON native_recoveries (account_id, created_at);
CREATE INDEX native_recoveries_handle ON native_recoveries (handle_hash);

-- §19.6: one locator entry per Character from the Game recovery locator feed.
CREATE TABLE native_recovery_locators (
    character_id uuid PRIMARY KEY,
    account_id uuid NOT NULL,
    world_id uuid NOT NULL,
    channel_id uuid NOT NULL,
    present boolean NOT NULL,
    assignment_epoch numeric(20, 0) NOT NULL CHECK (assignment_epoch > 0),
    scope_ownership_generation numeric(20, 0) NOT NULL CHECK (scope_ownership_generation > 0),
    locator_revision numeric(20, 0) NOT NULL CHECK (locator_revision > 0),
    committed_at bigint NOT NULL,
    source_authority text NOT NULL CHECK (octet_length(source_authority) BETWEEN 1 AND 128),
    conflicted boolean NOT NULL DEFAULT false,
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);

-- §19.15 DB-6: a terminal device-family revocation revokes its handles in the
-- same transaction, whichever path revoked the family.
CREATE FUNCTION revoke_native_recovery_handles() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.revoked_at IS NULL AND NEW.revoked_at IS NOT NULL THEN
        UPDATE native_recovery_handles
           SET revoked_at = floor(extract(epoch FROM clock_timestamp()))::bigint
         WHERE device_family_id = NEW.id AND revoked_at IS NULL;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER device_sessions_revoke_recovery_handles
    AFTER UPDATE OF revoked_at ON device_sessions
    FOR EACH ROW EXECUTE FUNCTION revoke_native_recovery_handles();
