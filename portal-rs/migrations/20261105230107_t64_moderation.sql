-- Restrictive Platform policy; no Game command or receipt schema is introduced.
CREATE TABLE bazaar_moderation_holds (
    listing_id uuid PRIMARY KEY REFERENCES bazaar_listings(id),
    revision bigint NOT NULL CHECK (revision > 0),
    actor uuid NOT NULL,
    reason text NOT NULL CHECK (char_length(reason) BETWEEN 10 AND 500),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE bazaar_invalidation_requests (
    id uuid PRIMARY KEY,
    listing_id uuid NOT NULL REFERENCES bazaar_listings(id),
    hold_revision bigint NOT NULL,
    requested_by uuid NOT NULL,
    approved_by uuid CHECK (approved_by <> requested_by),
    reason text NOT NULL CHECK (char_length(reason) BETWEEN 10 AND 500),
    state text NOT NULL DEFAULT 'pending' CHECK (state IN ('pending', 'approved')),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    CHECK ((state = 'pending' AND approved_by IS NULL) OR (state = 'approved' AND approved_by IS NOT NULL))
);
CREATE UNIQUE INDEX bazaar_one_invalidation ON bazaar_invalidation_requests(listing_id);
-- This is a Platform saga request, never a wire intent. T58 must reconcile custody,
-- funding and current CHAR_CMD receipts before constructing any safe compensation.
CREATE TABLE bazaar_moderation_outbox (
    id uuid PRIMARY KEY REFERENCES bazaar_invalidation_requests(id),
    listing_id uuid NOT NULL REFERENCES bazaar_listings(id),
    kind text NOT NULL CHECK (kind = 'reconcile_invalidation'),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE bazaar_reports (
    id uuid PRIMARY KEY,
    listing_id uuid NOT NULL REFERENCES bazaar_listings(id),
    reporter uuid NOT NULL,
    category text NOT NULL CHECK (category IN ('fraud', 'inappropriate', 'other')),
    detail text NOT NULL CHECK (char_length(detail) BETWEEN 10 AND 1000),
    state text NOT NULL DEFAULT 'pending' CHECK (state IN ('pending', 'accepted', 'dismissed')),
    reviewed_by uuid,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    CHECK ((state = 'pending' AND reviewed_by IS NULL) OR (state <> 'pending' AND reviewed_by IS NOT NULL))
);
CREATE UNIQUE INDEX bazaar_report_open ON bazaar_reports(listing_id, reporter) WHERE state = 'pending';
CREATE INDEX bazaar_reports_reporter ON bazaar_reports(reporter, created_at);
CREATE TABLE bazaar_fee_revisions (
    id uuid PRIMARY KEY,
    actor uuid NOT NULL,
    coin_bps integer NOT NULL CHECK (coin_bps = 1000),
    gold_bps integer NOT NULL CHECK (gold_bps = 250),
    listing_fee bigint NOT NULL CHECK (listing_fee = 0),
    promotion_price bigint NOT NULL CHECK (promotion_price > 0),
    reason text NOT NULL CHECK (char_length(reason) BETWEEN 10 AND 500),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE bazaar_item_restrictions (
    id uuid PRIMARY KEY,
    world_id uuid NOT NULL,
    family text NOT NULL,
    production_key text NOT NULL,
    revision_ref text NOT NULL,
    minimum_value bigint NOT NULL CHECK (minimum_value > 0),
    denied boolean NOT NULL,
    actor uuid NOT NULL,
    reason text NOT NULL CHECK (char_length(reason) BETWEEN 10 AND 500),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX bazaar_item_restriction_latest ON bazaar_item_restrictions(world_id, family, production_key, revision_ref, created_at DESC, id DESC);
CREATE TABLE bazaar_promotions (
    id uuid PRIMARY KEY,
    listing_id uuid NOT NULL REFERENCES bazaar_listings(id),
    owner uuid NOT NULL,
    fee_revision uuid NOT NULL REFERENCES bazaar_fee_revisions(id),
    amount bigint NOT NULL CHECK (amount > 0),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE(listing_id)
);
-- Serialize all trading writes on the same listing lock as moderation. A hold
-- prevents bidding, matching, cancellation and settlement; it never releases value.
CREATE FUNCTION bazaar_moderation_fence() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE listing uuid;
BEGIN
    PERFORM pg_advisory_xact_lock_shared(64002);
    IF TG_TABLE_NAME = 'bazaar_listings' THEN listing := NEW.id;
        -- Until T58 supplies admitted catalog/instance bindings, a restriction
        -- fences all item activation. No unknown definition can bypass policy.
        IF NEW.kind = 'item' AND NEW.state = 'active' AND EXISTS (SELECT 1 FROM bazaar_item_restrictions) THEN
            RAISE EXCEPTION 'item policy requires verified binding' USING ERRCODE = '23514';
        END IF;
    ELSE listing := NEW.listing_id;
        PERFORM 1 FROM bazaar_listings WHERE id = listing FOR UPDATE;
    END IF;
    IF EXISTS (SELECT 1 FROM bazaar_moderation_holds WHERE listing_id = listing) THEN
        RAISE EXCEPTION 'listing suspended' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER bazaar_moderation_listing BEFORE UPDATE ON bazaar_listings FOR EACH ROW EXECUTE FUNCTION bazaar_moderation_fence();
CREATE TRIGGER bazaar_moderation_bid BEFORE INSERT ON bazaar_bids FOR EACH ROW EXECUTE FUNCTION bazaar_moderation_fence();
CREATE TRIGGER bazaar_moderation_offer BEFORE INSERT OR UPDATE ON bazaar_offers FOR EACH ROW EXECUTE FUNCTION bazaar_moderation_fence();
CREATE TRIGGER bazaar_moderation_settlement BEFORE INSERT OR UPDATE ON bazaar_settlements FOR EACH ROW EXECUTE FUNCTION bazaar_moderation_fence();
CREATE TRIGGER bazaar_fee_immutable BEFORE UPDATE OR DELETE ON bazaar_fee_revisions FOR EACH ROW EXECUTE FUNCTION bazaar_append_only();
CREATE TRIGGER bazaar_item_policy_immutable BEFORE UPDATE OR DELETE ON bazaar_item_restrictions FOR EACH ROW EXECUTE FUNCTION bazaar_append_only();
CREATE TRIGGER bazaar_promotion_immutable BEFORE UPDATE OR DELETE ON bazaar_promotions FOR EACH ROW EXECUTE FUNCTION bazaar_append_only();
CREATE TRIGGER bazaar_moderation_outbox_immutable BEFORE UPDATE OR DELETE ON bazaar_moderation_outbox FOR EACH ROW EXECUTE FUNCTION bazaar_append_only();

CREATE FUNCTION bazaar_hold_audit_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO bazaar_audit(relation_name,row_id,before_row,after_row)
    VALUES (TG_TABLE_NAME,NEW.listing_id,CASE WHEN TG_OP='UPDATE' THEN to_jsonb(OLD) ELSE NULL END,to_jsonb(NEW));
    RETURN NEW;
END;
$$;
CREATE TRIGGER bazaar_hold_audit AFTER INSERT OR UPDATE ON bazaar_moderation_holds FOR EACH ROW EXECUTE FUNCTION bazaar_hold_audit_change();
CREATE TRIGGER bazaar_report_audit AFTER INSERT OR UPDATE ON bazaar_reports FOR EACH ROW EXECUTE FUNCTION bazaar_audit_change();
CREATE TRIGGER bazaar_invalidation_audit AFTER INSERT OR UPDATE ON bazaar_invalidation_requests FOR EACH ROW EXECUTE FUNCTION bazaar_audit_change();
CREATE TRIGGER bazaar_fee_audit AFTER INSERT ON bazaar_fee_revisions FOR EACH ROW EXECUTE FUNCTION bazaar_audit_change();
CREATE TRIGGER bazaar_policy_audit AFTER INSERT ON bazaar_item_restrictions FOR EACH ROW EXECUTE FUNCTION bazaar_audit_change();
CREATE TRIGGER bazaar_promotion_audit AFTER INSERT ON bazaar_promotions FOR EACH ROW EXECUTE FUNCTION bazaar_audit_change();
CREATE TRIGGER bazaar_fee_no_truncate BEFORE TRUNCATE ON bazaar_fee_revisions FOR EACH STATEMENT EXECUTE FUNCTION bazaar_append_only();
CREATE TRIGGER bazaar_policy_no_truncate BEFORE TRUNCATE ON bazaar_item_restrictions FOR EACH STATEMENT EXECUTE FUNCTION bazaar_append_only();
CREATE TRIGGER bazaar_outbox_no_truncate BEFORE TRUNCATE ON bazaar_moderation_outbox FOR EACH STATEMENT EXECUTE FUNCTION bazaar_append_only();
CREATE TRIGGER bazaar_promotion_no_truncate BEFORE TRUNCATE ON bazaar_promotions FOR EACH STATEMENT EXECUTE FUNCTION bazaar_append_only();
