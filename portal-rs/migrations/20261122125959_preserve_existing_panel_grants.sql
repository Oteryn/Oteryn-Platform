-- A database that already applied the operations package still has its grants
-- when it applies the other package's earlier, immutable catalogue migration.
CREATE TABLE portal_panel_merge_grants (
    role_id uuid NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission text NOT NULL,
    PRIMARY KEY (role_id, permission)
);

WITH moved AS (
    DELETE FROM role_permissions
    WHERE permission IN (
        'admin.roles.grant',
        'admin.rate_limits.view',
        'admin.flags.view',
        'admin.health.view'
    )
    RETURNING role_id, permission
)
INSERT INTO portal_panel_merge_grants (role_id, permission)
SELECT role_id, permission FROM moved;
