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
        'forum.moderate',
        'forum.board.create',
        'forum.board.edit',
        'forum.thread.create',
        'forum.post.create',
        'forum.thread.lock',
        'forum.thread.pin',
        'forum.thread.move',
        'forum.thread.delete',
        'forum.thread.restore',
        'forum.thread.edit',
        'forum.post.delete',
        'forum.post.restore',
        'forum.post.edit',
        'forum.subscription.edit',
        'forum.profile.edit'
    ));

INSERT INTO role_permissions (role_id, permission)
SELECT id, permission FROM roles CROSS JOIN (VALUES
    ('forum.moderate'),
    ('forum.board.create'),
    ('forum.board.edit'),
    ('forum.thread.create'),
    ('forum.post.create'),
    ('forum.thread.lock'),
    ('forum.thread.pin'),
    ('forum.thread.move'),
    ('forum.thread.delete'),
    ('forum.thread.restore'),
    ('forum.thread.edit'),
    ('forum.post.delete'),
    ('forum.post.restore'),
    ('forum.post.edit'),
    ('forum.subscription.edit'),
    ('forum.profile.edit')
) AS forum_permissions(permission)
WHERE built_in AND (key='platform_admin' OR (key='moderator' AND permission NOT LIKE 'forum.board.%'))
ON CONFLICT DO NOTHING;

CREATE TABLE forum_boards (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    name_pl text NOT NULL CHECK (char_length(btrim(name_pl)) BETWEEN 1 AND 100),
    name_en text NOT NULL CHECK (char_length(btrim(name_en)) BETWEEN 1 AND 100),
    position integer NOT NULL CHECK (position BETWEEN 0 AND 10000),
    visibility text NOT NULL CHECK (visibility IN ('public', 'signed-in', 'staff')),
    version bigint NOT NULL DEFAULT 1
);
CREATE INDEX forum_boards_order ON forum_boards(position, id);

CREATE TABLE forum_profiles (
    account_id uuid PRIMARY KEY REFERENCES accounts(id),
    public_handle text NOT NULL CHECK (public_handle ~ '^[A-Za-z][A-Za-z0-9_-]{2,23}$')
);
CREATE UNIQUE INDEX forum_profiles_public_handle ON forum_profiles(lower(public_handle));

CREATE TABLE forum_threads (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    board_id uuid NOT NULL REFERENCES forum_boards(id),
    author_id uuid NOT NULL REFERENCES accounts(id),
    title text NOT NULL CHECK (char_length(btrim(title)) BETWEEN 1 AND 160),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    activity_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    locked boolean NOT NULL DEFAULT false,
    pinned boolean NOT NULL DEFAULT false,
    deleted_at timestamptz,
    last_position bigint NOT NULL DEFAULT 0,
    version bigint NOT NULL DEFAULT 1
);
CREATE INDEX forum_threads_order ON forum_threads(board_id, pinned DESC, activity_at DESC, id DESC);

CREATE TABLE forum_posts (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    thread_id uuid NOT NULL REFERENCES forum_threads(id),
    author_id uuid NOT NULL REFERENCES accounts(id),
    position bigint NOT NULL CHECK (position > 0),
    body text NOT NULL CHECK (char_length(btrim(body)) BETWEEN 1 AND 8000),
    quote_id uuid,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    edited_at timestamptz,
    deleted_at timestamptz,
    version bigint NOT NULL DEFAULT 1,
    UNIQUE (thread_id, id),
    UNIQUE (thread_id, position),
    FOREIGN KEY (thread_id, quote_id) REFERENCES forum_posts(thread_id, id)
);
CREATE INDEX forum_posts_author_rate ON forum_posts(author_id, created_at DESC);

CREATE TABLE forum_read_markers (
    account_id uuid NOT NULL REFERENCES accounts(id),
    thread_id uuid NOT NULL REFERENCES forum_threads(id),
    position bigint NOT NULL CHECK (position > 0),
    PRIMARY KEY (account_id, thread_id)
);
CREATE TABLE forum_subscriptions (
    thread_id uuid NOT NULL REFERENCES forum_threads(id),
    account_id uuid NOT NULL REFERENCES accounts(id),
    subscribed_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (thread_id, account_id)
);
CREATE INDEX forum_subscription_account ON forum_subscriptions(account_id);

ALTER TABLE player_reports ADD COLUMN forum_post_id uuid REFERENCES forum_posts(id);
CREATE UNIQUE INDEX forum_reports_actor_post ON player_reports(reporter_account_id, forum_post_id)
    WHERE forum_post_id IS NOT NULL;

ALTER TABLE account_notification_prefs DROP CONSTRAINT account_notification_prefs_category_check;
ALTER TABLE account_notification_prefs ADD CONSTRAINT account_notification_prefs_category_check
    CHECK (category IN ('security', 'news', 'support', 'forum'));
ALTER TABLE email_outbox DROP CONSTRAINT email_outbox_category_check;
ALTER TABLE email_outbox ADD CONSTRAINT email_outbox_category_check
    CHECK (category IN ('security', 'news', 'support', 'forum'));

CREATE TABLE forum_mail_batches (
    thread_id uuid NOT NULL REFERENCES forum_threads(id),
    account_id uuid NOT NULL,
    window_starts timestamptz NOT NULL,
    email_id uuid NOT NULL UNIQUE REFERENCES email_outbox(id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED,
    visibility text NOT NULL CHECK (visibility IN ('public', 'signed-in', 'staff')),
    PRIMARY KEY (thread_id, account_id, window_starts)
);
