CREATE TABLE media (
    id uuid PRIMARY KEY,
    alt_text text NOT NULL CHECK (length(btrim(alt_text)) BETWEEN 1 AND 500),
    mime_type text NOT NULL CHECK (mime_type IN ('image/jpeg','image/png','image/webp')),
    width integer NOT NULL CHECK (width BETWEEN 1 AND 6000),
    height integer NOT NULL CHECK (height BETWEEN 1 AND 6000),
    content bytea NOT NULL CHECK (octet_length(content) BETWEEN 1 AND 8388608),
    sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),
    thumbnail bytea,
    thumbnail_sha256 bytea,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (width::bigint * height <= 20000000),
    CHECK ((thumbnail IS NULL AND thumbnail_sha256 IS NULL) OR
           (thumbnail IS NOT NULL AND thumbnail_sha256 IS NOT NULL
            AND octet_length(thumbnail) BETWEEN 1 AND 8388608
            AND octet_length(thumbnail_sha256) = 32))
);
-- Consumers insert references transactionally; the foreign key refuses deletion.
CREATE TABLE media_references (
    media_id uuid NOT NULL REFERENCES media(id) ON DELETE RESTRICT,
    consumer text NOT NULL CHECK (length(consumer) BETWEEN 1 AND 256),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (media_id, consumer)
);
