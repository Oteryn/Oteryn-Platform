SELECT event.request_id, event.actor, event.action, event.operation, event.target,
       event.created_at::text AS recorded_at, projected.before, projected.after
FROM (
    SELECT request_id, actor, action, COALESCE(operation, action) AS operation,
           COALESCE(admin_control_target->>'character_id', admin_control_target->>'view', target) AS target,
           id, created_at, before, after
    FROM audit_log
    WHERE ($1::uuid IS NULL OR request_id = $1)
      AND ($2::uuid IS NULL OR actor = $2)
      AND ($3::text IS NULL OR COALESCE(operation, action) = $3)
      AND ($4::text IS NULL OR COALESCE(admin_control_target->>'character_id', admin_control_target->>'view', target) = $4)
    ORDER BY created_at DESC, id DESC
    LIMIT 100
) AS event
CROSS JOIN LATERAL (
    SELECT
        CASE WHEN jsonb_typeof(event.before) = 'object' THEN
            coalesce(jsonb_object_agg(field, snapshot -> field) FILTER (WHERE NOT is_after), '{}'::jsonb)
        ELSE 'null'::jsonb END AS before,
        CASE WHEN jsonb_typeof(event.after) = 'object' THEN
            coalesce(jsonb_object_agg(field, snapshot -> field) FILTER (WHERE is_after), '{}'::jsonb)
        ELSE 'null'::jsonb END AS after
    FROM (VALUES (false, event.before), (true, event.after)) AS sides(is_after, snapshot)
    CROSS JOIN LATERAL jsonb_array_elements_text(
        coalesce($5::jsonb -> event.action, '[]'::jsonb)
        || CASE WHEN event.action = ANY($6::text[]) AND event.target = ANY($7::text[])
                THEN $8::jsonb ELSE '[]'::jsonb END
        || '["email"]'::jsonb
    ) AS allowed(field)
    WHERE jsonb_typeof(snapshot) = 'object' AND snapshot ? field
) AS projected
ORDER BY event.created_at DESC, event.id DESC
