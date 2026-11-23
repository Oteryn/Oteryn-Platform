ALTER TABLE account_notification_prefs ADD CONSTRAINT account_notification_prefs_operational_email_only
    CHECK (category <> 'operational' OR NOT portal_enabled);
CREATE INDEX operational_mail_opted_in ON account_notification_prefs(account_id)
    WHERE category='operational' AND email_enabled;

ALTER TABLE email_outbox ADD CONSTRAINT email_outbox_operational_unsubscribe
    CHECK (category <> 'operational' OR unsubscribe_url IS NOT NULL);

CREATE TABLE operational_mail_campaigns (
    id uuid PRIMARY KEY,
    author uuid NOT NULL,
    audience text NOT NULL CHECK (audience IN ('all','pl','en')),
    subject_pl text NOT NULL CHECK (char_length(subject_pl) BETWEEN 1 AND 200),
    body_pl text NOT NULL CHECK (char_length(body_pl) BETWEEN 1 AND 4000),
    subject_en text NOT NULL CHECK (char_length(subject_en) BETWEEN 1 AND 200),
    body_en text NOT NULL CHECK (char_length(body_en) BETWEEN 1 AND 4000),
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL DEFAULT (now() + interval '24 hours'),
    decision text CHECK (decision IN ('approved','rejected')),
    reviewer uuid,
    reviewed_at timestamptz,
    CHECK (expires_at > created_at),
    CHECK (reviewer <> author),
    CHECK ((decision IS NULL AND reviewer IS NULL AND reviewed_at IS NULL)
        OR (decision IS NOT NULL AND reviewer IS NOT NULL AND reviewed_at IS NOT NULL))
);
CREATE INDEX operational_mail_campaigns_author_created ON operational_mail_campaigns(author, created_at DESC);

CREATE TABLE operational_mail_recipients (
    campaign_id uuid NOT NULL REFERENCES operational_mail_campaigns(id),
    account_id uuid NOT NULL,
    locale text NOT NULL CHECK (locale IN ('pl','en')),
    address_digest bytea NOT NULL CHECK (octet_length(address_digest)=32),
    verified_at timestamptz NOT NULL,
    consent_at timestamptz NOT NULL,
    mail_id uuid UNIQUE REFERENCES email_outbox(id),
    job_id uuid UNIQUE,
    skipped_at timestamptz,
    PRIMARY KEY (campaign_id, account_id),
    CHECK ((mail_id IS NULL) = (job_id IS NULL)),
    CHECK (skipped_at IS NULL OR mail_id IS NOT NULL)
);
