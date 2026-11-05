WITH candidates AS (
 SELECT e.kind,e.key,e.data,
 COALESCE(e.data->'definition',e.data->'declaration',e.data#>'{bundle,spell}',e.data) body
 FROM world_entities e
 JOIN world_active_version a ON a.version_id=e.version_id
 JOIN world_visibility v ON v.kind=e.kind AND v.key=e.key AND v.allowed
 WHERE e.publishable AND e.kind IN ('Item','Creature','NPC','Spell','Quest','Loot','Achievement','Area')
)
SELECT kind,key FROM candidates
WHERE to_tsvector('simple',key || ' ' || COALESCE(data#>>'{authoring,profile,details,display_name}',body->'names'->>$3,body->>'display_name',body->>'name','')) @@ plainto_tsquery('simple',$1)
ORDER BY kind COLLATE "C",key COLLATE "C" LIMIT 21 OFFSET $2
