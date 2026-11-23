ALTER TABLE news ADD COLUMN expires_at timestamptz;
ALTER TABLE news ADD CONSTRAINT news_publication_window
    CHECK (expires_at IS NULL OR (
        isfinite(expires_at) AND published_at IS NOT NULL AND published_at < expires_at
        AND (translation_published_at IS NULL OR translation_published_at < expires_at)
    ));

ALTER TABLE editorial_pages ADD COLUMN expires_at timestamptz;
ALTER TABLE editorial_pages ADD CONSTRAINT editorial_pages_publication_window
    CHECK (expires_at IS NULL OR (
        isfinite(expires_at) AND published_at IS NOT NULL AND published_at < expires_at
    ));
