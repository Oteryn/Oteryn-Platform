WITH documents AS (
 SELECT 'news'::text kind, slug, CASE WHEN $1='pl' THEN title_pl ELSE title END title,
 CASE WHEN $1='pl' THEN body_pl ELSE body END body FROM news
 WHERE published_at<=now() AND ($1='en' OR (translation_published_at<=now() AND translation_source_updated_at>=updated_at AND length(trim(title_pl))>0 AND length(trim(body_pl))>0))
 UNION ALL
 SELECT 'pages',p.slug,e.title,e.body FROM pages p
 JOIN editorial_pages e ON e.page_id=p.id AND e.locale=$1
 JOIN editorial_pages source ON source.page_id=p.id AND source.locale='en'
 WHERE e.published_at<=now() AND source.published_at<=now()
 AND (e.locale='en' OR e.source_revision=source.revision)
 AND (p.slug NOT IN ('terms-of-service','privacy-policy','cookie-policy') OR (e.legal_version IS NOT NULL AND e.legal_effective_date IS NOT NULL))
 UNION ALL
 SELECT 'events',t.slug,t.title,t.summary || ' ' || t.body FROM events e
 JOIN event_translations t ON t.event_id=e.id AND t.locale=$1
 WHERE e.status='published' AND e.published_by IS NOT NULL
 UNION ALL
 SELECT 'wiki',r.slug,r.title,r.summary || ' ' || r.markdown FROM wiki_guides g
 JOIN wiki_revisions r ON r.guide_id=g.id AND r.version=g.revision_version AND r.locale=$1
 WHERE g.status='published' AND g.published_at<=now()
), ranked AS (
 SELECT kind,slug,title,setweight(to_tsvector('simple',title),'A') || to_tsvector('simple',body) document FROM documents
)
SELECT kind,slug,title FROM ranked WHERE ($3='' OR kind=$3) AND document @@ plainto_tsquery('simple',$2)
ORDER BY ts_rank(document,plainto_tsquery('simple',$2)) DESC,kind COLLATE "C",slug COLLATE "C"
LIMIT 21 OFFSET $4
