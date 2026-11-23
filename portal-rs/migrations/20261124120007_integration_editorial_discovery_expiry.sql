-- Match the publication windows and FAQ paths used by the public routes.
CREATE OR REPLACE VIEW editorial_publication AS
SELECT 'pages'::text AS kind, p.id AS document_id, e.locale, p.slug,
       '/' || e.locale || '/' || CASE p.slug
           WHEN 'rules' THEN 'rules'
           WHEN 'support' THEN 'help'
           WHEN 'report-a-bug' THEN 'support/report-a-bug'
           WHEN 'terms-of-service' THEN 'legal/terms'
           WHEN 'privacy-policy' THEN 'legal/privacy'
           WHEN 'cookie-policy' THEN 'legal/cookies'
           WHEN 'server-information' THEN 'server-information'
           ELSE CASE WHEN p.slug LIKE 'faq-%' THEN 'faq/' || substring(p.slug FROM 5)
                     ELSE 'pages/' || p.slug END END AS path,
       greatest(e.updated_at, e.published_at) AS modified_at
FROM pages p
JOIN editorial_pages e ON e.page_id = p.id
JOIN editorial_pages source ON source.page_id = p.id AND source.locale = 'en'
WHERE e.published_at <= statement_timestamp() AND source.published_at <= statement_timestamp()
  AND (e.expires_at IS NULL OR statement_timestamp() < e.expires_at)
  AND (source.expires_at IS NULL OR statement_timestamp() < source.expires_at)
  AND (e.locale = 'en' OR e.source_revision = source.revision)
  AND p.slug <> 'getting-started'
  AND (p.slug NOT IN ('terms-of-service', 'privacy-policy', 'cookie-policy')
       OR (e.legal_effective_date IS NOT NULL AND EXISTS (
           SELECT FROM page_legal_versions edition
           WHERE edition.page_id = p.id AND edition.locale = e.locale AND edition.version = e.legal_version
             AND edition.content @> '{"final_approved":true}'::jsonb
       )))
UNION ALL
SELECT 'news', n.id, language.locale, n.slug, '/' || language.locale || '/news/' || n.slug,
       greatest(n.updated_at, n.published_at,
                CASE WHEN language.locale = 'pl' THEN n.translation_published_at END)
FROM news n CROSS JOIN (VALUES ('en'), ('pl')) language(locale)
WHERE n.published_at <= statement_timestamp()
  AND (n.expires_at IS NULL OR statement_timestamp() < n.expires_at)
  AND (language.locale = 'en' OR (n.translation_published_at <= statement_timestamp()
       AND n.translation_source_updated_at >= n.updated_at
       AND length(trim(n.title_pl)) > 0 AND length(trim(n.body_pl)) > 0))
UNION ALL
SELECT 'events', e.id, t.locale, t.slug, '/' || t.locale || '/events/' || t.slug,
       greatest(e.updated_at, t.updated_at)
FROM events e JOIN event_translations t ON t.event_id = e.id
WHERE e.status = 'published' AND e.published_by IS NOT NULL
UNION ALL
SELECT 'announcements', a.id, language.locale, '', '/' || language.locale || '/announcements/' || a.id,
       greatest(a.updated_at, a.starts_at,
                CASE WHEN language.locale = 'pl' THEN a.translation_published_at END)
FROM announcements a CROSS JOIN (VALUES ('en'), ('pl')) language(locale)
WHERE a.publication_state = 'published' AND a.severity IN ('info', 'maintenance', 'warning')
  AND a.starts_at <= statement_timestamp() AND (a.ends_at IS NULL OR a.ends_at > statement_timestamp())
  AND (language.locale = 'en' OR (a.translation_published_at <= statement_timestamp()
       AND a.translation_source_updated_at >= a.updated_at
       AND length(trim(a.title_pl)) > 0 AND length(trim(a.body_pl)) > 0
       AND (a.action_url IS NULL OR length(trim(a.action_label_pl)) > 0)));
