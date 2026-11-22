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
        'admin.roles.grant',
        'account.email.edit',
        'account.status.edit',
        'account.flags.edit'
    ));

INSERT INTO role_permissions (role_id, permission)
SELECT id, permission
FROM roles CROSS JOIN (VALUES
    ('account.email.edit'), ('account.status.edit'), ('account.flags.edit')
) AS edits(permission)
WHERE key = 'platform_admin' AND built_in
ON CONFLICT DO NOTHING;

CREATE TABLE staff_account_flags (
    account_id uuid PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    needs_review boolean NOT NULL
);

ALTER TABLE account_email_changes
    ADD COLUMN initiated_by_staff boolean NOT NULL DEFAULT false,
    ADD COLUMN owner_approved_at timestamptz CHECK (owner_approved_at IS NULL OR initiated_by_staff);

-- Cancel a pending assisted request on promotion. An applied change must
-- finish its reversal window before its account can receive a role.
-- Taking the same account lock as identity serializes with apply and revert.
CREATE FUNCTION cancel_staff_email_on_role_grant() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM id FROM accounts WHERE id = NEW.account_id FOR UPDATE;
    IF EXISTS (
        SELECT 1 FROM account_email_changes
        WHERE account_id = NEW.account_id AND initiated_by_staff
          AND applied_at IS NOT NULL AND reverted_at IS NULL
          AND revert_until > clock_timestamp()
    ) THEN
        RAISE EXCEPTION 'assisted email change is still reversible'
            USING ERRCODE = '23514', CONSTRAINT = 'role_grant_assisted_email_reversible';
    END IF;
    UPDATE account_email_changes SET cancelled_at = now(), updated_at = now()
    WHERE account_id = NEW.account_id AND initiated_by_staff
      AND applied_at IS NULL AND cancelled_at IS NULL AND reverted_at IS NULL;
    RETURN NEW;
END $$;

CREATE TRIGGER cancel_staff_email_on_role_grant
AFTER INSERT OR UPDATE OF account_id, role_id ON account_roles FOR EACH ROW
EXECUTE FUNCTION cancel_staff_email_on_role_grant();
