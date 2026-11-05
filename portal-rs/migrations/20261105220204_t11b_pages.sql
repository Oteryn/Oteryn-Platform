-- Platform-owned editorial content; no game or account mutations.
CREATE TABLE pages (
    id uuid PRIMARY KEY,
    slug varchar(160) NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE editorial_pages (
    page_id uuid NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    locale text NOT NULL CHECK (locale IN ('pl','en')),
    title varchar(200) NOT NULL CHECK (length(trim(title)) > 0),
    body text NOT NULL CHECK (length(body) BETWEEN 1 AND 100000),
    published_at timestamptz,
    source_revision bigint NOT NULL DEFAULT 1,
    revision bigint NOT NULL DEFAULT 1,
    legal_version varchar(80),
    legal_effective_date date,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (page_id, locale)
);
CREATE TABLE page_legal_versions (
    page_id uuid NOT NULL REFERENCES pages(id),
    locale text NOT NULL,
    version varchar(80) NOT NULL,
    content jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (page_id,locale,version)
);
CREATE FUNCTION preserve_page_legal_version() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'published legal versions are immutable';
END;
$$;
CREATE TRIGGER page_legal_versions_immutable BEFORE UPDATE OR DELETE ON page_legal_versions
FOR EACH ROW EXECUTE FUNCTION preserve_page_legal_version();
