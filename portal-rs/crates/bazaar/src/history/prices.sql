SELECT form, currency, to_char(period, 'YYYY-MM') AS month,
       count(*) AS samples, percentile_disc(0.5) WITHIN GROUP (ORDER BY amount) AS median
FROM (
    SELECT form, currency, amount, date_trunc('month', day::timestamp) AS period
    FROM bazaar_history_market
) sales
GROUP BY form, currency, GROUPING SETS ((period), ())
HAVING count(*) >= 5
ORDER BY form, currency, period DESC NULLS FIRST
