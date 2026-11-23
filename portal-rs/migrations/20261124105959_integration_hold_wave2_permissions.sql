-- The reviewed Wave 2 migrations independently replace the permission check.
-- Carry earlier grants across those replacements without changing their checksums.
CREATE TABLE integration_wave2_permission_holds (
    role_id uuid NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission text NOT NULL,
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
    PRIMARY KEY (role_id, permission)
);

CREATE FUNCTION integration_hold_wave2_permissions(VARIADIC scopes text[])
RETURNS void
LANGUAGE sql
AS $$
    WITH held AS (
        DELETE FROM role_permissions
        WHERE permission LIKE ANY (scopes)
        RETURNING role_id, permission, created_at, updated_at
    )
    INSERT INTO integration_wave2_permission_holds (role_id, permission, created_at, updated_at)
    SELECT role_id, permission, created_at, updated_at FROM held
    ON CONFLICT (role_id, permission) DO NOTHING;
$$;

-- An upgrade from integration-1 already has the mail outbox grants.
SELECT integration_hold_wave2_permissions('mail.outbox.%');
