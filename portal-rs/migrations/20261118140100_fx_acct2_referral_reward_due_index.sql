-- no-transaction
CREATE INDEX CONCURRENTLY referral_campaign_premium_due
    ON referral_campaign_rewards (COALESCE(next_attempt_at, created_at), created_at, id)
    WHERE kind = 'premium' AND applied_at IS NULL;
