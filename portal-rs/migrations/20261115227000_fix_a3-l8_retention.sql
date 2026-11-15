-- Replay floors survive bounded digest retention.
ALTER TABLE pd_g9_streams ADD COLUMN retention_floor numeric(20,0) NOT NULL DEFAULT 0;
ALTER TABLE pd_t78_records ADD COLUMN erased_at_ms bigint;
-- Existing erased character rows are recognizable by their cleared subject fields.
UPDATE pd_t78_records SET erased_at_ms=floor(extract(epoch FROM clock_timestamp())*1000)::bigint
WHERE payload IS NULL AND entity_key='' AND occurred_at_ms=0;
ALTER TABLE pd_records ADD COLUMN ownership_revision text;
-- Recover historical ownership only from the same independently committed cuts
-- used by admission. The interval ends at the next committed ownership cut.
WITH cuts AS (
    SELECT character_id::text AS character_id,account_id::text AS account_id,
           character_revision::text AS revision,projection_epoch::text AS epoch,
           recovery_generation::text AS recovery,extract(epoch FROM accepted_at)*1000 AS at_ms
    FROM pd_ownership_baselines
    WHERE NOT EXISTS(SELECT 1 FROM character_commands c WHERE c.command='CreateCharacter'
          AND c.state='TERMINAL' AND c.receipt_json::jsonb->>'character_id'=pd_ownership_baselines.character_id::text)
    UNION ALL
    SELECT receipt_json::jsonb->>'character_id',
           CASE WHEN command='CreateCharacter' THEN receipt_json::jsonb->>'account_id'
                ELSE receipt_json::jsonb->>'to_account_id' END,
           receipt_json::jsonb->>'character_revision',receipt_json::jsonb->>'projection_epoch',
           receipt_json::jsonb->>'recovery_generation',
           (receipt_json::jsonb->>'decided_at')::numeric*1000
    FROM character_commands
    WHERE command IN ('CreateCharacter','TransferCharacterOwnership') AND state='TERMINAL'
      AND receipt_json::jsonb->>'outcome'='COMMITTED' AND receipt_json::jsonb->>'result_code'='CHAR_CMD_OK'
), intervals AS (
    SELECT *,lead(at_ms) OVER(PARTITION BY character_id ORDER BY revision::numeric) AS until_ms FROM cuts
)
UPDATE pd_records r SET ownership_revision=c.revision
FROM pd_streams s,intervals c
WHERE r.stream_key=s.stream_key AND r.character_id=c.character_id AND s.account_id=c.account_id
  AND r.occurred_ms>=c.at_ms AND (c.until_ms IS NULL OR r.occurred_ms<c.until_ms)
  AND split_part(r.scope_key,'|',4)=length(c.epoch)::text || ':' || c.epoch
  AND split_part(r.scope_key,'|',5)=length(c.recovery)::text || ':' || c.recovery;
CREATE INDEX pd_g9_purged_expiry ON pd_g9_records(purged_at) WHERE purged_at IS NOT NULL;
CREATE INDEX pd_t78_erased_expiry ON pd_t78_records(erased_at_ms) WHERE erased_at_ms IS NOT NULL;
-- Repair already-superseded payloads; the latest value and unproven ownership
-- intervals are left intact. Minimal digest/sequence fences are unaffected.
UPDATE pd_records r SET expires_ms=LEAST(r.expires_ms,floor(extract(epoch FROM clock_timestamp())*1000)::bigint)
FROM pd_streams stream
WHERE stream.stream_key=r.stream_key AND stream.family IN ('skull','premium_end')
  AND (stream.family='premium_end' OR r.ownership_revision IS NOT NULL)
  AND EXISTS(SELECT 1 FROM pd_records newer WHERE newer.stream_key=r.stream_key
      AND newer.entity_key=r.entity_key AND newer.source_revision>r.source_revision
      AND newer.ownership_revision IS NOT DISTINCT FROM r.ownership_revision
      AND split_part(newer.scope_key,'|',1)=split_part(r.scope_key,'|',1)
      AND split_part(newer.scope_key,'|',2)=split_part(r.scope_key,'|',2)
      AND split_part(newer.scope_key,'|',4)=split_part(r.scope_key,'|',4)
      AND split_part(newer.scope_key,'|',5)=split_part(r.scope_key,'|',5));
