CREATE TABLE events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    status text NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'scheduled', 'published', 'archived')),
    starts_at timestamptz NOT NULL,
    ends_at timestamptz NOT NULL CHECK (ends_at > starts_at),
    featured boolean NOT NULL DEFAULT false,
    lock_version bigint NOT NULL DEFAULT 1 CHECK (lock_version > 0),
    created_by uuid NOT NULL,
    updated_by uuid NOT NULL,
    published_by uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (status = 'draft' OR published_by IS NOT NULL)
);
CREATE TABLE event_translations (
    event_id uuid NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    locale text NOT NULL CHECK (locale IN ('pl', 'en')),
    title text NOT NULL CHECK (length(btrim(title)) BETWEEN 1 AND 200),
    slug text NOT NULL CHECK (length(slug) BETWEEN 1 AND 160 AND slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
    summary text NOT NULL CHECK (length(btrim(summary)) BETWEEN 1 AND 500),
    body text NOT NULL CHECK (length(btrim(body)) BETWEEN 1 AND 100000),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (event_id, locale),
    UNIQUE (locale, slug)
);
CREATE INDEX events_public_order ON events (featured DESC, starts_at, id) WHERE status = 'published';
