-- Native policy is provisioned explicitly; no existing client is opted in.
CREATE TABLE oauth_native_clients (
    client_id uuid PRIMARY KEY,
    enabled boolean NOT NULL DEFAULT false,
    first_party boolean NOT NULL DEFAULT false,
    public_client boolean NOT NULL DEFAULT false,
    native_ticket boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
ALTER TABLE accounts ADD COLUMN game_auth_generation bigint NOT NULL DEFAULT 1 CHECK (game_auth_generation > 0);
-- Existing grants without an original snapshot cannot enroll native devices.
ALTER TABLE oauth_token_families
    ADD COLUMN game_generation bigint CHECK (game_generation > 0),
    ADD COLUMN native_generation bigint CHECK (native_generation > 0);
ALTER TABLE device_sessions
    ADD COLUMN persistence_class text NOT NULL DEFAULT 'remembered' CHECK (persistence_class IN ('session','remembered')),
    ADD COLUMN device_public_key bytea CHECK (octet_length(device_public_key)=32),
    ADD COLUMN device_key_thumbprint text UNIQUE,
    ADD CONSTRAINT device_key_pair CHECK ((device_public_key IS NULL) = (device_key_thumbprint IS NULL)),
    ADD CONSTRAINT session_requires_key CHECK (persistence_class <> 'session' OR device_public_key IS NOT NULL);
ALTER TABLE game_login_tickets ADD COLUMN device_family_id uuid REFERENCES device_sessions(id);
CREATE INDEX game_login_tickets_device_family ON game_login_tickets(device_family_id);

-- Account writers serialize on the account. Fence prior families across a later
-- restoration of status/password/MFA settings as well as their initial change.
CREATE FUNCTION fence_native_device_credentials() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.password_hash IS DISTINCT FROM OLD.password_hash
       OR NEW.email IS DISTINCT FROM OLD.email
       OR NEW.email_verified_at IS DISTINCT FROM OLD.email_verified_at
       OR NEW.status IS DISTINCT FROM OLD.status
       OR NEW.login_mfa_required IS DISTINCT FROM OLD.login_mfa_required THEN
        NEW.game_auth_generation := OLD.game_auth_generation + 1;
        NEW.native_security_generation := OLD.native_security_generation + 1;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER native_device_security_fence BEFORE UPDATE ON accounts
    FOR EACH ROW EXECUTE FUNCTION fence_native_device_credentials();
CREATE FUNCTION revoke_native_device_credentials() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.game_auth_generation IS DISTINCT FROM OLD.game_auth_generation
       OR NEW.native_security_generation IS DISTINCT FROM OLD.native_security_generation THEN
        UPDATE device_sessions SET revoked_at=coalesce(revoked_at,floor(extract(epoch FROM clock_timestamp()))::bigint),updated_at=now() WHERE account_id=NEW.id;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER native_device_security_revoke AFTER UPDATE ON accounts
    FOR EACH ROW EXECUTE FUNCTION revoke_native_device_credentials();
-- Confirmed MFA replacement/removal fences authority even if the required bit
-- stays true. Routine TOTP last-step updates do not change the generations.
CREATE FUNCTION fence_native_device_mfa() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF OLD.confirmed THEN
            UPDATE accounts SET game_auth_generation=game_auth_generation+1,native_security_generation=native_security_generation+1 WHERE id=OLD.account_id;
        END IF;
        RETURN OLD;
    END IF;
    IF TG_OP = 'INSERT' THEN
        IF NEW.confirmed THEN
            UPDATE accounts SET game_auth_generation=game_auth_generation+1,native_security_generation=native_security_generation+1 WHERE id=NEW.account_id;
        END IF;
        RETURN NEW;
    END IF;
    IF (OLD.confirmed OR NEW.confirmed) AND
       (NEW.secret IS DISTINCT FROM OLD.secret OR NEW.confirmed IS DISTINCT FROM OLD.confirmed OR NEW.generation IS DISTINCT FROM OLD.generation) THEN
        UPDATE accounts SET game_auth_generation=game_auth_generation+1,native_security_generation=native_security_generation+1 WHERE id=NEW.account_id;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER native_device_mfa_fence AFTER INSERT OR UPDATE OR DELETE ON account_mfa_totp
    FOR EACH ROW EXECUTE FUNCTION fence_native_device_mfa();
