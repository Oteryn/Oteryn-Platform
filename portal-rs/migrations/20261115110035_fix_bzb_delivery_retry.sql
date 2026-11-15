-- Failed deliveries retain their payload and stable identity for recovery.
ALTER TABLE referral_email_attempts ADD COLUMN next_attempt_at timestamptz NOT NULL DEFAULT '-infinity', ADD COLUMN delivery_attempts integer NOT NULL DEFAULT 0;
CREATE INDEX referral_email_attempts_retry ON referral_email_attempts(next_attempt_at,created_at) WHERE delivered_at IS NULL;
ALTER TABLE account_email_change_mail ADD COLUMN next_attempt_at timestamptz NOT NULL DEFAULT '-infinity', ADD COLUMN delivery_attempts integer NOT NULL DEFAULT 0;
CREATE INDEX account_email_change_mail_retry ON account_email_change_mail(next_attempt_at,created_at) WHERE delivered_at IS NULL;
