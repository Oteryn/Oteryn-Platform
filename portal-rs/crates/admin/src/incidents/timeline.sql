WITH scoped AS (
    SELECT r.request_id, r.actor, r.operation, r.dry_run, r.created_at
    FROM admin_control_requests r
    JOIN admin_incident_cases c ON c.id = $1
    WHERE r.kind = 'command' AND r.reason->>'ticket_ref' = c.id::text
      AND r.target->>'world_id' = c.world_id
      AND r.target->>'node_id' IS NOT DISTINCT FROM c.node_id
      AND r.target->>'character_id' IS NOT DISTINCT FROM c.character_id
      AND r.target->>'account_ref' IS NOT DISTINCT FROM c.account_ref::text
), events AS (
    SELECT l.id AS event_id, l.created_at AS at, l.action AS kind, l.actor,
           NULL::uuid AS request_id, NULL::text AS operation, NULL::text AS status,
           NULL::uuid AS approval_id
    FROM audit_log l
    JOIN admin_incident_cases c ON c.id = $1
    WHERE (l.id = c.alarm_id AND l.actor IS NULL
           AND l.action = 'ops.playerdata_binding_conflict')
       OR (l.target = c.id::text AND l.action IN ('incident.created', 'incident.assigned'))
    UNION ALL
    SELECT r.request_id, r.created_at, CASE WHEN r.dry_run THEN 'dry_run' ELSE 'request' END,
           r.actor, r.request_id, r.operation, NULL, NULL
    FROM scoped r
    UNION ALL
    SELECT l.id, l.created_at, 'transition', l.actor, r.request_id,
           r.operation, l.after->>'status', NULL
    FROM scoped r JOIN audit_log l USING (request_id)
    WHERE l.action = 'admin.control.transition'
      AND l.after->>'status' IN ('delivered', 'accepted', 'in_progress', 'succeeded',
                                'failed', 'rejected', 'expired', 'cancelled')
    UNION ALL
    SELECT p.id, p.created_at, 'preview', p.requester, r.request_id, r.operation, NULL, NULL
    FROM scoped r JOIN admin_control_previews p ON p.id = r.request_id
    WHERE r.dry_run AND p.requester = r.actor AND p.operation = r.operation
    UNION ALL
    SELECT a.id, a.created_at, 'approved', a.admin_id, r.request_id, r.operation, NULL, a.id
    FROM scoped r
    JOIN admin_control_previews p ON p.id = r.request_id
    JOIN admin_control_approvals a ON a.preview_id = p.id
    WHERE r.dry_run AND p.requester = r.actor AND p.operation = r.operation
      AND a.admin_id <> p.requester
    UNION ALL
    SELECT a.approval_id, a.created_at, 'approval_used', a.approver, r.request_id,
           r.operation, NULL, a.approval_id
    FROM scoped r
    JOIN admin_approvals a USING (request_id)
    JOIN admin_control_approvals stored ON stored.id = a.approval_id AND stored.admin_id = a.approver
    JOIN scoped preview ON preview.request_id = stored.preview_id
    WHERE NOT r.dry_run AND preview.dry_run AND preview.actor = r.actor
      AND preview.operation = r.operation AND a.approver <> r.actor AND a.decision = 'approved'
)
SELECT event_id, at, kind, actor, request_id, operation, status, approval_id
FROM events
WHERE at >= $2 AND at < $3
  AND ($5::timestamptz IS NULL OR (at, event_id, kind) < ($5, $6::uuid, $7::text))
ORDER BY at DESC, event_id DESC, kind DESC
LIMIT $4
