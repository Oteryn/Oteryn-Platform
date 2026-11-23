-- Extend the existing capability constraint without removing any grant.
ALTER TABLE role_permissions DROP CONSTRAINT role_permissions_permission_check;
ALTER TABLE role_permissions ADD CONSTRAINT role_permissions_permission_check
    CHECK (permission IN (
        'admin.access',
        'admin.analytics.view',
        'admin.analytics.export',
        'admin.roles.manage',
        'audit.view',
        'cms.news.manage',
        'cms.pages.manage',
        'media.manage',
        'portal.access',
        'portal.announcements.manage',
        'portal.settings.manage',
        'downloads.manage',
        'events.manage',
        'events.publish',
        'support.content.manage',
        'support.tickets.manage',
        'support.reports.manage',
        'support.enforcement.manage',
        'marketplace.manage',
        'products.premium.manage',
        'payments.reconcile',
        'game_catalog.access',
        'game_catalog.snapshots.view',
        'game_catalog.snapshots.import',
        'game_catalog.snapshots.activate',
        'game_catalog.profiles.manage',
        'game_catalog.translations.manage',
        'game_catalog.overrides.manage',
        'wiki.access',
        'wiki.articles.manage',
        'wiki.categories.manage',
        'wiki.publish',
        'game.ops.read',
        'game.ops.write',
        'game.ops.logs.read',
        'game.world.read',
        'game.world.ops',
        'game.world.config',
        'game.world.events',
        'game.world.admin',
        'game.node.read',
        'game.node.ops',
        'game.node.admin',
        'game.node.deploy',
        'game.db.read',
        'game.db.backup',
        'game.db.query',
        'game.db.admin',
        'game.player.read',
        'game.player.moderate',
        'game.player.manage',
        'game.player.value.write',
        'game.economy.read',
        'game.economy.moderate',
        'game.sanction.read',
        'game.sanction.write',
        'game.sanction.ban',
        'game.staff.admin',
        'game.analytics.read',
        'game.analytics.player',
        'game.analytics.sql',
        'game.analytics.relink',
        'game.analytics.export',
        'game.audit.read',
        'account.security.kick',
        'payments.coin_hold.release',
        'store.catalog.import',
        'store.catalog.activate',
        'store.codes.generate',
        'mail.outbox.view',
        'mail.outbox.recipient',
        'mail.outbox.retry',
        'mail.outbox.retry_batch',
        'mail.outbox.cancel'
    ));

INSERT INTO role_permissions (role_id, permission)
SELECT id, permission FROM roles
CROSS JOIN unnest(ARRAY['mail.outbox.view', 'mail.outbox.retry',
                        'mail.outbox.retry_batch', 'mail.outbox.cancel']) AS grants(permission)
WHERE key = 'platform_admin' AND built_in ON CONFLICT DO NOTHING;

ALTER TABLE email_outbox ADD COLUMN template_kind text CHECK (
    template_kind IN ('new-device', 'password-changed', 'news', 'support')
);

ALTER TABLE outbox
    ADD COLUMN mail_cancelled_at timestamptz,
    ADD COLUMN mail_reset_issued_at timestamptz,
    ADD CONSTRAINT outbox_mail_cancelled_state CHECK (mail_cancelled_at IS NULL OR state = 'dead');

-- Keep the reset issuance witness without duplicating the recovery address.
CREATE FUNCTION capture_mail_reset_issuance() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF pg_input_is_valid(NEW.payload->>'account_id', 'uuid') THEN
        SELECT created_at INTO NEW.mail_reset_issued_at FROM account_password_resets
        WHERE account_id = (NEW.payload->>'account_id')::uuid;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER capture_mail_reset_issuance BEFORE INSERT ON outbox
    FOR EACH ROW WHEN (NEW.kind = 'email.password-reset') EXECUTE FUNCTION capture_mail_reset_issuance();

CREATE INDEX outbox_mail_created ON outbox (created_at DESC, id DESC)
    WHERE kind IN ('email.account_exists', 'email.verify', 'email.password-reset', 'notify.email');
CREATE INDEX outbox_mail_failed ON outbox (created_at, id)
    WHERE kind IN ('email.account_exists', 'email.verify', 'email.password-reset', 'notify.email')
        AND state = 'pending' AND last_error IS NOT NULL;

CREATE VIEW admin_mail_outbox AS
SELECT o.id, o.kind AS queue_kind,
    CASE WHEN o.kind = 'notify.email' THEN 'notify.' || coalesce(e.template_kind, e.category, 'email')
         ELSE o.kind END AS kind,
    CASE WHEN o.mail_cancelled_at IS NOT NULL THEN 'cancelled'
         WHEN o.state = 'done' THEN 'sent'
         WHEN o.state = 'pending' AND o.last_error IS NOT NULL THEN 'failed'
         ELSE o.state END AS state,
    CASE o.kind
        WHEN 'notify.email' THEN e.recipient
        WHEN 'email.account_exists' THEN coalesce(o.payload->>'email',
             CASE WHEN a.email_change_blocked_until = '-infinity' THEN a.email END)
        WHEN 'email.verify' THEN
            CASE WHEN a.status <> 'closed' AND EXISTS (
                SELECT 1 FROM account_email_tokens t
                WHERE t.token_hash = sha256(convert_to(o.payload->>'token', 'UTF8'))
                  AND t.account_id = a.id AND t.purpose = 'verify_email'
            ) AND a.email_change_blocked_until = '-infinity' THEN a.email END
        WHEN 'email.password-reset' THEN
            CASE WHEN a.status <> 'closed' AND r.account_id IS NOT NULL THEN coalesce(r.recovery_email, a.email) END
    END AS recipient,
    o.attempts, o.last_error, o.created_at
FROM outbox o
LEFT JOIN email_outbox e ON e.id = CASE
    WHEN o.kind = 'notify.email' AND pg_input_is_valid(o.payload #>> '{}', 'uuid')
    THEN (o.payload #>> '{}')::uuid END
LEFT JOIN accounts a ON a.id = CASE
    WHEN o.kind <> 'notify.email' AND pg_input_is_valid(o.payload->>'account_id', 'uuid')
    THEN (o.payload->>'account_id')::uuid END
LEFT JOIN account_password_resets r ON o.kind = 'email.password-reset'
    AND r.account_id = a.id AND r.created_at = o.mail_reset_issued_at
WHERE o.kind IN ('email.account_exists', 'email.verify', 'email.password-reset', 'notify.email');
