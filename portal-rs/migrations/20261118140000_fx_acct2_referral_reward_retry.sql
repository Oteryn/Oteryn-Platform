ALTER TABLE referral_campaign_rewards
    ADD COLUMN next_attempt_at timestamptz NOT NULL DEFAULT '-infinity';

CREATE INDEX referral_campaign_premium_due
    ON referral_campaign_rewards (next_attempt_at, created_at, id)
    WHERE kind = 'premium' AND applied_at IS NULL;
