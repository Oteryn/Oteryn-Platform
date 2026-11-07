-- Add economy states without changing the rev-2 subjects, fences or receipts.
SELECT pg_advisory_xact_lock(71140001);
ALTER TABLE admin_analytics.rows DROP CONSTRAINT rows_state_check;
ALTER TABLE admin_analytics.rows ADD CONSTRAINT rows_state_check CHECK (
 (family IN ('economy_circulation_daily','economy_flows_daily','economy_market_daily','economy_trades_daily','economy_store_daily')
  AND state IN ('visible','low_n','incomplete')) OR
 (family NOT IN ('economy_circulation_daily','economy_flows_daily','economy_market_daily','economy_trades_daily','economy_store_daily')
  AND state IN ('visible','suppressed','incomplete'))
);
