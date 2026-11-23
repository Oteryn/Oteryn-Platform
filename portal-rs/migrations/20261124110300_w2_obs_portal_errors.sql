CREATE TABLE portal_errors (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    route text NOT NULL,
    error_kind text NOT NULL,
    top_frame text,
    status text NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'resolved', 'ignored')),
    occurrence_count bigint NOT NULL DEFAULT 1 CHECK (occurrence_count > 0),
    first_seen timestamptz NOT NULL DEFAULT statement_timestamp(),
    last_seen timestamptz NOT NULL DEFAULT statement_timestamp(),
    last_request_id uuid NOT NULL,
    alert_enqueued_at timestamptz,
    UNIQUE NULLS NOT DISTINCT (route, error_kind, top_frame)
);
CREATE INDEX portal_errors_recent ON portal_errors (last_seen DESC, id DESC);
CREATE INDEX portal_errors_status_recent ON portal_errors (status, last_seen DESC, id DESC);
CREATE INDEX portal_errors_unalerted ON portal_errors (first_seen, id) WHERE alert_enqueued_at IS NULL;

-- Rolling five-minute rate, bounded to one counter per second even with alerting disabled.
CREATE TABLE portal_error_rate (
    slot smallint PRIMARY KEY CHECK (slot BETWEEN 0 AND 299),
    second bigint NOT NULL CHECK (second % 300 = slot),
    count bigint NOT NULL CHECK (count > 0)
);
CREATE TABLE portal_error_spike (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    above_threshold boolean NOT NULL DEFAULT false,
    last_enqueued_at timestamptz
);
INSERT INTO portal_error_spike DEFAULT VALUES;

-- Operational messages have a configured blind recipient list and no account.
ALTER TABLE email_outbox
    ADD COLUMN staff_recipients text[],
    ALTER COLUMN account_id DROP NOT NULL,
    DROP CONSTRAINT email_outbox_category_check,
    ADD CONSTRAINT email_outbox_category_check
        CHECK (category IN ('security', 'news', 'support', 'operations')),
    ADD CONSTRAINT email_outbox_recipient_type CHECK (
        (category = 'operations' AND account_id IS NULL AND recipient = ''
            AND unsubscribe_url IS NULL AND staff_recipients IS NOT NULL
            AND cardinality(staff_recipients) BETWEEN 1 AND 25
            AND array_position(staff_recipients, NULL) IS NULL)
        OR (category <> 'operations' AND account_id IS NOT NULL AND staff_recipients IS NULL)
    );

-- Extend the existing capability constraint without removing any grant.
ALTER TABLE role_permissions DROP CONSTRAINT role_permissions_permission_check;
ALTER TABLE role_permissions ADD CONSTRAINT role_permissions_permission_check
    CHECK (permission IN (
        'admin.access',
        'admin.errors.view',
        'admin.errors.resolve',
        'admin.errors.ignore',
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
        'store.codes.generate'
    ));
INSERT INTO role_permissions (role_id, permission)
SELECT id, permission FROM roles CROSS JOIN (VALUES
    ('admin.errors.view'), ('admin.errors.resolve'), ('admin.errors.ignore')
) AS grants(permission)
WHERE key = 'platform_admin' AND built_in ON CONFLICT DO NOTHING;
