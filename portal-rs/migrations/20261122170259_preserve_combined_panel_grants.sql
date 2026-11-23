-- Preserve grants across the immutable Game enforcement permission constraint.
WITH moved AS (
    DELETE FROM role_permissions
    WHERE permission IN (
        'admin.rate_limits.view',
        'admin.flags.view',
        'admin.health.view'
    )
    RETURNING role_id, permission
)
INSERT INTO portal_panel_merge_grants (role_id, permission)
SELECT role_id, permission FROM moved;
