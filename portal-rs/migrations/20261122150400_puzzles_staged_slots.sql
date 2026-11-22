-- Future slots are staged in place, including slots created before this migration.
-- Replacement is delete-and-insert so a new slot always gets a fresh puzzle ID.
CREATE OR REPLACE FUNCTION puzzles_daily_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' AND OLD.day > (clock_timestamp() AT TIME ZONE 'UTC')::date THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'published daily puzzles are immutable';
END;
$$;

-- A transaction that deletes staged slots must still be before release at commit.
CREATE CONSTRAINT TRIGGER puzzles_daily_staged_cutoff
    AFTER DELETE ON puzzles_daily DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION puzzles_daily_immutable();
