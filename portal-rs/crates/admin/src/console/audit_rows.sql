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
    WHERE ($11::boolean OR action IS DISTINCT FROM 'ops.playerdata_binding_conflict')
      AND ($1::uuid IS NULL OR request_id = $1)
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
            -- Limit normalization to absolute dates whose order does not depend on DateStyle.
            -- Let PostgreSQL validate and parse the result, including fractional seconds.
            -- Test its zone suffix against an already zoned literal: a real zone conflicts
            -- with +00, while non-zone qualifiers like AD or PM do not.
            AND jsonb_typeof(recorded.value) = 'string'
            AND pg_input_is_valid(recorded.value #>> '{}', 'timestamp with time zone')
            AND NOT pg_input_is_valid(
                '2000-01-01 00:00:00+00 ' || substring(
                    recorded.value #>> '{}' FROM
                    $audit_timestamp$(?ix)
                    ^ [[:space:]]*
                    (?: (?:mon|tue|wed|thu|fri|sat|sun)[a-z]* [[:space:],]+ )?
                    (?:
                        [0-9]{4} (?:[[:space:]./-]+[0-9]{1,2}){2}
                      | [0-9]{4}[.-][0-9]{3} | [0-9]{8} | j[0-9]+
                      | # Find a named month and four-digit year in the date fields.
                        # PostgreSQL's traditional format can put the clock before the year.
                        (?= (?:[0-9]{1,4}[[:space:],./-]+){0,2}
                            (?:jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*\y )
                        (?= (?:(?:[a-z]+|[0-9]{1,2})[[:space:],./-]+){0,2}
                            (?:[0-9]{1,2}:[0-9:.]+[[:space:]]+)? [0-9]{4}(?![0-9]) )
                        (?:[a-z]+|[0-9]{1,4})
                        (?:[[:space:],./-]+(?:[0-9]{1,2}:[0-9:.]+[[:space:]]+)?(?:[a-z]+|[0-9]{1,4})){2}
                    )
                    (?: [T[:space:]]+ (?:at[[:space:]]+)? [0-9]+ (?:[:.][0-9]+)*
                        (?:[[:space:]]+(?:am|pm)[[:space:]]+)? )?
                    [[:space:]]* ([+-][0-9:]+|[a-z][a-z0-9_/+.:-]*(?:[[:space:]]+dst)?)
                    (?:[[:space:]]+(?:at|on|ad))*
                    [[:space:]]* $
                    $audit_timestamp$
                ),
                'timestamp with time zone'
            )
            THEN to_jsonb((to_jsonb((recorded.value #>> '{}')::timestamptz AT TIME ZONE 'UTC') #>> '{}') || 'Z')
            ELSE recorded.value
        END AS value
    ) AS normalized
    WHERE jsonb_typeof(snapshot) = 'object' AND snapshot ? field
) AS projected
ORDER BY event.created_at DESC, event.id DESC
