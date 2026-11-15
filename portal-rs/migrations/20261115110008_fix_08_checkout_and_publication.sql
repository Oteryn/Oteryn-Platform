-- The provider key survives interruption between claim, intent and link.
-- Preserve existing unlinked claims: their old provider key is unknown.
ALTER TABLE store_web_topups ADD COLUMN intended_payment_intent_id uuid;
UPDATE store_web_topups SET intended_payment_intent_id=payment_intent_id
WHERE payment_intent_id IS NOT NULL;
CREATE UNIQUE INDEX store_web_topups_intended_payment ON store_web_topups(intended_payment_intent_id);

-- Publication progress is independent of immutable consumer windows.
ALTER TABLE loyalty_stream_head ADD COLUMN publication_after uuid;
CREATE INDEX referral_loyalty_credits_account ON referral_loyalty_credits(account_id);
CREATE INDEX coin_debt_settlements_reversal ON coin_debt_settlements(reversal_id);
CREATE INDEX coin_purchase_reversals_held_account ON coin_purchase_reversals(payer_account_id,created_at,id) WHERE outcome='held';
CREATE INDEX premium_ledger_publication ON premium_ledger(account_id,entitlement_id,effective_from,lifecycle_revision DESC) INCLUDE(effective_until);
