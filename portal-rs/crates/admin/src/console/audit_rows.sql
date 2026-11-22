SELECT event.request_id, event.actor, event.action, event.operation, event.target,
       event.created_at::text AS recorded_at, projected.before, projected.after
FROM (
    SELECT request_id, actor, action, COALESCE(operation, action) AS operation,
           visible.target,
           id, created_at, before, after
    FROM audit_log
    CROSS JOIN LATERAL (VALUES (
        COALESCE(admin_control_target->>'character_id', admin_control_target->>'view', audit_log.target)
    )) AS stored(target)
    CROSS JOIN LATERAL (VALUES (
        CASE
            WHEN action = ANY($6::text[]) AND NOT (stored.target = ANY($7::text[])) THEN NULL
            WHEN action = 'admin.access.denied' AND starts_with(stored.target, $9::text) THEN $10::text
            ELSE stored.target
        END
    )) AS visible(target)
    WHERE ($1::uuid IS NULL OR request_id = $1)
      AND ($2::uuid IS NULL OR actor = $2)
      AND ($3::text IS NULL OR COALESCE(operation, action) = $3)
      AND ($4::text IS NULL OR visible.target = $4)
    ORDER BY created_at DESC, id DESC
    LIMIT 100
) AS event
CROSS JOIN LATERAL (
    SELECT
        CASE WHEN jsonb_typeof(event.before) = 'object' THEN
            coalesce(jsonb_object_agg(field, normalized.value) FILTER (WHERE NOT is_after), '{}'::jsonb)
        ELSE 'null'::jsonb END AS before,
        CASE WHEN jsonb_typeof(event.after) = 'object' THEN
            coalesce(jsonb_object_agg(field, normalized.value) FILTER (WHERE is_after), '{}'::jsonb)
        ELSE 'null'::jsonb END AS after
    FROM (VALUES (false, event.before), (true, event.after)) AS sides(is_after, snapshot)
    CROSS JOIN LATERAL jsonb_array_elements_text(
        coalesce($5::jsonb -> event.action, '[]'::jsonb)
        || CASE WHEN event.action = ANY($6::text[]) AND event.target = ANY($7::text[])
                THEN $8::jsonb ELSE '[]'::jsonb END
        || '["email"]'::jsonb
    ) AS allowed(field)
    CROSS JOIN LATERAL (SELECT snapshot -> field AS value) AS recorded
    CROSS JOIN LATERAL (
        SELECT CASE
            WHEN field IN ('published_at', 'translation_published_at')
            -- Restrict the cast to absolute ISO dates with an explicit offset or a fixed-offset zone.
            -- PostgreSQL's parser handles offset variants and rounds to microseconds.
            -- Relative legacy values such as "today" must remain unknown.
            AND jsonb_typeof(recorded.value) = 'string'
            AND recorded.value #>> '{}' ~*
                '^[[:space:]]*[0-9]{4}-[0-9]{2}-[0-9]{2}([Tt]|[[:space:]]+)[0-9]{2}:[0-9]{2}(:[0-9]{2}([.][0-9]+)?)?[[:space:]]*((Z|UTC|GMT|UT|ZULU|Etc/UTC|Etc/GMT)([+-][0-9:]+)?|[+-][0-9:]+)[[:space:]]*$'
            AND pg_input_is_valid(recorded.value #>> '{}', 'timestamp with time zone')
            THEN to_jsonb((to_jsonb((recorded.value #>> '{}')::timestamptz AT TIME ZONE 'UTC') #>> '{}') || 'Z')
            ELSE recorded.value
        END AS value
    ) AS normalized
    WHERE jsonb_typeof(snapshot) = 'object' AND snapshot ? field
) AS projected
ORDER BY event.created_at DESC, event.id DESC
