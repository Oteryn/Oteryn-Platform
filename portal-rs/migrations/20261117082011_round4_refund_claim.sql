-- Refund dispatch claims the row in a short transaction and calls the provider
-- outside it. `refunding` rows carry the attempt that may finalize them; the
-- provider idempotency key stays the intent id, never the attempt id.
ALTER TABLE commerce_refunds
    ADD COLUMN state text NOT NULL DEFAULT 'pending'
        CHECK (state IN ('pending', 'refunding', 'done')),
    ADD COLUMN attempt_id uuid,
    ADD COLUMN claimed_at timestamptz,
    ADD CONSTRAINT commerce_refunds_claim_check
        CHECK (state <> 'refunding' OR (attempt_id IS NOT NULL AND claimed_at IS NOT NULL)),
    ADD CONSTRAINT commerce_refunds_done_check
        CHECK (state <> 'done' OR provider_reference IS NOT NULL);
-- Rows already dispatched by the previous worker read as done.
UPDATE commerce_refunds SET state = 'done' WHERE provider_reference IS NOT NULL;
