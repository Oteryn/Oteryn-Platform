-- An unconfirmed or refused successor does not cancel an earlier confirmed
-- timer. Repair timers from versions that cleared them at command publication.
UPDATE support_enforcement_decisions d
SET reconcile_at_ms=d.expires_at_ms
FROM support_enforcement_commands c
JOIN support_enforcement_results r ON r.operation_id=c.operation_id AND r.terminal
WHERE c.operation_id=d.operation_id
  AND d.reconcile_at_ms IS NULL
  AND r.state IN ('APPLIED','REPLACED')
  AND convert_from(r.result,'UTF8')::jsonb->>'stream_state'='ACTIVE'
  AND (convert_from(r.result,'UTF8')::jsonb->>'accepted_decision_revision')::bigint=d.decision_revision
  AND NOT EXISTS (
      SELECT 1 FROM support_enforcement_commands newer
      LEFT JOIN support_enforcement_results result ON result.operation_id=newer.operation_id AND result.terminal
      WHERE newer.sanction_stream_id=d.sanction_id AND newer.position>c.position
        AND (result.state<>'REJECTED' OR (newer.intent='EXPIRE' AND newer.decision_revision=c.decision_revision))
  );
