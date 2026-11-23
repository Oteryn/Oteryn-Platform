-- Preserve grants across the immutable foundation permission constraint.
WITH moved AS (
    DELETE FROM role_permissions
    WHERE permission IN (
        'ops.playerdata_binding_conflict.read',
        'support.reports.review',
        'puzzles.publish'
    )
    RETURNING role_id, permission
)
INSERT INTO portal_panel_merge_grants (role_id, permission)
SELECT role_id, permission FROM moved;
