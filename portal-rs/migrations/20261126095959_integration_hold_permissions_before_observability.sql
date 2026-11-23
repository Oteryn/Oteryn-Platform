-- Observability's original check predates the capabilities applied above.
WITH held AS (
    DELETE FROM role_permissions
    WHERE permission LIKE ANY (ARRAY[
        'admin.staff_invites.%', 'registration.policy.%', 'mail.outbox.%',
        'admin.system.view', 'status.%', 'forum.%', 'newsletter.%'
    ])
    RETURNING role_id, permission, created_at, updated_at
)
INSERT INTO integration_portal_permission_holds (role_id, permission, created_at, updated_at)
SELECT role_id, permission, created_at, updated_at FROM held;
