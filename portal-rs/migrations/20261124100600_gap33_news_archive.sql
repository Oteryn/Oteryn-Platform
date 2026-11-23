ALTER TABLE news
    ADD COLUMN is_changelog boolean NOT NULL DEFAULT false,
    ADD COLUMN tags text[] NOT NULL DEFAULT '{}'
        CHECK (cardinality(tags) <= 10),
    ADD CONSTRAINT news_publication_date_range CHECK (
        published_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND published_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    ) NOT VALID,
    ADD CONSTRAINT news_translation_date_range CHECK (
        translation_published_at >= TIMESTAMPTZ '0001-01-01 00:00:00+00'
        AND translation_published_at < TIMESTAMPTZ '10000-01-01 00:00:00+00'
    ) NOT VALID;

CREATE INDEX news_section_publication ON news (is_changelog, published_at DESC, id DESC)
    WHERE published_at IS NOT NULL;
CREATE INDEX news_tags ON news USING gin (tags) WHERE NOT is_changelog;
