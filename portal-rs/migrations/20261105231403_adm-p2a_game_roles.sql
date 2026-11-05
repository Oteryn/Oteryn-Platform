-- Extend the closed permission vocabulary and seed ADM-01 role defaults.
ALTER TABLE role_permissions DROP CONSTRAINT role_permissions_permission_check;
ALTER TABLE role_permissions ADD CONSTRAINT role_permissions_permission_check
    CHECK (permission IN (
        'admin.access',
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
        'game.audit.read'
    ));

INSERT INTO roles (key, built_in) VALUES ('game_viewer', true);
INSERT INTO role_permissions (role_id, permission)
SELECT roles.id, permission FROM roles CROSS JOIN (VALUES
    ('game.ops.read'),
    ('game.ops.logs.read'),
    ('game.world.read'),
    ('game.node.read'),
    ('game.db.read'),
    ('game.economy.read'),
    ('game.sanction.read'),
    ('game.analytics.read'),
    ('game.audit.read')
) AS defaults(permission) WHERE roles.key = 'game_viewer';

INSERT INTO roles (key, built_in) VALUES ('game_support', true);
INSERT INTO role_permissions (role_id, permission)
SELECT roles.id, permission FROM roles CROSS JOIN (VALUES
    ('game.ops.read'),
    ('game.ops.logs.read'),
    ('game.world.read'),
    ('game.node.read'),
    ('game.db.read'),
    ('game.economy.read'),
    ('game.sanction.read'),
    ('game.analytics.read'),
    ('game.audit.read'),
    ('game.player.read'),
    ('game.player.moderate'),
    ('game.sanction.write')
) AS defaults(permission) WHERE roles.key = 'game_support';

INSERT INTO roles (key, built_in) VALUES ('game_master_senior', true);
INSERT INTO role_permissions (role_id, permission)
SELECT roles.id, permission FROM roles CROSS JOIN (VALUES
    ('game.ops.read'),
    ('game.ops.logs.read'),
    ('game.world.read'),
    ('game.node.read'),
    ('game.db.read'),
    ('game.economy.read'),
    ('game.sanction.read'),
    ('game.analytics.read'),
    ('game.audit.read'),
    ('game.player.read'),
    ('game.player.moderate'),
    ('game.sanction.write'),
    ('game.player.manage'),
    ('game.player.value.write'),
    ('game.sanction.ban'),
    ('game.world.events'),
    ('game.economy.moderate')
) AS defaults(permission) WHERE roles.key = 'game_master_senior';

INSERT INTO roles (key, built_in) VALUES ('game_operator', true);
INSERT INTO role_permissions (role_id, permission)
SELECT roles.id, permission FROM roles CROSS JOIN (VALUES
    ('game.ops.read'),
    ('game.ops.logs.read'),
    ('game.world.read'),
    ('game.node.read'),
    ('game.db.read'),
    ('game.economy.read'),
    ('game.sanction.read'),
    ('game.analytics.read'),
    ('game.audit.read'),
    ('game.world.ops'),
    ('game.world.config'),
    ('game.node.ops'),
    ('game.db.backup'),
    ('game.ops.write')
) AS defaults(permission) WHERE roles.key = 'game_operator';

INSERT INTO roles (key, built_in) VALUES ('game_admin', true);
INSERT INTO role_permissions (role_id, permission)
SELECT roles.id, permission FROM roles CROSS JOIN (VALUES
    ('game.ops.read'),
    ('game.ops.logs.read'),
    ('game.world.read'),
    ('game.node.read'),
    ('game.db.read'),
    ('game.economy.read'),
    ('game.sanction.read'),
    ('game.analytics.read'),
    ('game.audit.read'),
    ('game.world.ops'),
    ('game.world.config'),
    ('game.node.ops'),
    ('game.db.backup'),
    ('game.ops.write'),
    ('game.world.admin'),
    ('game.node.admin'),
    ('game.db.admin'),
    ('game.staff.admin')
) AS defaults(permission) WHERE roles.key = 'game_admin';

INSERT INTO roles (key, built_in) VALUES ('game_dba', true);
INSERT INTO role_permissions (role_id, permission)
SELECT roles.id, permission FROM roles CROSS JOIN (VALUES
    ('game.db.read'),
    ('game.db.backup'),
    ('game.db.query'),
    ('game.analytics.sql')
) AS defaults(permission) WHERE roles.key = 'game_dba';

INSERT INTO roles (key, built_in) VALUES ('game_analyst', true);
INSERT INTO role_permissions (role_id, permission)
SELECT roles.id, permission FROM roles CROSS JOIN (VALUES
    ('game.analytics.read'),
    ('game.analytics.player'),
    ('game.analytics.sql'),
    ('game.analytics.export')
) AS defaults(permission) WHERE roles.key = 'game_analyst';

INSERT INTO roles (key, built_in) VALUES ('game_security', true);
INSERT INTO role_permissions (role_id, permission)
SELECT roles.id, permission FROM roles CROSS JOIN (VALUES
    ('game.analytics.read'),
    ('game.analytics.player'),
    ('game.analytics.sql'),
    ('game.analytics.export'),
    ('game.analytics.relink'),
    ('game.audit.read'),
    ('game.sanction.read')
) AS defaults(permission) WHERE roles.key = 'game_security';

INSERT INTO roles (key, built_in) VALUES ('game_release', true);
INSERT INTO role_permissions (role_id, permission)
SELECT roles.id, permission FROM roles CROSS JOIN (VALUES
    ('game.node.deploy')
) AS defaults(permission) WHERE roles.key = 'game_release';
