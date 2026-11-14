-- Native recovery hardening (gateway login contract §19.3, §19.4).
--
-- 1. The recovery foreign keys of 20261112100007 had no ON DELETE rule, so an
--    account or device-family deletion failed on a leftover handle or attempt
--    record. Each row is owned by the referenced account or credential, so it
--    goes with it (CASCADE). SET NULL is not an option for device_family_id: it
--    would break the family/thumbprint pair CHECK and unbind a device-bound
--    handle. Drop and re-add happen in one ALTER TABLE per table, so no moment
--    exists without the constraint, and existing rows already satisfy it.
ALTER TABLE native_recovery_handles
    DROP CONSTRAINT native_recovery_handles_account_id_fkey,
    ADD CONSTRAINT native_recovery_handles_account_id_fkey
        FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    DROP CONSTRAINT native_recovery_handles_device_family_id_fkey,
    ADD CONSTRAINT native_recovery_handles_device_family_id_fkey
        FOREIGN KEY (device_family_id) REFERENCES device_sessions(id) ON DELETE CASCADE;
ALTER TABLE native_recoveries
    DROP CONSTRAINT native_recoveries_account_id_fkey,
    ADD CONSTRAINT native_recoveries_account_id_fkey
        FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE;

-- 2. §19.3: a change of native_security_generation ends every handle of the
--    account. Every credential change (password change, reset or rehash, e-mail
--    change or verification, MFA enable, replace or remove, recovery-key use,
--    status change, staff kick) advances native_security_generation or
--    game_auth_generation in its own transaction (native_device_security_fence,
--    fence_native_device_mfa and the identity writers), so revoking here covers
--    every writer, present and future, in that same transaction. Attempt
--    records (native_recoveries) stay until retirement (§6.1, §19.4 step 3); a
--    retry of one re-authenticates its handle and is refused once it is revoked.
CREATE FUNCTION revoke_native_recovery_handles_on_security_change() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    UPDATE native_recovery_handles
       SET revoked_at = floor(extract(epoch FROM clock_timestamp()))::bigint
     WHERE account_id = NEW.id AND revoked_at IS NULL;
    RETURN NULL;
END;
$$;
CREATE TRIGGER accounts_revoke_native_recovery_handles AFTER UPDATE ON accounts
    FOR EACH ROW
    WHEN (NEW.native_security_generation IS DISTINCT FROM OLD.native_security_generation
       OR NEW.game_auth_generation IS DISTINCT FROM OLD.game_auth_generation)
    EXECUTE FUNCTION revoke_native_recovery_handles_on_security_change();
