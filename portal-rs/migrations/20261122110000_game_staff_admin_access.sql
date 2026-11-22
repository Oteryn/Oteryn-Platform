-- Game staff need the common staff-panel gate before operation-specific checks.
INSERT INTO role_permissions (role_id, permission)
SELECT id, 'admin.access'
FROM roles
WHERE built_in AND key IN (
    'game_admin',
    'game_operator',
    'game_dba',
    'game_master_senior',
    'game_release',
    'game_security',
    'game_support',
    'game_analyst',
    'game_viewer'
)
ON CONFLICT DO NOTHING;
