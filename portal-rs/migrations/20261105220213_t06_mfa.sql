CREATE TABLE account_mfa_totp (
    account_id uuid PRIMARY KEY REFERENCES accounts(id),
    secret bytea NOT NULL,
    confirmed boolean NOT NULL DEFAULT false,
    last_step bigint NOT NULL DEFAULT -1,
    generation uuid NOT NULL DEFAULT uuidv7(),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE account_recovery_codes (
    account_id uuid NOT NULL REFERENCES accounts(id),
    code_hash bytea NOT NULL CHECK (octet_length(code_hash)=32),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(account_id,code_hash)
);
CREATE TABLE account_mfa_tokens (
    token_hash bytea PRIMARY KEY CHECK (octet_length(token_hash)=32),
    account_id uuid NOT NULL REFERENCES accounts(id),
    generation uuid NOT NULL,
    credential_hash bytea NOT NULL,
    purpose text NOT NULL CHECK (purpose IN ('challenge','email','trusted','grant')),
    parent_hash bytea,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX account_mfa_tokens_account ON account_mfa_tokens(account_id,purpose);
CREATE INDEX account_mfa_tokens_expiry ON account_mfa_tokens(expires_at);
-- Credential changes invalidate browser proofs, pending challenges and remembered devices.
CREATE FUNCTION invalidate_mfa_credentials() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.password_hash IS DISTINCT FROM OLD.password_hash OR NEW.status IS DISTINCT FROM OLD.status
       OR NEW.email IS DISTINCT FROM OLD.email OR NEW.login_mfa_required IS DISTINCT FROM OLD.login_mfa_required THEN
        DELETE FROM account_mfa_tokens WHERE account_id=NEW.id;
        DELETE FROM web_sessions WHERE account_id=NEW.id;
        DELETE FROM native_admissions WHERE account_id=NEW.id;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER account_mfa_credential_change AFTER UPDATE ON accounts
    FOR EACH ROW EXECUTE FUNCTION invalidate_mfa_credentials();
