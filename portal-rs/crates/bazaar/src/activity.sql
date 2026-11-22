WITH owned_listings AS (
    SELECT * FROM bazaar_listings WHERE owner = $1 AND ($2::uuid IS NULL OR id = $2)
), listing_changes AS (
    -- Private proxy maximum updates leave all four public listing fields unchanged.
    SELECT a.row_id, max(a.created_at) AS updated_at
    FROM bazaar_audit a JOIN owned_listings l ON l.id = a.row_id
    WHERE a.relation_name = 'bazaar_listings' AND a.before_row IS NOT NULL
      AND (a.before_row->'state', a.before_row->'price', a.before_row->'ends_at', a.before_row->'accept_offers')
          IS DISTINCT FROM
          (a.after_row->'state', a.after_row->'price', a.after_row->'ends_at', a.after_row->'accept_offers')
    GROUP BY a.row_id
), settlements AS (
    SELECT id, listing_id, currency::text AS currency, amount, completed_at, updated_at
    FROM bazaar_settlements
    WHERE (buyer = $1 OR seller = $1) AND ($2 IS NULL OR listing_id = $2)
    UNION ALL
    SELECT id, listing_id, 'gold', gross_amount, completed_at, NULL
    FROM bazaar_authority_gold_sales
    WHERE (buyer = $1 OR seller = $1) AND ($2 IS NULL OR listing_id = $2)
), activity AS (
    SELECT 0 AS section, l.id, l.id AS listing, l.kind::text AS kind,
           l.currency::text AS currency, l.price AS amount,
           CASE WHEN a.refusal_code IS NOT NULL THEN 'refused'
                WHEN a.cancel_requested AND l.state NOT IN ('completed', 'cancelled', 'expired')
                    THEN 'cancel_requested'
                WHEN h.listing_id IS NOT NULL AND l.state IN ('draft', 'active')
                    THEN 'suspended'
                ELSE l.state::text END AS state,
           greatest(l.created_at, change.updated_at, h.updated_at,
               CASE WHEN a.cancel_requested AND l.state NOT IN ('completed', 'cancelled', 'expired')
                    THEN (
                        SELECT max(r.created_at)
                        FROM bazaar_authority_browser_requests r
                        WHERE r.listing_id = l.id AND r.actor = l.owner AND r.action = 'cancel'
                    )
               END) AS updated_at
    FROM owned_listings l
    LEFT JOIN listing_changes change ON change.row_id = l.id
    LEFT JOIN bazaar_authority_assets a ON a.listing_id = l.id
    LEFT JOIN bazaar_moderation_holds h ON h.listing_id = l.id
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
    SELECT 2, s.id, l.id, l.kind::text, s.currency, s.amount,
           CASE WHEN s.completed_at IS NOT NULL THEN 'settlement_completed'
                WHEN l.state = 'cancelled' THEN 'settlement_cancelled'
                WHEN refusal.command = 'PayoutBazaarGold' THEN 'settlement_payout_refused'
                WHEN l.state = 'awaiting_delivery'
                     AND (refusal.command IS NOT NULL OR a.refusal_code IS NOT NULL)
                    THEN 'settlement_refused'
                ELSE 'settlement_pending' END,
           greatest(s.updated_at, s.completed_at, l.updated_at, refusal.updated_at)
    FROM settlements s JOIN bazaar_listings l ON l.id = s.listing_id
    LEFT JOIN bazaar_authority_assets a ON a.listing_id = s.listing_id
    LEFT JOIN LATERAL (
        -- Use the latest operation for each phase of this sale, in the current recovery generation.
        SELECT command.command, greatest(r.created_at, decision.created_at) AS updated_at
        FROM (
            SELECT DISTINCT ON (o.command) o.operation_id, o.command
            FROM bazaar_game_outbox o
            WHERE o.listing_id = s.listing_id AND o.intent::jsonb->>'sale_id' = s.id::text
              AND (o.command = CASE l.kind WHEN 'character' THEN 'BindBazaarCharacterTransfer'
                                          ELSE 'DeliverBazaarItem' END
                   OR (s.currency = 'gold' AND o.command = 'PayoutBazaarGold'))
            ORDER BY o.command, o.source_revision DESC
        ) command
        JOIN bazaar_authority_receipts r USING (operation_id)
        JOIN bazaar_game_issuer issuer ON issuer.singleton AND issuer.recovery_generation = r.recovery_generation
        JOIN LATERAL (
            SELECT d.created_at FROM bazaar_authority_decisions d
            WHERE d.operation_id = r.operation_id AND d.recovery_generation = r.recovery_generation
            ORDER BY d.created_at LIMIT 1
        ) decision ON true
        WHERE r.outcome = 'REJECTED'
          AND NOT EXISTS (SELECT 1 FROM bazaar_authority_conflicts c WHERE c.operation_id = r.operation_id)
        ORDER BY (command.command = 'PayoutBazaarGold') DESC
        LIMIT 1
    ) refusal ON l.state = 'awaiting_delivery' AND s.completed_at IS NULL
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
