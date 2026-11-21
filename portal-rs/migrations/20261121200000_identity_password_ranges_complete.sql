-- Owner decision 2026-11-21: the breach corpus is the HIBP range API with a 30-day cache.
-- Existing operator-imported rows kept only a subset of each range, so they become
-- incomplete and no longer decide a lookup; only rows written from a full fetch are complete.
ALTER TABLE registration_password_ranges ADD COLUMN complete boolean NOT NULL DEFAULT false;
