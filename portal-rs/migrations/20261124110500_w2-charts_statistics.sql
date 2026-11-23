-- One UTC bucket per day. No account, credential, session or source identifiers.
CREATE TABLE dashboard_login_failures (
    day date PRIMARY KEY,
    failures bigint NOT NULL CHECK (failures >= 0)
);
INSERT INTO dashboard_login_failures (day, failures)
VALUES ((now() AT TIME ZONE 'UTC')::date, 0);

CREATE INDEX charts_accounts_created ON accounts (created_at);
CREATE INDEX charts_accounts_verified ON accounts (email_verified_at) WHERE email_verified_at IS NOT NULL;
CREATE INDEX charts_web_logins_succeeded ON security_events (created_at) WHERE kind = 'login_succeeded';
CREATE INDEX charts_web_sessions_active ON web_sessions (idle_expires_at, absolute_expires_at) WHERE account_id IS NOT NULL;
CREATE INDEX charts_tickets_opened ON tickets (created_at);
CREATE INDEX charts_tickets_resolved ON audit_log (created_at)
    WHERE action = 'support.ticket_status_changed'
      AND after->>'status' = 'resolved' AND before->>'status' IS DISTINCT FROM 'resolved';
CREATE INDEX charts_ticket_public_replies ON ticket_messages (ticket_id, created_at)
    WHERE author_kind = 'staff' AND NOT internal;
CREATE INDEX charts_sanctions_issued ON audit_log (created_at)
    WHERE action = 'support.enforcement_approved'
       OR (action = 'support.enforcement_created'
           AND (after->>'status' = 'active' OR after->>'pending' = 'false'));
CREATE INDEX charts_wallet_operations ON wallet_operations (created_at);
CREATE INDEX charts_world_rollup_days ON stats_world_daily (day);
CREATE INDEX charts_world_names ON world_status (name, id);
