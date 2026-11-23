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
        'status.view',
        'status.incident.create',
        'status.incident.update',
        'status.incident.resolve',
        'status.maintenance.schedule'
    ));
INSERT INTO role_permissions (role_id, permission)
SELECT roles.id, grants.permission FROM roles CROSS JOIN (VALUES
    ('status.view'),
    ('status.incident.create'),
    ('status.incident.update'),
    ('status.incident.resolve'),
    ('status.maintenance.schedule')
) AS grants(permission)
WHERE roles.key='platform_admin' AND roles.built_in ON CONFLICT DO NOTHING;

CREATE TABLE status_incidents (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    -- A null world identifies the portal component.
    world_id uuid REFERENCES world_status(id),
    severity text NOT NULL CHECK (severity IN ('minor', 'major')),
    title_pl text NOT NULL CHECK (length(btrim(title_pl)) BETWEEN 1 AND 120),
    title_en text NOT NULL CHECK (length(btrim(title_en)) BETWEEN 1 AND 120),
    opened_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    resolved_at timestamptz CHECK (resolved_at >= opened_at)
);

CREATE INDEX status_incidents_open ON status_incidents (world_id, opened_at DESC)
    WHERE resolved_at IS NULL;
CREATE INDEX status_incidents_resolved ON status_incidents (resolved_at DESC, id DESC)
    WHERE resolved_at IS NOT NULL;

CREATE TABLE status_incident_updates (
    incident_id uuid NOT NULL REFERENCES status_incidents(id),
    sequence smallint NOT NULL CHECK (sequence BETWEEN 1 AND 100),
    posted_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    kind text NOT NULL CHECK (kind IN ('opened', 'updated', 'resolved')),
    message_pl text NOT NULL CHECK (length(btrim(message_pl)) BETWEEN 1 AND 2000),
    message_en text NOT NULL CHECK (length(btrim(message_en)) BETWEEN 1 AND 2000),
    PRIMARY KEY (incident_id, sequence)
);

CREATE TABLE status_maintenance (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    -- A null world covers all registered worlds.
    world_id uuid REFERENCES world_status(id),
    starts_at timestamptz NOT NULL,
    ends_at timestamptz NOT NULL CHECK (ends_at > starts_at),
    message_pl text NOT NULL CHECK (length(btrim(message_pl)) BETWEEN 1 AND 2000),
    message_en text NOT NULL CHECK (length(btrim(message_en)) BETWEEN 1 AND 2000)
);

CREATE INDEX status_maintenance_ends ON status_maintenance (ends_at);

CREATE TABLE status_publications (
    actor_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    publication_key uuid NOT NULL,
    command_digest bytea NOT NULL CHECK (octet_length(command_digest) = 32),
    result_id uuid NOT NULL,
    published_at timestamptz NOT NULL DEFAULT statement_timestamp(),
    PRIMARY KEY (actor_id, publication_key)
);

CREATE INDEX status_publications_actor_recent ON status_publications (actor_id, published_at);
