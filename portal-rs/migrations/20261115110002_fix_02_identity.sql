-- Fair, bounded retries retain the original application and expiry deadlines.
ALTER TABLE account_email_changes ADD COLUMN next_attempt_at timestamptz NOT NULL DEFAULT '-infinity';
CREATE INDEX account_email_changes_retry ON account_email_changes(next_attempt_at,apply_after,id)
WHERE applied_at IS NULL AND cancelled_at IS NULL AND reverted_at IS NULL;
-- Bind delivery and consumption to the requested address and exact confirmed backup.
ALTER TABLE account_password_resets ADD COLUMN recovery_email text;
ALTER TABLE account_password_resets ADD COLUMN backup_verified_at timestamptz;
-- Keep immutable technical attempts; the locked work row owns the exclusive decision.
ALTER TABLE plat3_closure_work ADD COLUMN next_resolution_attempt timestamptz NOT NULL DEFAULT '-infinity';
DROP INDEX plat3_closure_one_resolution;
CREATE INDEX plat3_closure_resolution_attempts ON plat3_closure_commands(termination_id,source_revision DESC)
WHERE command='ResolveAccountClosure';
-- Authentication snapshots are immutable proof generations, never session activity.
ALTER TABLE web_sessions ADD COLUMN authorization_generation bigint CHECK (authorization_generation > 0);
CREATE OR REPLACE FUNCTION reset_guard_session_authentication() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE current_generation bigint;
BEGIN
    IF NEW.account_id IS NOT NULL AND NEW.account_id IS DISTINCT FROM OLD.account_id THEN
        SELECT native_security_generation INTO current_generation FROM accounts WHERE id=NEW.account_id FOR UPDATE;
        IF NEW.authorization_generation IS NOT NULL THEN
            IF NEW.authorization_generation IS DISTINCT FROM current_generation THEN
                RAISE EXCEPTION 'credentials changed during authentication' USING ERRCODE = '42501';
            END IF;
        ELSIF EXISTS (
            SELECT 1 FROM account_password_resets
            WHERE account_id=NEW.account_id AND last_reset_at >= OLD.created_at
        ) THEN
            -- Legacy callers may only use anonymous sessions created after a reset.
            -- Current password/MFA/passkey callers always provide a proof generation.
            RAISE EXCEPTION 'credentials changed during authentication' USING ERRCODE = '42501';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
