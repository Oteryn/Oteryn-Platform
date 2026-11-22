ALTER TABLE tickets ADD COLUMN assigned_to uuid REFERENCES accounts(id) ON DELETE SET NULL;
ALTER TABLE player_reports ADD COLUMN sanction_id uuid REFERENCES sanctions(id) ON DELETE SET NULL;

-- Import the latest assignment visible at the ticket's current revision.
WITH latest_assignment AS (
    SELECT DISTINCT ON (t.id) t.id, a.after->>'assigned_to' AS assignee
    FROM tickets t
    JOIN audit_log a ON a.target = t.id::text
    WHERE a.action = 'support.ticket_assigned'
      AND jsonb_typeof(a.after->'lock_version') = 'number'
      AND a.after->'lock_version' <= to_jsonb(t.lock_version)
    ORDER BY t.id, a.after->'lock_version' DESC, a.id DESC
)
UPDATE tickets t SET assigned_to = account.id
FROM latest_assignment latest
JOIN accounts account ON account.id = CASE
    WHEN pg_input_is_valid(latest.assignee, 'uuid') THEN latest.assignee::uuid
END
WHERE t.id = latest.id;

-- Older events could link a display name without verifying its account.
-- Import only links whose reported account matches the recorded sanction.
WITH latest_link AS (
    SELECT DISTINCT ON (r.id) r.id, r.target_reference, a.after->>'sanction_id' AS sanction
    FROM player_reports r
    JOIN audit_log a ON a.target = r.id::text
    WHERE a.action = 'support.report_sanction_linked'
      AND r.report_type = 'player'
      AND r.status IN ('actioned', 'closed')
      AND jsonb_typeof(a.after->'lock_version') = 'number'
      AND a.after->'lock_version' <= to_jsonb(r.lock_version)
    ORDER BY r.id, a.after->'lock_version' DESC, a.id DESC
)
UPDATE player_reports r SET sanction_id = s.id
FROM latest_link latest
JOIN sanctions s ON s.id = CASE
    WHEN pg_input_is_valid(latest.sanction, 'uuid') THEN latest.sanction::uuid
END
WHERE r.id = latest.id
  AND s.status <> 'pending'
  AND s.account_id = CASE
      WHEN pg_input_is_valid(latest.target_reference, 'uuid') THEN latest.target_reference::uuid
  END;

CREATE INDEX tickets_assigned_to_idx ON tickets (assigned_to) WHERE assigned_to IS NOT NULL;
CREATE INDEX player_reports_sanction_id_idx ON player_reports (sanction_id) WHERE sanction_id IS NOT NULL;
