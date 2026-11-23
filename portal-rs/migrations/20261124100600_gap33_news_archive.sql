ALTER TABLE news
    ADD COLUMN is_changelog boolean NOT NULL DEFAULT false,
    ADD COLUMN tags text[] NOT NULL DEFAULT '{}'
        CHECK (cardinality(tags) <= 10);

CREATE INDEX news_section_publication ON news (is_changelog, published_at DESC, id DESC)
    WHERE published_at IS NOT NULL;
CREATE INDEX news_tags ON news USING gin (tags) WHERE NOT is_changelog;
