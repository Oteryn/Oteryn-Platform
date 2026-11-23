-- The last feature migration already permits the shared, forum and newsletter grants.
DO $$
DECLARE
    current_check text;
BEGIN
    SELECT pg_get_expr(conbin, conrelid)
    INTO STRICT current_check
    FROM pg_constraint
    WHERE conrelid = 'role_permissions'::regclass AND conname = 'role_permissions_permission_check';

    ALTER TABLE role_permissions DROP CONSTRAINT role_permissions_permission_check;
    EXECUTE format(
        $check$
        ALTER TABLE role_permissions ADD CONSTRAINT role_permissions_permission_check
        CHECK ((%s) OR permission IN (
            'admin.staff_invites.list', 'admin.staff_invites.create', 'admin.staff_invites.revoke',
            'registration.policy.view', 'registration.policy.manage',
            'mail.outbox.view', 'mail.outbox.retry', 'mail.outbox.retry_batch',
            'mail.outbox.cancel', 'mail.outbox.recipient',
            'admin.errors.view', 'admin.errors.resolve', 'admin.errors.ignore', 'admin.system.view',
            'status.view', 'status.incident.create', 'status.incident.update',
            'status.incident.resolve', 'status.maintenance.schedule'
        ))
        $check$,
        current_check
    );
END $$;

INSERT INTO role_permissions (role_id, permission, created_at, updated_at)
SELECT role_id, permission, created_at, updated_at FROM integration_wave2_permission_holds
ON CONFLICT (role_id, permission) DO NOTHING;

DROP FUNCTION integration_hold_wave2_permissions(text[]);
DROP TABLE integration_wave2_permission_holds;

ALTER TABLE account_notification_prefs
    DROP CONSTRAINT account_notification_prefs_category_check,
    ADD CONSTRAINT account_notification_prefs_category_check
        CHECK (category IN ('security', 'news', 'support', 'events', 'forum_digest', 'forum'));

ALTER TABLE email_outbox
    DROP CONSTRAINT email_outbox_category_check,
    ADD CONSTRAINT email_outbox_category_check
        CHECK (category IN ('security', 'news', 'support', 'operations', 'forum'));
