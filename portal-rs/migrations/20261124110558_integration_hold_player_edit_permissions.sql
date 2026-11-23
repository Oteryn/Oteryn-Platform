-- The status and forum migrations replace the capability check before the
-- observability migration can be applied at its new version.
CREATE TABLE integration_portal_permission_holds (
    role_id uuid NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission text NOT NULL,
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
    PRIMARY KEY (role_id, permission)
);

WITH held AS (
    DELETE FROM role_permissions
    WHERE permission LIKE 'players.account.edit.%'
    RETURNING role_id, permission, created_at, updated_at
)
INSERT INTO integration_portal_permission_holds (role_id, permission, created_at, updated_at)
SELECT role_id, permission, created_at, updated_at FROM held;
