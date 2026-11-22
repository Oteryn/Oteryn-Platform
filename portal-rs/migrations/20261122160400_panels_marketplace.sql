-- Support account-scoped activity reads without scanning other players' rows.
CREATE INDEX bazaar_listings_activity_owner ON bazaar_listings (owner, updated_at DESC, id DESC);
CREATE INDEX bazaar_bids_activity_account ON bazaar_bids (account, listing_id);
CREATE INDEX bazaar_gold_bids_activity_buyer ON bazaar_authority_gold_bids (buyer, listing_id);
CREATE INDEX bazaar_settlements_activity_buyer ON bazaar_settlements (buyer, listing_id);
CREATE INDEX bazaar_settlements_activity_seller ON bazaar_settlements (seller, listing_id);
CREATE INDEX bazaar_gold_sales_activity_buyer ON bazaar_authority_gold_sales (buyer, listing_id);
CREATE INDEX bazaar_gold_sales_activity_seller ON bazaar_authority_gold_sales (seller, listing_id);
CREATE INDEX bazaar_item_sales_activity_buyer ON bazaar_item_sales (buyer, listing_id);
CREATE INDEX bazaar_item_sales_activity_seller ON bazaar_item_sales (seller, listing_id);
