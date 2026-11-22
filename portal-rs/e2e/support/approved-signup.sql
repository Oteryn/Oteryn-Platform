-- Approved synthetic editions for a freshly migrated local E2E database.
WITH documents AS (
    INSERT INTO pages(id, slug)
    VALUES (uuidv7(), 'terms-of-service'), (uuidv7(), 'privacy-policy')
    RETURNING id, slug
), sources AS (
    INSERT INTO editorial_pages
        (page_id, locale, title, body, published_at, legal_version, legal_effective_date)
    SELECT id, 'en', slug, 'Approved English signup fixture.', now(),
           CASE slug WHEN 'terms-of-service' THEN $1::text ELSE $2::text END, DATE '2026-11-22'
    FROM documents
    RETURNING *
), translations AS (
    INSERT INTO editorial_pages
        (page_id, locale, title, body, published_at, legal_version, legal_effective_date, source_revision)
    SELECT page_id, 'pl', title, 'Zatwierdzony dokument testowy.', now(),
           legal_version, legal_effective_date, revision
    FROM sources
    RETURNING *
)
INSERT INTO page_legal_versions(page_id, locale, version, content)
SELECT page_id, locale, legal_version, jsonb_build_object(
    'title', title, 'body', body, 'effective', legal_effective_date::text, 'final_approved', true
)
FROM (SELECT * FROM sources UNION ALL SELECT * FROM translations) AS editions;
