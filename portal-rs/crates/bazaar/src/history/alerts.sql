SELECT m.id, m.kind, m.form, m.currency, m.amount, m.day,
       'shared' AS role, false AS consent, false AS shareable
FROM bazaar_history_preferences p
JOIN bazaar_history_market m ON m.published_at > p.alerts_since
WHERE p.account = $1
  AND NOT EXISTS (SELECT 1 FROM bazaar_history_dismissals d WHERE d.account = $1 AND d.sale = m.id)
  AND EXISTS (
      SELECT 1 FROM bazaar_history_searches s
      WHERE s.account = $1 AND m.published_at > s.saved_at
        AND m.amount >= coalesce(nullif(s.criteria->>'min_price','')::bigint, m.amount)
        AND m.amount <= coalesce(nullif(s.criteria->>'max_price','')::bigint, m.amount)
        AND m.day >= coalesce(nullif(s.criteria->>'from','')::date, m.day)
        AND m.day <= coalesce(nullif(s.criteria->>'to','')::date, m.day)
        AND m.currency = coalesce(nullif(s.criteria->>'currency',''), m.currency)
        AND m.form = coalesce(nullif(s.criteria->>'form',''), m.form)
  )
ORDER BY m.published_at DESC, m.id DESC
LIMIT 24
