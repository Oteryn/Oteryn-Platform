ALTER TABLE accounts ADD COLUMN display_name text NOT NULL DEFAULT ''
    CHECK (char_length(display_name) <= 80);

CREATE INDEX accounts_email_prefix ON accounts (lower(email) text_pattern_ops) WHERE status<>'closed';
CREATE INDEX accounts_id_prefix ON accounts ((id::text) text_pattern_ops);
CREATE INDEX account_characters_name_prefix ON account_characters (lower(name) text_pattern_ops);

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
        'players.account.edit.email',
        'players.account.edit.display-name',
        'players.account.edit.locale',
        'players.account.edit.public-account-association',
        'players.account.edit.public-status-visible',
        'players.account.edit.character-profiles-hidden'
    ));
INSERT INTO role_permissions (role_id, permission)
SELECT r.id, p.permission
FROM roles r CROSS JOIN (VALUES
    ('players.account.edit.email'),
    ('players.account.edit.display-name'),
    ('players.account.edit.locale'),
    ('players.account.edit.public-account-association'),
    ('players.account.edit.public-status-visible'),
    ('players.account.edit.character-profiles-hidden')
) p(permission)
WHERE r.key='platform_admin' AND r.built_in ON CONFLICT DO NOTHING;
