INSERT INTO role_permissions (role_id, permission)
SELECT id, 'support.reports.manage'
FROM roles
WHERE built_in AND key = 'moderator'
ON CONFLICT DO NOTHING;
