-- Account security revocation stream V1
-- (docs/contracts/OTERYN_V2_ACCOUNT_SECURITY_REVOCATION_STREAM_CONTRACT_PROPOSAL.md).
-- One transactional counter serializes publication until commit, as for N009.
-- A bare sequence would allow an older allocated number to commit behind a
-- consumer cursor. Rollback removes both the increment and the record.
CREATE TABLE identity_security_revocation_counter (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    position bigint NOT NULL DEFAULT 0 CHECK (position >= 0)
);
INSERT INTO identity_security_revocation_counter DEFAULT VALUES;
CREATE TABLE identity_security_revocations (
    position bigint PRIMARY KEY CHECK (position > 0),
    revocation_id uuid NOT NULL UNIQUE CHECK (uuid_extract_version(revocation_id) = 7),
    account_id uuid NOT NULL CHECK (uuid_extract_version(account_id) = 7),
    native_security_generation bigint NOT NULL CHECK (native_security_generation > 0),
    -- 1 credential, 2 status, 3 sanction, 4 staff_kick.
    cause smallint NOT NULL CHECK (cause BETWEEN 1 AND 4),
    effective_at_ms bigint NOT NULL CHECK (effective_at_ms > 0),
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX identity_security_revocations_account ON identity_security_revocations(account_id, position);

-- Callers hold the account row lock (or, for the read-time reconciler, the
-- counter lock) so generations are non-decreasing per account in stream order.
-- Native game evidence only admits UUIDv7 accounts; other ids hold no Game
-- session and publish nothing rather than failing the security change.
CREATE FUNCTION identity_security_revocation_append(account uuid, generation bigint, wire_cause smallint)
RETURNS void LANGUAGE plpgsql AS $$
DECLARE next_position bigint;
BEGIN
    IF uuid_extract_version(account) IS DISTINCT FROM 7 THEN
        RETURN;
    END IF;
    UPDATE identity_security_revocation_counter SET position = position + 1
        WHERE singleton RETURNING position INTO next_position;
    INSERT INTO identity_security_revocations
        (position,revocation_id,account_id,native_security_generation,cause,effective_at_ms)
    VALUES (next_position,uuidv7(),account,generation,wire_cause,
        floor(extract(epoch FROM clock_timestamp())*1000)::bigint);
END $$;

-- (a) Every accounts UPDATE that advances native_security_generation, whatever
-- its writer. Deterministic cause mapping, first match wins:
--   status changed                                             -> 2 status
--   password_hash, email, email_verified_at or
--   login_mfa_required changed                                 -> 1 credential
--   generation-only advance by the staff kick (transaction-local
--   setting oteryn.security_revocation_cause = 'staff_kick')    -> 4 staff_kick
--   generation-only advance from a nested trigger (the confirmed
--   TOTP replace/remove fence fence_native_device_mfa)          -> 1 credential
--   any other generation-only advance (recovery-key use,
--   termination request)                                       -> 2 status
CREATE FUNCTION identity_security_revocation_account() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE wire_cause smallint;
BEGIN
    IF NEW.status IS DISTINCT FROM OLD.status THEN
        wire_cause := 2;
    ELSIF NEW.password_hash IS DISTINCT FROM OLD.password_hash
       OR NEW.email IS DISTINCT FROM OLD.email
       OR NEW.email_verified_at IS DISTINCT FROM OLD.email_verified_at
       OR NEW.login_mfa_required IS DISTINCT FROM OLD.login_mfa_required THEN
        wire_cause := 1;
    ELSIF current_setting('oteryn.security_revocation_cause', true) = 'staff_kick' THEN
        wire_cause := 4;
    ELSIF pg_trigger_depth() > 1 THEN
        wire_cause := 1;
    ELSE
        wire_cause := 2;
    END IF;
    PERFORM identity_security_revocation_append(NEW.id,NEW.native_security_generation,wire_cause);
    RETURN NULL;
END $$;
CREATE TRIGGER identity_security_revocation_account AFTER UPDATE ON accounts
    FOR EACH ROW WHEN (NEW.native_security_generation > OLD.native_security_generation)
    EXECUTE FUNCTION identity_security_revocation_account();

-- (b) Cause 3: an account-level sanction that `require_game_access` enforces
-- becomes effective. Expiry, lifting or revocation publishes nothing, and the
-- generation is not advanced (the consumer ends every session of the account).
-- Legacy T34 suspensions publish once per sanction id; the marker also lets the
-- read-time reconciler publish a future-dated suspension when it takes effect.
CREATE TABLE identity_security_revocation_suspensions (
    sanction_id uuid PRIMARY KEY,
    created_at timestamptz NOT NULL DEFAULT now()
);
-- Suspensions already effective before this journal existed are not replayed.
INSERT INTO identity_security_revocation_suspensions(sanction_id)
    SELECT id FROM sanctions WHERE category='suspension' AND status='active'
        AND effective_at <= now() AND (expires_at IS NULL OR expires_at > now());
CREATE INDEX identity_security_revocation_pending_suspensions ON sanctions(effective_at, id)
    WHERE category='suspension' AND status='active';

CREATE FUNCTION identity_security_revocation_suspension_publish(sanction uuid, account uuid)
RETURNS void LANGUAGE plpgsql AS $$
DECLARE generation bigint;
BEGIN
    -- Account before counter, matching the accounts trigger and sanction writers.
    SELECT native_security_generation INTO generation FROM accounts WHERE id=account FOR UPDATE;
    PERFORM 1 FROM identity_security_revocation_counter WHERE singleton FOR UPDATE;
    IF NOT FOUND OR EXISTS (SELECT 1 FROM identity_security_revocation_suspensions
        WHERE sanction_id=sanction) THEN
        RETURN;
    END IF;
    PERFORM identity_security_revocation_append(account,generation,3::smallint);
    INSERT INTO identity_security_revocation_suspensions(sanction_id) VALUES (sanction);
END $$;
CREATE FUNCTION identity_security_revocation_suspension() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.category = 'suspension' AND NEW.status = 'active'
       AND NEW.effective_at <= clock_timestamp()
       AND (NEW.expires_at IS NULL OR NEW.expires_at > clock_timestamp())
       -- Unlocked fast path for later edits; the publisher re-checks under lock.
       AND NOT EXISTS (SELECT 1 FROM identity_security_revocation_suspensions
           WHERE sanction_id=NEW.id) THEN
        PERFORM identity_security_revocation_suspension_publish(NEW.id,NEW.account_id);
    END IF;
    RETURN NULL;
END $$;
CREATE TRIGGER identity_security_revocation_suspension AFTER INSERT OR UPDATE ON sanctions
    FOR EACH ROW EXECUTE FUNCTION identity_security_revocation_suspension();

-- Typed bans publish on the revision that moves a ban from not effective (none,
-- inactive or expired) to effective. Extensions of an effective ban do not.
CREATE FUNCTION identity_security_revocation_ban() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    previous support_sanction_revisions%ROWTYPE;
    generation bigint;
BEGIN
    IF NOT NEW.active OR (NEW.expires_at IS NOT NULL AND NEW.expires_at <= clock_timestamp())
       OR NOT EXISTS (SELECT 1 FROM support_sanction_identities
           WHERE sanction_id=NEW.sanction_id AND kind='ban') THEN
        RETURN NULL;
    END IF;
    SELECT * INTO previous FROM support_sanction_revisions
        WHERE account_id=NEW.account_id AND sanction_id=NEW.sanction_id AND revision<NEW.revision
        ORDER BY revision DESC LIMIT 1;
    IF FOUND AND previous.active
       AND (previous.expires_at IS NULL OR previous.expires_at > clock_timestamp()) THEN
        RETURN NULL;
    END IF;
    SELECT native_security_generation INTO generation FROM accounts WHERE id=NEW.account_id FOR UPDATE;
    PERFORM identity_security_revocation_append(NEW.account_id,generation,3::smallint);
    RETURN NULL;
END $$;
CREATE TRIGGER identity_security_revocation_ban AFTER INSERT ON support_sanction_revisions
    FOR EACH ROW EXECUTE FUNCTION identity_security_revocation_ban();

-- Before each page read: publish legacy suspensions whose effective_at has
-- elapsed since they were written. A busy publisher defers this to a later
-- poll; the reader never locks accounts (writers take account then counter).
CREATE FUNCTION identity_security_revocation_reconcile() RETURNS void LANGUAGE plpgsql AS $$
DECLARE item record;
BEGIN
    PERFORM 1 FROM identity_security_revocation_counter WHERE singleton FOR UPDATE SKIP LOCKED;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    FOR item IN
        SELECT s.id, s.account_id, a.native_security_generation AS generation
        FROM sanctions s JOIN accounts a ON a.id=s.account_id
        WHERE s.category='suspension' AND s.status='active'
            AND s.effective_at <= clock_timestamp()
            AND (s.expires_at IS NULL OR s.expires_at > clock_timestamp())
            AND NOT EXISTS (SELECT 1 FROM identity_security_revocation_suspensions m
                WHERE m.sanction_id=s.id)
        ORDER BY s.effective_at, s.id LIMIT 500
    LOOP
        PERFORM identity_security_revocation_append(item.account_id,item.generation,3::smallint);
        INSERT INTO identity_security_revocation_suspensions(sanction_id) VALUES (item.id);
    END LOOP;
END $$;

-- (c) The staff kick advances the generation through the accounts trigger above
-- with cause 4; see oteryn_identity::security_revocations::staff_kick.

CREATE FUNCTION identity_security_revocation_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'security revocation journal is append only' USING ERRCODE = '23514';
END $$;
CREATE TRIGGER identity_security_revocation_immutable BEFORE UPDATE OR DELETE ON identity_security_revocations
    FOR EACH ROW EXECUTE FUNCTION identity_security_revocation_immutable();

-- Dedicated staff kick permission (moderator and platform admin built-ins).
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
        'account.security.kick'
    ));
INSERT INTO role_permissions(role_id,permission)
    SELECT id,'account.security.kick' FROM roles
    WHERE key IN ('moderator','platform_admin') AND built_in ON CONFLICT DO NOTHING;
