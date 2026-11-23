ALTER TABLE accounts
    ADD COLUMN mail_locale text CHECK (mail_locale IN ('pl', 'en'));

ALTER TABLE account_notification_prefs
    DROP CONSTRAINT account_notification_prefs_category_check,
    ADD CONSTRAINT account_notification_prefs_category_check
        CHECK (category IN ('security', 'news', 'support', 'events', 'forum_digest'));

CREATE INDEX account_notification_newsletter_audience
    ON account_notification_prefs (account_id) WHERE category = 'news' AND email_enabled;

CREATE TABLE newsletter_campaigns (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    created_by uuid NOT NULL,
    audience_locale text CHECK (audience_locale IN ('pl', 'en')),
    subject_pl text NOT NULL CHECK (octet_length(subject_pl) BETWEEN 1 AND 200),
    body_pl text NOT NULL CHECK (octet_length(body_pl) BETWEEN 1 AND 8000),
    subject_en text NOT NULL CHECK (octet_length(subject_en) BETWEEN 1 AND 200),
    body_en text NOT NULL CHECK (octet_length(body_en) BETWEEN 1 AND 8000),
    cursor uuid,
    eligible_count bigint NOT NULL DEFAULT 0,
    queued_count bigint NOT NULL DEFAULT 0 CHECK (queued_count BETWEEN 0 AND eligible_count),
    created_at timestamptz NOT NULL DEFAULT now(),
    started_at timestamptz,
    queued_at timestamptz,
    CHECK (queued_at IS NULL OR started_at IS NOT NULL)
);

CREATE TABLE newsletter_campaign_recipients (
    campaign_id uuid NOT NULL REFERENCES newsletter_campaigns (id) ON DELETE CASCADE,
    account_id uuid NOT NULL,
    PRIMARY KEY (campaign_id, account_id)
);

CREATE TABLE newsletter_campaign_tests (
    campaign_id uuid NOT NULL REFERENCES newsletter_campaigns (id) ON DELETE CASCADE,
    actor uuid NOT NULL,
    locale text NOT NULL CHECK (locale IN ('pl', 'en')),
    PRIMARY KEY (campaign_id, actor, locale)
);

-- Extend the installed vocabulary so earlier migrations can add their own grants.
DO $$
DECLARE
    existing_check text;
BEGIN
    SELECT pg_get_expr(conbin, conrelid) INTO STRICT existing_check
    FROM pg_constraint
    WHERE conrelid = 'role_permissions'::regclass
      AND conname = 'role_permissions_permission_check' AND contype = 'c';

    ALTER TABLE role_permissions DROP CONSTRAINT role_permissions_permission_check;
    EXECUTE format(
        'ALTER TABLE role_permissions ADD CONSTRAINT role_permissions_permission_check
         CHECK ((%s) OR permission IN (%L, %L, %L, %L))',
        existing_check, 'newsletter.view', 'newsletter.compose', 'newsletter.test', 'newsletter.send'
    );
END $$;

INSERT INTO role_permissions (role_id, permission)
SELECT id, unnest(ARRAY['newsletter.view', 'newsletter.compose', 'newsletter.test', 'newsletter.send'])
FROM roles WHERE key = 'platform_admin' AND built_in
ON CONFLICT DO NOTHING;

INSERT INTO role_permissions (role_id, permission)
SELECT id, unnest(ARRAY['newsletter.view', 'newsletter.compose', 'newsletter.test'])
FROM roles WHERE key = 'content_editor' AND built_in
ON CONFLICT DO NOTHING;
