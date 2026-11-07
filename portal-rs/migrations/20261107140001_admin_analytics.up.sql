-- Aggregate-only consumer. Fences deliberately live outside disposable generations.
SELECT pg_advisory_xact_lock(71140002);
CREATE SCHEMA IF NOT EXISTS admin_analytics;
REVOKE ALL ON SCHEMA admin_analytics FROM PUBLIC;
CREATE TABLE IF NOT EXISTS admin_analytics.fence (
 authority text PRIMARY KEY, epoch numeric(20,0) NOT NULL CHECK(epoch>0),
 privacy_floor text NOT NULL, active_generation uuid,
 rate_window bigint NOT NULL DEFAULT 0, rate_count integer NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS admin_analytics.generations (
 id uuid PRIMARY KEY, authority text NOT NULL REFERENCES admin_analytics.fence(authority),
 epoch numeric(20,0) NOT NULL, baseline_digest text, baseline_cut numeric(20,0),
 accepted_at timestamptz, created_at timestamptz NOT NULL DEFAULT now(),
 CHECK ((accepted_at IS NULL AND baseline_digest IS NULL AND baseline_cut IS NULL) OR
        (accepted_at IS NOT NULL AND length(baseline_digest)=64 AND baseline_cut>0))
);
CREATE TABLE IF NOT EXISTS admin_analytics.subjects (
 authority text NOT NULL, family text NOT NULL, world uuid NOT NULL, bucket_start numeric(20,0) NOT NULL,
 bucket_end numeric(20,0) NOT NULL, epoch numeric(20,0) NOT NULL, revision numeric(20,0) NOT NULL,
 privacy text NOT NULL, observed numeric(20,0) NOT NULL, commit_digest text NOT NULL, generation uuid NOT NULL REFERENCES admin_analytics.generations(id),
 conflict boolean NOT NULL DEFAULT false, erased boolean NOT NULL,
 PRIMARY KEY(authority,family,world,bucket_start)
);
CREATE TABLE IF NOT EXISTS admin_analytics.pages (
 authority text NOT NULL, family text NOT NULL, world uuid NOT NULL, bucket_start numeric(20,0) NOT NULL,
 epoch numeric(20,0) NOT NULL, revision numeric(20,0) NOT NULL, page_index integer NOT NULL,
 digest text NOT NULL, body bytea NOT NULL CHECK(octet_length(body)<=131072),
 staged_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(authority,family,world,bucket_start,epoch,revision,page_index)
);
CREATE INDEX IF NOT EXISTS admin_analytics_staging_expiry ON admin_analytics.pages(staged_at);
CREATE TABLE IF NOT EXISTS admin_analytics.page_receipts (
 authority text NOT NULL, family text NOT NULL, world uuid NOT NULL, bucket_start numeric(20,0) NOT NULL,
 epoch numeric(20,0) NOT NULL, revision numeric(20,0) NOT NULL, page_index integer NOT NULL, digest text NOT NULL,
 PRIMARY KEY(authority,family,world,bucket_start,epoch,revision,page_index)
);
CREATE TABLE IF NOT EXISTS admin_analytics.receipts (
 authority text NOT NULL, family text NOT NULL, world uuid NOT NULL, bucket_start numeric(20,0) NOT NULL,
 epoch numeric(20,0) NOT NULL, revision numeric(20,0) NOT NULL, digest text NOT NULL,
 PRIMARY KEY(authority,family,world,bucket_start,epoch,revision)
);
CREATE TABLE IF NOT EXISTS admin_analytics.rows (
 generation uuid NOT NULL REFERENCES admin_analytics.generations(id), authority text NOT NULL,
 family text NOT NULL, world uuid NOT NULL, bucket_start numeric(20,0) NOT NULL,
 bucket_end bigint NOT NULL, key text NOT NULL, state text NOT NULL CHECK(state IN ('visible','suppressed','incomplete')),
 values jsonb NOT NULL, epoch numeric(20,0) NOT NULL, revision numeric(20,0) NOT NULL,
 privacy text NOT NULL, observed numeric(20,0) NOT NULL,
 PRIMARY KEY(generation,authority,family,world,bucket_start,key,bucket_end)
) PARTITION BY RANGE(bucket_end);
CREATE TABLE IF NOT EXISTS admin_analytics.rows_retained PARTITION OF admin_analytics.rows DEFAULT;
CREATE INDEX IF NOT EXISTS admin_analytics_read ON admin_analytics.rows(authority,family,world,bucket_start,key);
CREATE TABLE IF NOT EXISTS admin_analytics.watermarks (
 authority text NOT NULL, family text NOT NULL, world uuid NOT NULL, epoch numeric(20,0) NOT NULL,
 privacy text NOT NULL, cut numeric(20,0) NOT NULL, observed numeric(20,0) NOT NULL,
 digest text NOT NULL, conflict boolean NOT NULL DEFAULT false,
 PRIMARY KEY(authority,family,world)
);
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
        'game.audit.read'
    ));
INSERT INTO role_permissions(role_id,permission)
 SELECT id,p FROM roles CROSS JOIN (VALUES ('admin.analytics.view'),('admin.analytics.export')) AS permissions(p)
 WHERE key='platform_admin' AND built_in ON CONFLICT DO NOTHING;
-- Defense in depth for every SQL writer, in addition to the role-editing API.
CREATE OR REPLACE FUNCTION admin_analytics.restrict_grants() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.permission IN ('admin.analytics.view','admin.analytics.export') AND NOT EXISTS
 (SELECT 1 FROM roles WHERE id=NEW.role_id AND key='platform_admin' AND built_in) THEN
  RAISE EXCEPTION 'analytics permissions require platform_admin';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER admin_analytics_grants BEFORE INSERT OR UPDATE ON role_permissions
 FOR EACH ROW EXECUTE FUNCTION admin_analytics.restrict_grants();
REVOKE ALL ON ALL TABLES IN SCHEMA admin_analytics FROM PUBLIC;
