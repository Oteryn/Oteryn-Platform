-- One coin class (owner decision 2026-11-12). Additive: no history is changed or
-- removed. Balances, receipts and history sum entries of either coin_class, and
-- new entries no longer name one, so they take the column default.
ALTER TABLE wallet_entries ALTER COLUMN coin_class SET DEFAULT 'transferable';
COMMENT ON COLUMN wallet_entries.coin_class IS
    'Retired 2026-11-12: one coin class. Historical label only; every row counts toward the one balance.';
-- Rollback is application rollback with flags off; do not rewrite ledger rows.
