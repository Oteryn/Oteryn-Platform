-- Older skull rows stored the observed CharacterRevision in ownership_revision.
-- Recover their period from committed cuts, including a return to a former owner.
-- Keep the cursor and payload intact; cleanup applies the repaired deadlines.
WITH receipts AS (
    SELECT command, receipt_json::jsonb AS r FROM character_commands
    WHERE state='TERMINAL' AND command IN ('CreateCharacter','TransferCharacterOwnership')
), cuts AS (
    SELECT character_id::text, account_id::text, character_revision AS revision
    FROM pd_ownership_baselines b
    WHERE NOT EXISTS (SELECT 1 FROM receipts c WHERE c.command='CreateCharacter'
                      AND c.r->>'character_id'=b.character_id::text)
    UNION ALL
    SELECT r->>'character_id',
           CASE command WHEN 'CreateCharacter' THEN r->>'account_id' ELSE r->>'to_account_id' END,
           (r->>'character_revision')::numeric
    FROM receipts WHERE r->>'outcome'='COMMITTED' AND r->>'result_code'='CHAR_CMD_OK'
        AND r->>'character_revision' ~ '^[1-9][0-9]{0,19}$'
), periods AS (
    SELECT *, lead(revision) OVER (PARTITION BY character_id ORDER BY revision) AS next_cut,
           count(*) OVER (PARTITION BY character_id,revision) AS cut_count
    FROM cuts
), proven AS (
    SELECT r.stream_key,r.source_revision,p.revision,s.baseline_complete,
           max(r.source_revision) FILTER (WHERE r.source_revision<=s.published_through)
               OVER (PARTITION BY r.stream_key,r.character_id,r.entity_key,p.revision,
                     split_part(r.scope_key,'|',2),split_part(r.scope_key,'|',5)) AS latest
    FROM pd_records r JOIN pd_streams s USING(stream_key)
    JOIN periods p ON p.character_id=r.character_id AND p.account_id=s.account_id
        AND numrange(p.revision,p.next_cut,'[)') @> CASE
            WHEN r.ownership_revision ~ '^[1-9][0-9]{0,19}$' THEN r.ownership_revision::numeric END
    WHERE s.family='skull' AND NOT s.erased AND r.entity_key<>'' AND p.cut_count=1
)
UPDATE pd_records r SET ownership_revision=p.revision::text,
    expires_ms=CASE WHEN p.baseline_complete AND r.source_revision<p.latest
        THEN LEAST(r.expires_ms, floor(extract(epoch FROM statement_timestamp())*1000)::bigint)
        ELSE r.expires_ms END
FROM proven p WHERE p.stream_key=r.stream_key AND p.source_revision=r.source_revision
    AND (r.ownership_revision IS DISTINCT FROM p.revision::text
         OR (p.baseline_complete AND r.source_revision<p.latest
             AND r.expires_ms>floor(extract(epoch FROM statement_timestamp())*1000)::bigint + 604800000));
