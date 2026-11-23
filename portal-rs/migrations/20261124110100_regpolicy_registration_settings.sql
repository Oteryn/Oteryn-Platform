-- Preserve the existing closed default and the limits already enforced for open signup.
ALTER TABLE registration_abuse_policy
    ADD COLUMN reason_pl text NOT NULL DEFAULT '' CHECK (char_length(reason_pl) <= 500),
    ADD COLUMN reason_en text NOT NULL DEFAULT '' CHECK (char_length(reason_en) <= 500),
    ADD COLUMN work_bits smallint NOT NULL DEFAULT 16 CHECK (work_bits BETWEEN 16 AND 18),
    ADD COLUMN work_ttl_seconds integer NOT NULL DEFAULT 300 CHECK (work_ttl_seconds BETWEEN 300 AND 900),
    ADD COLUMN challenge_limit smallint NOT NULL DEFAULT 5 CHECK (challenge_limit BETWEEN 1 AND 5),
    ADD COLUMN attempt_limit smallint NOT NULL DEFAULT 5 CHECK (attempt_limit BETWEEN 1 AND 5),
    ADD COLUMN rate_window_seconds integer NOT NULL DEFAULT 60 CHECK (rate_window_seconds BETWEEN 60 AND 3600),
    ADD COLUMN revision bigint NOT NULL DEFAULT 0 CHECK (revision >= 0);

CREATE INDEX audit_log_registration_policy_history_idx ON audit_log (created_at DESC, id DESC)
    WHERE action = 'registration.policy.changed' AND target = 'registration_abuse_policy';

-- Track bulk imports through the existing importer, including its upsert path.
-- Legacy ranges have no source marker and therefore no known import timestamp.
CREATE TABLE registration_corpus_import_status (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    imported_at timestamptz NOT NULL
);

CREATE FUNCTION record_registration_corpus_import() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF current_setting('oteryn.registration_range_source', true) IS DISTINCT FROM 'cache'
       AND EXISTS (SELECT 1 FROM imported_ranges WHERE complete) THEN
        INSERT INTO registration_corpus_import_status (imported_at)
        VALUES (clock_timestamp())
        ON CONFLICT (singleton) DO UPDATE
        SET imported_at = greatest(registration_corpus_import_status.imported_at, EXCLUDED.imported_at);
    END IF;
    RETURN NULL;
END
$$;

CREATE TRIGGER registration_corpus_insert
    AFTER INSERT ON registration_password_ranges
    REFERENCING NEW TABLE AS imported_ranges
    FOR EACH STATEMENT EXECUTE FUNCTION record_registration_corpus_import();

CREATE TRIGGER registration_corpus_update
    AFTER UPDATE ON registration_password_ranges
    REFERENCING NEW TABLE AS imported_ranges
    FOR EACH STATEMENT EXECUTE FUNCTION record_registration_corpus_import();

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
        'registration.policy.view',
        'registration.policy.manage'
    ));

INSERT INTO role_permissions (role_id, permission)
SELECT id, permission
FROM roles CROSS JOIN (VALUES ('registration.policy.view'), ('registration.policy.manage')) AS grants(permission)
WHERE key = 'platform_admin' AND built_in
ON CONFLICT DO NOTHING;
