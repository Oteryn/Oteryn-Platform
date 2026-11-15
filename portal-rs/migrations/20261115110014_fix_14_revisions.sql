-- A retained translation must never match a recreated source revision.
CREATE SEQUENCE editorial_page_revision_seq;
SELECT setval('editorial_page_revision_seq', GREATEST(COALESCE((SELECT max(GREATEST(revision, source_revision)) FROM editorial_pages), 0), 1));
ALTER TABLE editorial_pages ALTER COLUMN revision SET DEFAULT nextval('editorial_page_revision_seq');
