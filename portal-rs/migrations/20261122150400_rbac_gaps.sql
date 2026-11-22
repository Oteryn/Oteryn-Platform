-- Allow the built-in moderator to review reports in the staff panel.
INSERT INTO role_permissions (role_id, permission)
SELECT id, 'support.reports.manage' FROM roles
WHERE key = 'moderator' AND built_in ON CONFLICT DO NOTHING;
