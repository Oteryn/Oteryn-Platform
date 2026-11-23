-- Preserve the observability grants and restore the other reviewed capabilities.
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
            'mail.outbox.cancel', 'mail.outbox.recipient', 'admin.system.view',
            'status.view', 'status.incident.create', 'status.incident.update',
            'status.incident.resolve', 'status.maintenance.schedule',
            'forum.moderate', 'forum.board.create', 'forum.board.edit',
            'forum.thread.create', 'forum.post.create', 'forum.thread.lock',
            'forum.thread.pin', 'forum.thread.move', 'forum.thread.delete',
            'forum.thread.restore', 'forum.thread.edit', 'forum.post.delete',
            'forum.post.restore', 'forum.post.edit', 'forum.subscription.edit',
            'forum.profile.edit', 'newsletter.view', 'newsletter.compose',
            'newsletter.test', 'newsletter.send', 'players.account.edit.email',
            'players.account.edit.display-name', 'players.account.edit.locale',
            'players.account.edit.public-account-association',
            'players.account.edit.public-status-visible',
            'players.account.edit.character-profiles-hidden'
        ))
        $check$,
        current_check
    );
END $$;

INSERT INTO role_permissions (role_id, permission, created_at, updated_at)
SELECT role_id, permission, created_at, updated_at FROM integration_portal_permission_holds
ON CONFLICT (role_id, permission) DO NOTHING;

DROP TABLE integration_portal_permission_holds;

ALTER TABLE email_outbox
    DROP CONSTRAINT email_outbox_category_check,
    ADD CONSTRAINT email_outbox_category_check
        CHECK (category IN ('security', 'news', 'support', 'operations', 'forum'));
