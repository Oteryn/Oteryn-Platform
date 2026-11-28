ALTER TABLE account_notification_prefs
    DROP CONSTRAINT account_notification_prefs_category_check,
    ADD CONSTRAINT account_notification_prefs_category_check
        CHECK (category IN ('security', 'news', 'support', 'events', 'forum_digest', 'forum', 'operational', 'journal_reminder')),
    ADD CONSTRAINT account_notification_prefs_journal_reminder_email_only
        CHECK (category <> 'journal_reminder' OR NOT portal_enabled);

ALTER TABLE email_outbox
    ADD COLUMN journal_reminder_intent uuid,
    ADD COLUMN journal_reminder_suppressed_at timestamptz,
    DROP CONSTRAINT email_outbox_category_check,
    ADD CONSTRAINT email_outbox_category_check
        CHECK (category IN ('security', 'news', 'support', 'operations', 'forum', 'operational', 'journal_reminder')),
    DROP CONSTRAINT email_outbox_template_kind_check,
    ADD CONSTRAINT email_outbox_template_kind_check
        CHECK (template_kind IN ('new-device', 'password-changed', 'news', 'support', 'journal-reminder')),
    ADD CONSTRAINT email_outbox_journal_reminder_reference CHECK (
        (category = 'journal_reminder') = (journal_reminder_intent IS NOT NULL)
        AND (category = 'journal_reminder') = (template_kind IS NOT DISTINCT FROM 'journal-reminder')
        AND (category <> 'journal_reminder' OR unsubscribe_url IS NOT NULL)
        AND (category = 'journal_reminder' OR journal_reminder_suppressed_at IS NULL)
    );

CREATE UNIQUE INDEX email_outbox_journal_reminder_once
    ON email_outbox (account_id, journal_reminder_intent)
    WHERE journal_reminder_intent IS NOT NULL;
