-- Staff price proposals are separate from the immutable Game import namespace.
CREATE TABLE store_price_proposals (
    base_revision text NOT NULL,
    offer_key text NOT NULL,
    unit_price bigint NOT NULL CHECK (unit_price > 0),
    proposed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (base_revision, offer_key, unit_price),
    FOREIGN KEY (base_revision, offer_key) REFERENCES store_offers(catalog_revision, offer_key)
);

CREATE INDEX coin_purchase_orders_review_queue ON coin_purchase_orders (payment_intent_id)
    WHERE review_required AND payment_intent_id IS NOT NULL;
CREATE INDEX outbox_refund_review_queue ON outbox (idempotency_key)
    WHERE kind='commerce.refund' AND state='dead';
CREATE INDEX commerce_operations_refund_intent ON commerce_operations (target_id, updated_at DESC, id DESC)
    WHERE command->>'kind'='refund';
CREATE INDEX commerce_operations_pending_refund_intent ON commerce_operations (target_id)
    WHERE command->>'kind'='refund' AND result IS NULL;
CREATE INDEX commerce_operations_review_intent ON commerce_operations (target_id, updated_at DESC, id DESC)
    WHERE command->>'kind'='reconcile' AND result->>'resolution_code'='reviewed_no_payment_state_change';
