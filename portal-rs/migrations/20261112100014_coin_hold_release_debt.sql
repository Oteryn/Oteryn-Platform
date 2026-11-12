-- Coordinator decision 2026-11-12: a staff hold release removes the hold only
-- and never clears or zeroes the debt. The debt still owed at release
-- (held reversal: coins - debited) is recorded with the release.
-- Additive: one nullable column; NULL only on releases recorded before it.
ALTER TABLE coin_purchase_hold_releases
    ADD COLUMN debt_coins bigint CHECK (debt_coins IS NULL OR debt_coins >= 0);
