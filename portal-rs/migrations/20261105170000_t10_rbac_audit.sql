-- Account identities are owned by a future identity migration; UUIDs deliberately have no FK.
CREATE TABLE roles (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    key text NOT NULL UNIQUE CHECK (key ~ '^[a-z][a-z0-9_]{0,63}$'),
    built_in boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE role_permissions (
    role_id uuid NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission text NOT NULL CHECK (permission IN (
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
        'wiki.publish'
    )),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (role_id, permission)
);
CREATE TABLE account_roles (
    account_id uuid NOT NULL,
    role_id uuid NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, role_id)
);
CREATE INDEX account_roles_role_idx ON account_roles(role_id);
CREATE TABLE audit_log (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    actor uuid,
    action text NOT NULL CHECK (length(btrim(action)) BETWEEN 1 AND 128),
    target text NOT NULL CHECK (length(btrim(target)) BETWEEN 1 AND 256),
    before jsonb NOT NULL,
    after jsonb NOT NULL,
    request_id uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX audit_log_actor_created_idx ON audit_log(actor, created_at);
CREATE INDEX audit_log_request_idx ON audit_log(request_id);

CREATE FUNCTION reject_audit_log_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'audit_log is append-only' USING ERRCODE = '42501';
END;
$$;
CREATE TRIGGER audit_log_append_only
    BEFORE UPDATE OR DELETE OR TRUNCATE ON audit_log
    FOR EACH STATEMENT EXECUTE FUNCTION reject_audit_log_mutation();

INSERT INTO roles (key, built_in) VALUES ('platform_admin', true);
INSERT INTO role_permissions (role_id, permission)
SELECT roles.id, permission FROM roles CROSS JOIN (VALUES
    ('admin.access'),
    ('admin.roles.manage'),
    ('audit.view'),
    ('cms.news.manage'),
    ('cms.pages.manage'),
    ('media.manage'),
    ('portal.access'),
    ('portal.announcements.manage'),
    ('portal.settings.manage'),
    ('downloads.manage'),
    ('events.manage'),
    ('events.publish'),
    ('support.content.manage'),
    ('support.tickets.manage'),
    ('support.reports.manage'),
    ('support.enforcement.manage'),
    ('marketplace.manage'),
    ('products.premium.manage'),
    ('payments.reconcile'),
    ('game_catalog.access'),
    ('game_catalog.snapshots.view'),
    ('game_catalog.snapshots.import'),
    ('game_catalog.snapshots.activate'),
    ('game_catalog.profiles.manage'),
    ('game_catalog.translations.manage'),
    ('game_catalog.overrides.manage'),
    ('wiki.access'),
    ('wiki.articles.manage'),
    ('wiki.categories.manage'),
    ('wiki.publish')
) AS defaults(permission) WHERE roles.key = 'platform_admin';

INSERT INTO roles (key, built_in) VALUES ('security_admin', true);
INSERT INTO role_permissions (role_id, permission)
SELECT roles.id, permission FROM roles CROSS JOIN (VALUES
    ('admin.access'),
    ('admin.roles.manage'),
    ('audit.view')
) AS defaults(permission) WHERE roles.key = 'security_admin';

INSERT INTO roles (key, built_in) VALUES ('content_editor', true);
INSERT INTO role_permissions (role_id, permission)
SELECT roles.id, permission FROM roles CROSS JOIN (VALUES
    ('cms.news.manage'),
    ('cms.pages.manage'),
    ('media.manage'),
    ('portal.announcements.manage'),
    ('events.manage'),
    ('events.publish'),
    ('support.content.manage'),
    ('wiki.access'),
    ('wiki.articles.manage'),
    ('wiki.categories.manage'),
    ('wiki.publish'),
    ('admin.access'),
    ('portal.access')
) AS defaults(permission) WHERE roles.key = 'content_editor';

INSERT INTO roles (key, built_in) VALUES ('support', true);
INSERT INTO role_permissions (role_id, permission)
SELECT roles.id, permission FROM roles CROSS JOIN (VALUES
    ('admin.access'),
    ('support.tickets.manage'),
    ('support.reports.manage')
) AS defaults(permission) WHERE roles.key = 'support';

INSERT INTO roles (key, built_in) VALUES ('moderator', true);
INSERT INTO role_permissions (role_id, permission)
SELECT roles.id, permission FROM roles CROSS JOIN (VALUES
    ('admin.access'),
    ('support.enforcement.manage'),
    ('marketplace.manage')
) AS defaults(permission) WHERE roles.key = 'moderator';

INSERT INTO roles (key, built_in) VALUES ('operations', true);
INSERT INTO role_permissions (role_id, permission)
SELECT roles.id, permission FROM roles CROSS JOIN (VALUES
    ('admin.access'),
    ('downloads.manage'),
    ('portal.access'),
    ('portal.settings.manage'),
    ('game_catalog.access'),
    ('game_catalog.snapshots.view'),
    ('game_catalog.snapshots.import'),
    ('game_catalog.snapshots.activate'),
    ('game_catalog.profiles.manage'),
    ('game_catalog.translations.manage'),
    ('game_catalog.overrides.manage')
) AS defaults(permission) WHERE roles.key = 'operations';
