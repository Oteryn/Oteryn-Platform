-- Staff release of a coin purchase hold (owner decision 2026-11-12: "the hold
-- is released by staff, through an RBAC action with an audit record").
-- Additive: one new permission and one history table.

-- A released hold is moved here, so the hold table keeps its one-row-per-account
-- meaning and a later insufficient reversal can place a new hold. The reversal
-- row (outcome 'held', debited 0) stays as the record of the unsettled debt;
-- releasing a hold settles nothing.
CREATE TABLE coin_purchase_hold_releases (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL CHECK (account_id <> '00000000-0000-0000-0000-000000000000'),
    order_id uuid NOT NULL REFERENCES coin_purchase_orders(id),
    reason text NOT NULL CHECK (reason IN ('reversal_insufficient_balance')),
    held_at timestamptz NOT NULL,
    released_at timestamptz NOT NULL DEFAULT now(),
    released_by uuid NOT NULL CHECK (released_by <> '00000000-0000-0000-0000-000000000000'),
    release_reason text NOT NULL CHECK (char_length(release_reason) BETWEEN 1 AND 500)
);
CREATE INDEX coin_purchase_hold_releases_account ON coin_purchase_hold_releases (account_id, released_at);
CREATE INDEX coin_purchase_hold_releases_order ON coin_purchase_hold_releases (order_id);

-- Dedicated staff permission (platform admin built-in only).
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
        'payments.coin_hold.release'
    ));
INSERT INTO role_permissions(role_id,permission)
    SELECT id,'payments.coin_hold.release' FROM roles
    WHERE key='platform_admin' AND built_in ON CONFLICT DO NOTHING;
