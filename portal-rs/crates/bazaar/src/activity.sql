WITH activity AS (
    SELECT 0 AS section, l.id, l.id AS listing, l.kind::text AS kind,
           l.currency::text AS currency, l.price AS amount,
           CASE WHEN a.refusal_code IS NOT NULL THEN 'refused'
                WHEN a.cancel_requested AND l.state NOT IN ('completed', 'cancelled', 'expired')
                    THEN 'cancel_requested'
                WHEN h.listing_id IS NOT NULL AND l.state IN ('draft', 'active')
                    THEN 'suspended'
                ELSE l.state::text END AS state, greatest(l.updated_at, h.updated_at) AS updated_at
    FROM bazaar_listings l
    LEFT JOIN bazaar_authority_assets a ON a.listing_id = l.id
    LEFT JOIN bazaar_moderation_holds h ON h.listing_id = l.id
    WHERE l.owner = $1 AND ($2::uuid IS NULL OR l.id = $2)
    UNION ALL
    SELECT 1, b.id, l.id, l.kind::text, l.currency::text, b.maximum,
           'recorded', b.created_at
    FROM bazaar_bids b JOIN bazaar_listings l ON l.id = b.listing_id
    WHERE b.account = $1 AND ($2 IS NULL OR b.listing_id = $2)
    AND NOT EXISTS (
        SELECT 1 FROM bazaar_authority_gold_bids g
        WHERE g.listing_id = b.listing_id AND g.buyer = b.account
        AND g.request_id::text = b.idempotency_key
    )
    UNION ALL
    SELECT 1, g.request_id, l.id, l.kind::text, l.currency::text, g.maximum,
           -- Release work can belong to the previous leader. Show only this bidder's decision.
           CASE WHEN g.phase = 'releasing'
                    THEN CASE WHEN g.release_accepted THEN 'accepted' ELSE 'refused' END
                ELSE g.phase END,
           CASE WHEN g.phase = 'pending' THEN h.created_at
                ELSE coalesce(b.created_at, hold_receipt.created_at, h.created_at) END
    FROM bazaar_authority_gold_bids g JOIN bazaar_listings l ON l.id = g.listing_id
    JOIN bazaar_game_outbox h ON h.operation_id = g.hold_operation_id
    LEFT JOIN bazaar_bids b ON b.listing_id = g.listing_id
        AND b.account = g.buyer AND b.idempotency_key = g.request_id::text
    LEFT JOIN LATERAL (
        SELECT min(r.created_at) AS created_at
        FROM bazaar_authority_receipts r WHERE r.operation_id = g.hold_operation_id
    ) hold_receipt ON g.phase <> 'pending' AND b.id IS NULL
    WHERE g.buyer = $1 AND ($2 IS NULL OR g.listing_id = $2)
    UNION ALL
    SELECT 2, s.id, l.id, l.kind::text, s.currency::text, s.amount,
           CASE WHEN s.completed_at IS NOT NULL THEN 'settlement_completed'
                WHEN l.state = 'cancelled' THEN 'settlement_cancelled'
                ELSE 'settlement_pending' END,
           greatest(s.updated_at, s.completed_at, l.updated_at)
    FROM bazaar_settlements s JOIN bazaar_listings l ON l.id = s.listing_id
    WHERE (s.buyer = $1 OR s.seller = $1) AND ($2 IS NULL OR s.listing_id = $2)
    UNION ALL
    SELECT 2, s.id, l.id, l.kind::text, 'gold', s.gross_amount,
           CASE WHEN s.completed_at IS NOT NULL THEN 'settlement_completed'
                WHEN l.state = 'cancelled' THEN 'settlement_cancelled'
                ELSE 'settlement_pending' END,
           coalesce(s.completed_at, l.updated_at)
    FROM bazaar_authority_gold_sales s JOIN bazaar_listings l ON l.id = s.listing_id
    WHERE (s.buyer = $1 OR s.seller = $1) AND ($2 IS NULL OR s.listing_id = $2)
    UNION ALL
    SELECT 2, s.id, l.id, l.kind::text, l.currency::text, s.gross_amount, 'matched', s.updated_at
    FROM bazaar_item_sales s JOIN bazaar_listings l ON l.id = s.listing_id
    WHERE (s.buyer = $1 OR s.seller = $1) AND ($2 IS NULL OR s.listing_id = $2)
    AND NOT EXISTS (SELECT 1 FROM bazaar_settlements settled WHERE settled.listing_id = l.id)
    AND NOT EXISTS (SELECT 1 FROM bazaar_authority_gold_sales settled WHERE settled.listing_id = l.id)
)
SELECT section, listing, kind, currency, amount, state, updated_at
FROM (
    SELECT *, row_number() OVER (PARTITION BY section ORDER BY updated_at DESC, id DESC) AS position
    FROM activity
) ranked
WHERE position > $3 AND position <= $3 + $4::bigint
ORDER BY section, position
