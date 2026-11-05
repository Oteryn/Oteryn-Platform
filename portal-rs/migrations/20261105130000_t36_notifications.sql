CREATE TABLE account_notification_prefs (
    account_id uuid NOT NULL,
    category text NOT NULL CHECK (category IN ('security', 'news', 'support')),
    email_enabled boolean NOT NULL DEFAULT false,
    portal_enabled boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, category),
    CHECK (category <> 'security' OR (email_enabled AND portal_enabled))
);
CREATE TABLE notifications (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL,
    category text NOT NULL CHECK (category IN ('security', 'news', 'support')),
    subject text NOT NULL,
    body text NOT NULL,
    read_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX notifications_account_unread ON notifications (account_id, created_at DESC);
CREATE TABLE email_outbox (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL,
    category text NOT NULL CHECK (category IN ('security', 'news', 'support')),
    recipient text NOT NULL,
    subject text NOT NULL,
    text_body text NOT NULL,
    html_body text NOT NULL,
    unsubscribe_url text,
    delivered_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (category <> 'security' OR unsubscribe_url IS NULL)
);
