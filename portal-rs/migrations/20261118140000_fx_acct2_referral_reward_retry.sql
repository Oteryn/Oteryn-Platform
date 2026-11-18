-- Existing awards keep their original due time via created_at.
ALTER TABLE referral_campaign_rewards ADD COLUMN next_attempt_at timestamptz;
ALTER TABLE referral_campaign_rewards ALTER COLUMN next_attempt_at SET DEFAULT now();
