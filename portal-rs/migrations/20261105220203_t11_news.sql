CREATE TABLE news (
 id uuid PRIMARY KEY DEFAULT uuidv7(),
 slug varchar(160) NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
 title varchar(200) NOT NULL CHECK (length(trim(title)) > 0),
 body text NOT NULL CHECK (length(trim(body)) > 0 AND length(body) <= 100000),
 published_at timestamptz CHECK (isfinite(published_at)),
 title_pl varchar(200),
 body_pl text CHECK (length(body_pl) <= 100000),
 translation_published_at timestamptz CHECK (isfinite(translation_published_at)),
 translation_source_updated_at timestamptz,
 created_at timestamptz NOT NULL DEFAULT now(),
 updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX news_publication ON news (published_at DESC, id DESC);
