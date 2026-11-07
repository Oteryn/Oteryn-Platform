-- Disable both switches first. Retain fences and all committed evidence.
-- Refuse rollback while low_n payloads exist rather than silently deleting or
-- relabelling them. Drain payloads under the retention/operator policy first.
SELECT pg_advisory_xact_lock(71140001);
ALTER TABLE admin_analytics.rows DROP CONSTRAINT rows_state_check;
ALTER TABLE admin_analytics.rows ADD CONSTRAINT rows_state_check
 CHECK (state IN ('visible','suppressed','incomplete'));
