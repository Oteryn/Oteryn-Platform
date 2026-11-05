-- Account references have no foreign key until the account domain exists.
CREATE TYPE bazaar_kind AS ENUM ('character', 'item');
CREATE TYPE bazaar_form AS ENUM ('auction', 'fixed_price', 'want_to_buy');
CREATE TYPE bazaar_currency AS ENUM ('coins', 'gold');
CREATE TYPE bazaar_state AS ENUM ('draft', 'active', 'matched', 'awaiting_delivery', 'completed', 'cancelled', 'expired');
CREATE TABLE bazaar_listings (
    id uuid PRIMARY KEY,
    owner uuid NOT NULL CHECK (owner <> '00000000-0000-0000-0000-000000000000'),
    kind bazaar_kind NOT NULL,
    form bazaar_form NOT NULL,
    currency bazaar_currency NOT NULL,
    price bigint NOT NULL CHECK (price > 0),
    accept_offers boolean NOT NULL,
    state bazaar_state NOT NULL DEFAULT 'draft',
    ends_at timestamptz NOT NULL,
    leader uuid,
    leader_max bigint,
    reservation uuid,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    CHECK (kind <> 'character' OR form = 'auction'),
    CHECK (form <> 'auction' OR NOT accept_offers),
    CHECK ((leader IS NULL AND leader_max IS NULL AND reservation IS NULL)
        OR (leader IS NOT NULL AND leader <> owner AND leader_max IS NOT NULL AND leader_max >= price AND reservation IS NOT NULL))
);
CREATE INDEX bazaar_listings_due ON bazaar_listings (ends_at) WHERE state = 'active';
CREATE TABLE bazaar_bids (
    id uuid PRIMARY KEY,
    listing_id uuid NOT NULL REFERENCES bazaar_listings(id),
    account uuid NOT NULL,
    maximum bigint NOT NULL CHECK (maximum > 0),
    idempotency_key text NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
    leader uuid NOT NULL,
    price bigint NOT NULL CHECK (price > 0),
    ends_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE (listing_id, idempotency_key)
);
CREATE TABLE bazaar_offers (
    id uuid PRIMARY KEY,
    listing_id uuid NOT NULL REFERENCES bazaar_listings(id),
    account uuid NOT NULL,
    amount bigint NOT NULL CHECK (amount > 0),
    idempotency_key text NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
    state text NOT NULL DEFAULT 'pending' CHECK (state IN ('pending', 'accepted', 'rejected', 'expired')),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE (listing_id, idempotency_key)
);
CREATE UNIQUE INDEX bazaar_offer_pending_account ON bazaar_offers(listing_id, account) WHERE state = 'pending';
CREATE UNIQUE INDEX bazaar_offer_one_accepted ON bazaar_offers(listing_id) WHERE state = 'accepted';
CREATE INDEX bazaar_offer_account ON bazaar_offers(account) WHERE state = 'pending';
CREATE TABLE bazaar_settlements (
    id uuid PRIMARY KEY,
    listing_id uuid NOT NULL UNIQUE REFERENCES bazaar_listings(id),
    buyer uuid NOT NULL,
    seller uuid NOT NULL CHECK (seller <> buyer),
    currency bazaar_currency NOT NULL,
    amount bigint NOT NULL CHECK (amount > 0),
    commission bigint NOT NULL CHECK (commission >= 0),
    seller_net bigint NOT NULL CHECK (seller_net >= 0 AND seller_net = amount - commission),
    reservation uuid NOT NULL,
    idempotency_key text NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
    completed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    CHECK (commission = CASE currency WHEN 'coins' THEN amount / 10 ELSE amount / 40 END)
);
CREATE TABLE bazaar_audit (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    relation_name text NOT NULL,
    row_id uuid NOT NULL,
    before_row jsonb,
    after_row jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE FUNCTION bazaar_audit_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO bazaar_audit(relation_name, row_id, before_row, after_row)
    VALUES (TG_TABLE_NAME, NEW.id, CASE WHEN TG_OP = 'UPDATE' THEN to_jsonb(OLD) ELSE NULL END, to_jsonb(NEW));
    RETURN NEW;
END;
$$;
CREATE TRIGGER bazaar_listing_audit AFTER INSERT OR UPDATE ON bazaar_listings FOR EACH ROW EXECUTE FUNCTION bazaar_audit_change();
CREATE TRIGGER bazaar_bid_audit AFTER INSERT ON bazaar_bids FOR EACH ROW EXECUTE FUNCTION bazaar_audit_change();
CREATE TRIGGER bazaar_offer_audit AFTER INSERT OR UPDATE ON bazaar_offers FOR EACH ROW EXECUTE FUNCTION bazaar_audit_change();
CREATE TRIGGER bazaar_settlement_audit AFTER INSERT OR UPDATE ON bazaar_settlements FOR EACH ROW EXECUTE FUNCTION bazaar_audit_change();
CREATE FUNCTION bazaar_append_only() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'bazaar history is append-only';
END;
$$;
CREATE TRIGGER bazaar_audit_immutable BEFORE UPDATE OR DELETE ON bazaar_audit FOR EACH ROW EXECUTE FUNCTION bazaar_append_only();
CREATE TRIGGER bazaar_bid_immutable BEFORE UPDATE OR DELETE ON bazaar_bids FOR EACH ROW EXECUTE FUNCTION bazaar_append_only();
CREATE FUNCTION bazaar_state_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.state <> OLD.state AND NOT (
        (OLD.state = 'draft' AND NEW.state IN ('active', 'cancelled')) OR
        (OLD.state = 'active' AND NEW.state IN ('matched', 'awaiting_delivery', 'cancelled', 'expired')) OR
        (OLD.state = 'matched' AND NEW.state = 'awaiting_delivery') OR
        (OLD.state = 'awaiting_delivery' AND NEW.state = 'completed')
    ) THEN
        RAISE EXCEPTION 'illegal bazaar transition';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER bazaar_state_transition BEFORE UPDATE ON bazaar_listings FOR EACH ROW EXECUTE FUNCTION bazaar_state_guard();
