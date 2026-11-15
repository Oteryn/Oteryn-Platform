-- Acceptance used to shorten current-state retention before publication.
-- Repair retained bytes whose replacement has not entered a proven prefix.
-- Scrubbed lifecycle rows and already-published superseded values stay erased/
-- bounded; this cannot recover bytes removed before the fix.
-- Fresh databases have nothing to repair; the guard keeps the file valid
-- when applied alone.
DO $repair$
BEGIN
IF to_regclass('pd_streams') IS NOT NULL THEN
UPDATE pd_records r
SET expires_ms = r.occurred_ms + CASE WHEN s.family = 'game_login'
    THEN 30::bigint * 86400000 ELSE 36500::bigint * 86400000 END
FROM pd_streams s
WHERE s.stream_key = r.stream_key
  AND s.progress
  AND NOT s.erased
  AND s.family NOT IN ('skill_history', 'death_recap', 'pvp')
  AND r.payload IS NOT NULL
  AND r.scope_key <> ''
  AND NOT EXISTS (
      SELECT 1 FROM pd_records n
      WHERE n.stream_key = r.stream_key
        AND n.entity_key = r.entity_key
        AND n.scope_key = r.scope_key
        AND n.source_revision > r.source_revision
        AND s.baseline_complete
        AND n.source_revision <= s.published_through
  );

END IF;
END
$repair$;
