-- Owner decision 2026-11-21: the breach corpus is the HIBP range API with a 30-day cache.
-- Existing operator-imported rows kept only a subset of each range, so they become
-- incomplete and no longer decide a lookup; only rows written from a full fetch are complete.
ALTER TABLE registration_password_ranges ADD COLUMN complete boolean NOT NULL DEFAULT false;

-- A row is complete only when the statement writing it asserts so by stamping its own
-- transaction id into `complete_xact`, on INSERT and on UPDATE (COPY fires the same
-- trigger). The guard then clears the stamp, so a later write in the same transaction or
-- savepoint cannot inherit it. SQL from an older importer, which never names `complete`,
-- or any unstamped write therefore leaves the row incomplete, and the live check decides.
-- Rollback order: drop the trigger and the function before the columns.
ALTER TABLE registration_password_ranges ADD COLUMN complete_xact xid8;

CREATE FUNCTION registration_password_ranges_complete_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.complete AND NEW.complete_xact IS DISTINCT FROM pg_current_xact_id() THEN
        NEW.complete := false;
    END IF;
    NEW.complete_xact := NULL;
    RETURN NEW;
END
$$;

CREATE TRIGGER registration_password_ranges_complete_guard
    BEFORE INSERT OR UPDATE ON registration_password_ranges
    FOR EACH ROW EXECUTE FUNCTION registration_password_ranges_complete_guard();
