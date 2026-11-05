-- Local item presentation and issuance journal. No new Game receipt schema.
CREATE TABLE bazaar_item_details (
    id uuid PRIMARY KEY REFERENCES bazaar_listings(id),
    definition_key text NOT NULL CHECK (length(definition_key) BETWEEN 1 AND 128),
    definition_revision text NOT NULL CHECK (length(definition_revision) BETWEEN 1 AND 128),
    world_id uuid NOT NULL,
    seller_character_id uuid,
    item_instance_id uuid,
    quantity bigint NOT NULL CHECK (quantity BETWEEN 1 AND 4294967295),
    requested_price bigint CHECK (requested_price > 0),
    buy_now_price bigint CHECK (buy_now_price > 0),
    message text NOT NULL DEFAULT '' CHECK (length(message) <= 280),
    approved_name text,
    approved_attributes jsonb,
    escrow_id uuid,
    escrow_receipt_digest text,
    projection_epoch text,
    recovery_generation text,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
-- Deliberately separate from the dispatchable shared outbox. All rows stay blocked
-- until T58 has an accepted command decoder, receipt mapping and capability gate.
CREATE TABLE bazaar_item_outbox (
    id uuid PRIMARY KEY,
    listing_id uuid NOT NULL REFERENCES bazaar_listings(id),
    step text NOT NULL CHECK (step IN ('escrow','delivery','return')),
    issuance_key text NOT NULL UNIQUE,
    local_payload jsonb NOT NULL,
    dispatch_state text NOT NULL DEFAULT 'blocked' CHECK (dispatch_state = 'blocked'),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TRIGGER bazaar_item_outbox_immutable BEFORE UPDATE OR DELETE ON bazaar_item_outbox
FOR EACH ROW EXECUTE FUNCTION bazaar_append_only();
CREATE TABLE bazaar_item_requests (
    actor uuid NOT NULL,
    request_id uuid NOT NULL,
    binding jsonb NOT NULL,
    listing_id uuid NOT NULL REFERENCES bazaar_listings(id),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY(actor, request_id)
);
CREATE TRIGGER bazaar_item_request_immutable BEFORE UPDATE OR DELETE ON bazaar_item_requests
FOR EACH ROW EXECUTE FUNCTION bazaar_append_only();
CREATE TRIGGER bazaar_item_details_audit AFTER INSERT OR UPDATE ON bazaar_item_details
FOR EACH ROW EXECUTE FUNCTION bazaar_audit_change();
-- A match is immutable commercial evidence, not proof of Game delivery or payout.
CREATE TABLE bazaar_item_sales (
    id uuid PRIMARY KEY,
    listing_id uuid NOT NULL UNIQUE REFERENCES bazaar_listings(id),
    buyer uuid NOT NULL,
    seller uuid NOT NULL CHECK (buyer <> seller),
    gross_amount bigint NOT NULL CHECK (gross_amount > 0),
    fee_amount bigint NOT NULL CHECK (fee_amount = gross_amount / 10),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TRIGGER bazaar_item_sale_immutable BEFORE UPDATE OR DELETE ON bazaar_item_sales
FOR EACH ROW EXECUTE FUNCTION bazaar_append_only();
