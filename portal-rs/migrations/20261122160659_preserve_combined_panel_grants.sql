-- Preserve grants across the immutable operations permission constraint.
WITH moved AS (
    DELETE FROM role_permissions
    WHERE permission IN (
        'account.email.edit',
        'account.flags.edit',
        'account.status.edit'
    )
    RETURNING role_id, permission
)
INSERT INTO portal_panel_merge_grants (role_id, permission)
SELECT role_id, permission FROM moved;
