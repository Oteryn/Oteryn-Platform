CREATE TABLE editorial_revisions (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    kind text NOT NULL,
    object_key text NOT NULL,
    locale text NOT NULL,
    actor uuid,
    saved_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    restored_from bigint REFERENCES editorial_revisions(id),
    baseline boolean NOT NULL DEFAULT false,
    snapshot jsonb NOT NULL CHECK (jsonb_typeof(snapshot) = 'object'),
    CHECK ((kind = 'page' AND locale IN ('pl','en')) OR
           (kind IN ('news','announcement') AND locale = ''))
);

CREATE INDEX editorial_revisions_history ON editorial_revisions (kind, object_key, locale, id DESC);

-- Capture the current editions so they are recoverable after the first edit.
-- Older news and page editions have no durable author on the live row.
INSERT INTO editorial_revisions (kind, object_key, locale, saved_at, baseline, snapshot)
SELECT 'news', id::text, '', updated_at, true, to_jsonb(n)
FROM news n;

INSERT INTO editorial_revisions (kind, object_key, locale, saved_at, baseline, snapshot)
SELECT 'page', p.slug, e.locale, e.updated_at, true, to_jsonb(e)
FROM editorial_pages e JOIN pages p ON p.id = e.page_id
WHERE p.slug NOT IN ('terms-of-service', 'privacy-policy', 'cookie-policy');

INSERT INTO editorial_revisions (kind, object_key, locale, actor, baseline, snapshot)
SELECT 'announcement', id::text, '', updated_by, true, to_jsonb(a)
FROM announcements a;

CREATE FUNCTION deny_editorial_revision_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'editorial history is append-only';
END;
$$;

CREATE TRIGGER editorial_revisions_immutable BEFORE UPDATE OR DELETE ON editorial_revisions
    FOR EACH ROW EXECUTE FUNCTION deny_editorial_revision_mutation();
CREATE TRIGGER editorial_revisions_no_truncate BEFORE TRUNCATE ON editorial_revisions
    FOR EACH STATEMENT EXECUTE FUNCTION deny_editorial_revision_mutation();
