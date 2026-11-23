ALTER TABLE media ADD COLUMN original_name text
    CHECK (length(original_name) BETWEEN 1 AND 255);

ALTER TABLE news ADD COLUMN media_id uuid REFERENCES media(id) ON DELETE RESTRICT;
CREATE INDEX news_media_id_idx ON news(media_id) WHERE media_id IS NOT NULL;
