ALTER TABLE support_enforcement_decisions
    ADD COLUMN expiry_blocked_by uuid REFERENCES support_enforcement_commands(operation_id),
    ADD CHECK (expiry_blocked_by IS NULL OR (expires_at_ms IS NOT NULL AND reconcile_at_ms IS NULL));

-- Recover elapsed timers which an earlier worker already consumed after a
-- newer refusal. The successful receipt, not the Platform status, proves the
-- prior runtime decision.
UPDATE support_enforcement_decisions d
SET reconcile_at_ms=NULL, expiry_blocked_by=head.operation_id
FROM support_enforcement_commands confirmed
JOIN support_enforcement_results applied ON applied.operation_id=confirmed.operation_id AND applied.terminal
JOIN LATERAL (
    SELECT operation_id,intent,decision_revision,command FROM support_enforcement_commands
    WHERE sanction_stream_id=confirmed.sanction_stream_id ORDER BY decision_revision DESC,position DESC LIMIT 1
) head ON true
JOIN support_enforcement_results refused ON refused.operation_id=head.operation_id AND refused.terminal
WHERE d.operation_id=confirmed.operation_id
  AND d.expires_at_ms<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint
  AND applied.state IN ('APPLIED','REPLACED') AND refused.state='REJECTED'
  AND ((head.intent='EXPIRE' AND head.decision_revision=d.decision_revision)
    OR (convert_from(head.command,'UTF8')::jsonb->>'prior_decision_revision')::bigint=d.decision_revision);
