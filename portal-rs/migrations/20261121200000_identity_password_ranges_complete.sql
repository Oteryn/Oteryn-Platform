-- Owner decision 2026-11-21: the breach corpus is the HIBP range API with a 30-day cache.
-- Existing operator-imported rows kept only a subset of each range, so they become
-- incomplete and no longer decide a lookup; only rows written from a full fetch are complete.
ALTER TABLE registration_password_ranges ADD COLUMN complete boolean NOT NULL DEFAULT false;

-- A row stays complete only when the writing transaction asserts it by stamping its own
-- transaction id. An older importer's conflict update (which never names `complete`)
-- replaces the suffixes of a complete row with a thresholded subset; the guard then turns
-- the row incomplete instead of letting the subset decide a lookup.
ALTER TABLE registration_password_ranges ADD COLUMN complete_xact xid8;

CREATE FUNCTION registration_password_ranges_complete_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.complete AND NEW.complete_xact IS DISTINCT FROM pg_current_xact_id() THEN
        NEW.complete := false;
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER registration_password_ranges_complete_guard
    BEFORE UPDATE ON registration_password_ranges
    FOR EACH ROW EXECUTE FUNCTION registration_password_ranges_complete_guard();
