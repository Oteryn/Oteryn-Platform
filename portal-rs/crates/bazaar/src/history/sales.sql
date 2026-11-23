WITH visible AS (
    SELECT m.id, m.kind, m.form, m.currency, m.amount, m.day,
           'shared' AS role, false AS consent, false AS shareable
    FROM bazaar_history_market m WHERE $1::uuid IS NULL
    UNION ALL
    SELECT s.id, s.kind, s.form, s.currency, s.amount,
           (s.completed_at AT TIME ZONE 'UTC')::date,
           CASE WHEN s.seller = $1 THEN 'sold' ELSE 'bought' END,
           CASE WHEN s.seller = $1 THEN h.seller_at IS NOT NULL ELSE h.buyer_at IS NOT NULL END,
           s.kind='item' AND s.completed_at >= now()-interval '90 days'
    FROM bazaar_history_settled s
    LEFT JOIN bazaar_history_shares h ON h.listing = s.id
    WHERE $1 IN (s.buyer, s.seller)
)
SELECT * FROM visible
WHERE ($2::bigint IS NULL OR amount >= $2)
  AND ($3::bigint IS NULL OR amount <= $3)
  AND ($4::date IS NULL OR day >= $4)
  AND ($5::date IS NULL OR day <= $5)
  AND ($6::text IS NULL OR currency = $6)
  AND ($7::text IS NULL OR form = $7)
  AND ($8::uuid[] IS NULL OR id = ANY($8))
ORDER BY day DESC, id DESC
LIMIT 25 OFFSET $9
