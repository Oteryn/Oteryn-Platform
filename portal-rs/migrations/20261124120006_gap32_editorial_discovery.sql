CREATE TABLE editorial_discovery (
    page_id uuid REFERENCES pages(id) ON DELETE CASCADE,
    news_id uuid REFERENCES news(id) ON DELETE CASCADE,
    event_id uuid REFERENCES events(id) ON DELETE CASCADE,
    announcement_id uuid REFERENCES announcements(id) ON DELETE CASCADE,
    kind text GENERATED ALWAYS AS (
        CASE WHEN page_id IS NOT NULL THEN 'pages'
             WHEN news_id IS NOT NULL THEN 'news'
             WHEN event_id IS NOT NULL THEN 'events'
             ELSE 'announcements' END
    ) STORED,
    document_id uuid GENERATED ALWAYS AS (coalesce(page_id, news_id, event_id, announcement_id)) STORED,
    revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    indexing boolean NOT NULL,
    title_en varchar(200) NOT NULL,
    title_pl varchar(200) NOT NULL,
    description_en varchar(320) NOT NULL,
    description_pl varchar(320) NOT NULL,
    placement text,
    menu_key varchar(64) UNIQUE,
    menu_position smallint,
    menu_label_en varchar(80),
    menu_label_pl varchar(80),
    PRIMARY KEY (kind, document_id),
    UNIQUE (placement, menu_position),
    CHECK (num_nonnulls(page_id, news_id, event_id, announcement_id) = 1),
    CHECK (num_nonnulls(placement, menu_key, menu_position, menu_label_en, menu_label_pl) IN (0, 5)),
    CHECK (placement IN ('world', 'community')),
    CHECK (menu_position BETWEEN 1 AND 8),
    CHECK (menu_key ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
    CHECK (btrim(menu_label_en) <> '' AND btrim(menu_label_pl) <> '')
);

-- Publication is read live, including time windows and translation freshness.
-- Each row represents one currently public, canonical document URL.
CREATE VIEW editorial_publication AS
SELECT 'pages'::text AS kind, p.id AS document_id, e.locale,
       '/' || e.locale || '/' || CASE p.slug
           WHEN 'rules' THEN 'rules'
           WHEN 'support' THEN 'help'
           WHEN 'report-a-bug' THEN 'support/report-a-bug'
           WHEN 'terms-of-service' THEN 'legal/terms'
           WHEN 'privacy-policy' THEN 'legal/privacy'
           WHEN 'cookie-policy' THEN 'legal/cookies'
           WHEN 'server-information' THEN 'server-information'
           ELSE 'pages/' || p.slug END AS path,
       greatest(e.updated_at, e.published_at) AS modified_at
FROM pages p
JOIN editorial_pages e ON e.page_id = p.id
JOIN editorial_pages source ON source.page_id = p.id AND source.locale = 'en'
WHERE e.published_at <= now() AND source.published_at <= now()
  AND (e.locale = 'en' OR e.source_revision = source.revision)
  AND p.slug <> 'getting-started'
  AND (p.slug NOT IN ('terms-of-service', 'privacy-policy', 'cookie-policy')
       OR (e.legal_effective_date IS NOT NULL AND EXISTS (
           SELECT FROM page_legal_versions edition
           WHERE edition.page_id = p.id AND edition.locale = e.locale AND edition.version = e.legal_version
             AND edition.content @> '{"final_approved":true}'::jsonb
       )))
UNION ALL
SELECT 'news', n.id, language.locale, '/' || language.locale || '/news/' || n.slug,
       greatest(n.updated_at, n.published_at,
                CASE WHEN language.locale = 'pl' THEN n.translation_published_at END)
FROM news n CROSS JOIN (VALUES ('en'), ('pl')) language(locale)
WHERE n.published_at <= now()
  AND (language.locale = 'en' OR (n.translation_published_at <= now()
       AND n.translation_source_updated_at >= n.updated_at
       AND length(trim(n.title_pl)) > 0 AND length(trim(n.body_pl)) > 0))
UNION ALL
SELECT 'events', e.id, t.locale, '/' || t.locale || '/events/' || t.slug,
       greatest(e.updated_at, t.updated_at)
FROM events e JOIN event_translations t ON t.event_id = e.id
WHERE e.status = 'published' AND e.published_by IS NOT NULL
UNION ALL
SELECT 'announcements', a.id, language.locale, '/' || language.locale || '/announcements/' || a.id,
       greatest(a.updated_at, a.starts_at,
                CASE WHEN language.locale = 'pl' THEN a.translation_published_at END)
FROM announcements a CROSS JOIN (VALUES ('en'), ('pl')) language(locale)
WHERE a.publication_state = 'published' AND a.severity IN ('info', 'maintenance', 'warning')
  AND a.starts_at <= now() AND (a.ends_at IS NULL OR a.ends_at > now())
  AND (language.locale = 'en' OR (a.translation_published_at <= now()
       AND a.translation_source_updated_at >= a.updated_at
       AND length(trim(a.title_pl)) > 0 AND length(trim(a.body_pl)) > 0
       AND (a.action_url IS NULL OR length(trim(a.action_label_pl)) > 0)));
