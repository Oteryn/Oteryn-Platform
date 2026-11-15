-- Bound only the active catalogue; immutable historical definitions remain available.
CREATE INDEX collection_cards_world_version_id_idx
ON collection_cards ((definition->>'world_version'), id);
