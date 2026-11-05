CREATE TABLE announcements (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    publication_state text NOT NULL DEFAULT 'draft' CHECK (publication_state IN ('draft','published')),
    severity text NOT NULL CHECK (severity IN ('info','maintenance','warning')),
    starts_at timestamptz NOT NULL,
    ends_at timestamptz CHECK (ends_at > starts_at),
    title text NOT NULL CHECK (length(btrim(title)) BETWEEN 1 AND 200),
    body text NOT NULL CHECK (length(btrim(body)) BETWEEN 1 AND 10000),
    action_label text,
    action_url text,
    title_pl text,
    body_pl text,
    action_label_pl text,
    translation_published_at timestamptz,
    translation_source_updated_at timestamptz,
    created_by uuid NOT NULL,
    updated_by uuid NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    lock_version bigint NOT NULL DEFAULT 1 CHECK (lock_version > 0),
    CHECK ((action_url IS NULL AND action_label IS NULL) OR
        (action_label IS NOT NULL AND action_url IS NOT NULL AND action_url ~ '^/([^/]|$)' AND action_url !~ '[[:space:]\\<>"''?#]' AND length(action_label) BETWEEN 1 AND 100))
);
CREATE INDEX announcements_active ON announcements (severity DESC, starts_at DESC, id DESC) WHERE publication_state='published';
