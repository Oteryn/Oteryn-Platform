-- UTC arithmetic makes the resend window independent of session time zone.
CREATE INDEX account_email_tokens_cleanup_due ON account_email_tokens
    ((greatest((created_at AT TIME ZONE 'UTC') + interval '60 seconds',
                  CASE WHEN used_at IS NULL THEN expires_at AT TIME ZONE 'UTC' END)), token_hash)
    WHERE purpose = 'verify_email';

-- Only the scheduler sets this marker. Existing domain idempotency keys are opaque.
ALTER TABLE outbox ADD COLUMN schedule_slot bigint;
CREATE INDEX outbox_completed_schedule_slots ON outbox (kind, schedule_slot)
    WHERE state = 'done' AND schedule_slot IS NOT NULL;
