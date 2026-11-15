-- Preserve every historical request, allowing one request per hold revision.
DROP INDEX bazaar_one_invalidation;
CREATE UNIQUE INDEX bazaar_one_invalidation_revision
    ON bazaar_invalidation_requests(listing_id, hold_revision);

-- Accepted bids can also owe an outbid release. Keep their admission outcome
-- separately while the immutable cleanup attempt is reconciled.
ALTER TABLE bazaar_authority_gold_bids
    ADD COLUMN release_accepted boolean NOT NULL DEFAULT false;
UPDATE bazaar_authority_gold_bids b
SET phase='releasing', release_accepted=true, release_operation_id=o.operation_id
FROM bazaar_game_outbox o
WHERE b.phase='accepted' AND b.release_operation_id IS NULL
    AND o.issuance_key='bazaar:' || b.listing_id::text || ':ReleaseBazaarGold:' || b.request_id::text || ':1';
