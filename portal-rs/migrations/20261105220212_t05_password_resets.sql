CREATE TABLE account_password_resets (
    account_id uuid PRIMARY KEY REFERENCES accounts(id),
    token_hash bytea NOT NULL CHECK (octet_length(token_hash) = 32),
    expires_at timestamptz NOT NULL,
    used_at timestamptz,
    last_reset_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX account_password_resets_expiry ON account_password_resets(expires_at);

-- Serialize session authentication with credential replacement. A login that already
-- verified the old password must not create a session after the revocation commits.
-- last_seen_at is stamped by session middleware before password verification.
CREATE FUNCTION reset_guard_session_authentication() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.account_id IS NOT NULL AND NEW.account_id IS DISTINCT FROM OLD.account_id THEN
        PERFORM id FROM accounts WHERE id=NEW.account_id FOR UPDATE;
        IF EXISTS (
            SELECT 1 FROM account_password_resets
            WHERE account_id=NEW.account_id AND last_reset_at >= OLD.last_seen_at
        ) THEN
            RAISE EXCEPTION 'credentials changed during authentication' USING ERRCODE = '42501';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER reset_session_authentication_guard BEFORE UPDATE OF account_id ON web_sessions
    FOR EACH ROW EXECUTE FUNCTION reset_guard_session_authentication();
