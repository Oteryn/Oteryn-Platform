-- Editorial text only; world statistics remain owned by the world module.
CREATE TABLE wiki_guides (
    id uuid PRIMARY KEY,
    status text NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','in_review','published','archived')),
    version bigint NOT NULL DEFAULT 1 CHECK (version > 0),
    revision_version bigint NOT NULL DEFAULT 1 CHECK (revision_version > 0),
    entity_key text CHECK (length(entity_key) BETWEEN 1 AND 256),
    published_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK ((status = 'published') = (published_at IS NOT NULL))
);
CREATE TABLE wiki_revisions (
    id uuid PRIMARY KEY,
    guide_id uuid NOT NULL REFERENCES wiki_guides(id),
    version bigint NOT NULL CHECK (version > 0),
    locale text NOT NULL CHECK (locale IN ('pl','en')),
    title text NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
    slug text NOT NULL CHECK (length(slug) <= 160 AND slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
    summary text NOT NULL CHECK (length(summary) <= 1000),
    markdown text NOT NULL CHECK (length(markdown) <= 100000),
    editor uuid NOT NULL,
    change_note text NOT NULL CHECK (length(change_note) <= 500),
    source_version bigint,
    search_document tsvector GENERATED ALWAYS AS
        (setweight(to_tsvector('simple', title), 'A') || setweight(to_tsvector('simple', summary), 'B') || to_tsvector('simple', markdown)) STORED,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (guide_id, version, locale)
);
CREATE INDEX wiki_revisions_search ON wiki_revisions USING gin(search_document);
CREATE INDEX wiki_revisions_slug ON wiki_revisions(locale, slug);
CREATE TABLE wiki_reviews (
    id uuid PRIMARY KEY,
    guide_id uuid NOT NULL REFERENCES wiki_guides(id),
    revision_version bigint NOT NULL,
    actor uuid NOT NULL,
    action text NOT NULL CHECK (action IN ('submit-review','return-draft','publish','unpublish','archive')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE FUNCTION wiki_deny_history_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'wiki history is append-only';
END;
$$;
CREATE TRIGGER wiki_revisions_immutable BEFORE UPDATE OR DELETE ON wiki_revisions
    FOR EACH ROW EXECUTE FUNCTION wiki_deny_history_mutation();
CREATE TRIGGER wiki_reviews_immutable BEFORE UPDATE OR DELETE ON wiki_reviews
    FOR EACH ROW EXECUTE FUNCTION wiki_deny_history_mutation();
CREATE TRIGGER wiki_revisions_no_truncate BEFORE TRUNCATE ON wiki_revisions
    FOR EACH STATEMENT EXECUTE FUNCTION wiki_deny_history_mutation();
CREATE TRIGGER wiki_reviews_no_truncate BEFORE TRUNCATE ON wiki_reviews
    FOR EACH STATEMENT EXECUTE FUNCTION wiki_deny_history_mutation();
