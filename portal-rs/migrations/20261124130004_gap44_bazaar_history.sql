CREATE TABLE bazaar_history_shares (
    listing uuid PRIMARY KEY REFERENCES bazaar_listings(id),
    public_id uuid NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    seller_at timestamptz,
    buyer_at timestamptz,
    CHECK (seller_at IS NOT NULL OR buyer_at IS NOT NULL)
);

CREATE TABLE bazaar_history_searches (
    account uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name text NOT NULL CHECK (length(name) BETWEEN 1 AND 80),
    criteria jsonb NOT NULL,
    saved_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (account, name)
);

CREATE TABLE bazaar_history_preferences (
    account uuid PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    alerts_since timestamptz NOT NULL DEFAULT clock_timestamp()
);

CREATE TABLE bazaar_history_watches (
    account uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    sale uuid NOT NULL REFERENCES bazaar_history_shares(public_id) ON DELETE CASCADE,
    PRIMARY KEY (account, sale)
);

CREATE TABLE bazaar_history_dismissals (
    account uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    sale uuid NOT NULL REFERENCES bazaar_history_shares(public_id) ON DELETE CASCADE,
    PRIMARY KEY (account, sale)
);

CREATE VIEW bazaar_history_settled AS
SELECT l.id, l.kind::text AS kind, l.form::text AS form, l.currency::text AS currency,
       s.buyer, s.seller, s.amount, s.completed_at
FROM bazaar_listings l
JOIN (
    SELECT c.listing_id, c.buyer, c.seller, c.currency, c.amount, c.completed_at
    FROM bazaar_settlements c
    WHERE c.completed_at IS NOT NULL AND (
        c.currency = 'coins' OR NOT EXISTS (
            SELECT 1 FROM bazaar_authority_gold_sales g WHERE g.listing_id = c.listing_id
        )
    )
    UNION ALL
    SELECT listing_id, buyer, seller, 'gold'::bazaar_currency, gross_amount, completed_at
    FROM bazaar_authority_gold_sales WHERE completed_at IS NOT NULL
) s ON s.listing_id = l.id AND s.currency = l.currency
WHERE l.state = 'completed' AND s.completed_at <= now()
  AND ((l.form = 'want_to_buy' AND l.owner = s.buyer)
       OR (l.form <> 'want_to_buy' AND l.owner = s.seller));

-- Sharing is a separate opt-in by both participants. Only settlement facts appear here.
CREATE VIEW bazaar_history_market AS
SELECT h.public_id AS id, s.kind, s.form, s.currency, s.amount,
       (s.completed_at AT TIME ZONE 'UTC')::date AS day,
       greatest(h.seller_at, h.buyer_at) AS published_at
FROM bazaar_history_shares h
JOIN bazaar_history_settled s ON s.id = h.listing
JOIN accounts seller ON seller.id = s.seller AND seller.status = 'active'
JOIN accounts buyer ON buyer.id = s.buyer AND buyer.status = 'active'
WHERE h.seller_at IS NOT NULL AND h.buyer_at IS NOT NULL
  AND s.kind = 'item' AND s.completed_at >= now() - interval '90 days'
  AND NOT EXISTS (SELECT 1 FROM bazaar_moderation_holds m WHERE m.listing_id = s.id)
  AND NOT EXISTS (SELECT 1 FROM bazaar_authority_assets a
                  WHERE a.listing_id = s.id AND a.refusal_code IS NOT NULL);

CREATE INDEX bazaar_history_coin_buyer ON bazaar_settlements (buyer) WHERE completed_at IS NOT NULL;
CREATE INDEX bazaar_history_coin_seller ON bazaar_settlements (seller) WHERE completed_at IS NOT NULL;
CREATE INDEX bazaar_history_gold_buyer ON bazaar_authority_gold_sales (buyer) WHERE completed_at IS NOT NULL;
CREATE INDEX bazaar_history_gold_seller ON bazaar_authority_gold_sales (seller) WHERE completed_at IS NOT NULL;
