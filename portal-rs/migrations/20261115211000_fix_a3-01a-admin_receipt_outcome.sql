-- Preserve the first acknowledged commit outcome across exact retries.
SELECT pg_advisory_xact_lock(71140002);
ALTER TABLE admin_analytics.receipts
    ADD COLUMN IF NOT EXISTS outcome text NOT NULL DEFAULT 'accepted'
    CHECK (outcome IN ('accepted', 'superseded'));
