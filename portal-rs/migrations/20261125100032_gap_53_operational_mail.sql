-- Extend the permission vocabulary without assigning either campaign role.
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
        'mail.outbox.cancel',
        'players.account.edit.email',
        'players.account.edit.display-name',
        'players.account.edit.locale',
        'players.account.edit.public-account-association',
        'players.account.edit.public-status-visible',
        'players.account.edit.character-profiles-hidden',
        'mail.operational.compose',
        'mail.operational.review'
    ));

ALTER TABLE account_notification_prefs DROP CONSTRAINT account_notification_prefs_category_check;
ALTER TABLE account_notification_prefs ADD CONSTRAINT account_notification_prefs_category_check
    CHECK (category IN ('security', 'news', 'support', 'operational'));
ALTER TABLE account_notification_prefs ADD CONSTRAINT account_notification_prefs_operational_email_only
    CHECK (category <> 'operational' OR NOT portal_enabled);
CREATE INDEX operational_mail_opted_in ON account_notification_prefs(account_id)
    WHERE category='operational' AND email_enabled;

ALTER TABLE email_outbox DROP CONSTRAINT email_outbox_category_check;
ALTER TABLE email_outbox ADD CONSTRAINT email_outbox_category_check
    CHECK (category IN ('security', 'news', 'support', 'operational'));
ALTER TABLE email_outbox ADD CONSTRAINT email_outbox_operational_unsubscribe
    CHECK (category <> 'operational' OR unsubscribe_url IS NOT NULL);

CREATE TABLE operational_mail_campaigns (
    id uuid PRIMARY KEY,
    author uuid NOT NULL,
    audience text NOT NULL CHECK (audience IN ('all','pl','en')),
    subject_pl text NOT NULL CHECK (octet_length(subject_pl) BETWEEN 1 AND 200),
    body_pl text NOT NULL CHECK (octet_length(body_pl) BETWEEN 1 AND 4000),
    subject_en text NOT NULL CHECK (octet_length(subject_en) BETWEEN 1 AND 200),
    body_en text NOT NULL CHECK (octet_length(body_en) BETWEEN 1 AND 4000),
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL DEFAULT (now() + interval '24 hours'),
    decision text CHECK (decision IN ('approved','rejected')),
    reviewer uuid,
    reviewed_at timestamptz,
    CHECK (expires_at > created_at),
    CHECK (reviewer <> author),
    CHECK ((decision IS NULL AND reviewer IS NULL AND reviewed_at IS NULL)
        OR (decision IS NOT NULL AND reviewer IS NOT NULL AND reviewed_at IS NOT NULL))
);
CREATE INDEX operational_mail_campaigns_author_created ON operational_mail_campaigns(author, created_at DESC);

CREATE TABLE operational_mail_recipients (
    campaign_id uuid NOT NULL REFERENCES operational_mail_campaigns(id),
    account_id uuid NOT NULL,
    locale text NOT NULL CHECK (locale IN ('pl','en')),
    address_digest bytea NOT NULL CHECK (octet_length(address_digest)=32),
    verified_at timestamptz NOT NULL,
    consent_at timestamptz NOT NULL,
    mail_id uuid UNIQUE REFERENCES email_outbox(id),
    job_id uuid UNIQUE,
    skipped_at timestamptz,
    PRIMARY KEY (campaign_id, account_id),
    CHECK ((mail_id IS NULL) = (job_id IS NULL)),
    CHECK (skipped_at IS NULL OR mail_id IS NOT NULL)
);
