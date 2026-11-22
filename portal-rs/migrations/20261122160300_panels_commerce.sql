-- Staff price proposals are separate from the immutable Game import namespace.
CREATE TABLE store_price_proposals (
    base_revision text NOT NULL,
    offer_key text NOT NULL,
    unit_price bigint NOT NULL CHECK (unit_price > 0),
    proposed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (base_revision, offer_key, unit_price),
    FOREIGN KEY (base_revision, offer_key) REFERENCES store_offers(catalog_revision, offer_key)
);
