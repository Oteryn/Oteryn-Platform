-- Observability restores the earlier vocabulary. Extend it without replacing existing grants.
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
        CHECK ((%s) OR permission IN ('mail.operational.compose', 'mail.operational.review'))
        $check$,
        current_check
    );
END $$;

ALTER TABLE account_notification_prefs
    DROP CONSTRAINT account_notification_prefs_category_check,
    ADD CONSTRAINT account_notification_prefs_category_check
        CHECK (category IN ('security', 'news', 'support', 'events', 'forum_digest', 'forum', 'operational'));

ALTER TABLE email_outbox
    DROP CONSTRAINT email_outbox_category_check,
    ADD CONSTRAINT email_outbox_category_check
        CHECK (category IN ('security', 'news', 'support', 'operations', 'forum', 'operational'));
