-- Store price list S-5 follows Store wire owner decision D1 (2026-11-02): every
-- offer is quoted in COINS. Rows are never converted or deleted here; an existing
-- offer in another currency stops the migration so an operator decides first.
-- store_offers has no offer_key CHECK, so the Game key pattern stays in the importer.
LOCK TABLE store_offers IN ACCESS EXCLUSIVE MODE;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM store_offers WHERE currency <> 'COINS') THEN
        RAISE EXCEPTION 'store_offers holds offers not quoted in COINS; owner decision D1 allows COINS only, resolve those catalog revisions before this migration';
    END IF;
END;
$$;
ALTER TABLE store_offers DROP CONSTRAINT store_offers_currency_check;
ALTER TABLE store_offers ADD CONSTRAINT store_offers_currency_check CHECK (currency = 'COINS');
