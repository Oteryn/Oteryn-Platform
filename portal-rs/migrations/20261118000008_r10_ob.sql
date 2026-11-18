-- Supply the claim order across pending jobs and reclaimable leases.
CREATE INDEX outbox_claim_order ON outbox (run_after, id)
    WHERE state IN ('pending', 'running');

-- Legacy slots encode the number in the key. Numeric also handles old keys
-- whose digits exceed bigint range without changing their eligibility.
CREATE INDEX outbox_completed_effective_slot ON outbox
    (kind, coalesce(schedule_slot::numeric, substring(idempotency_key FROM '^schedule:([0-9]+)$')::numeric))
    WHERE state = 'done' AND (schedule_slot IS NOT NULL OR payload = '{}'::jsonb);
