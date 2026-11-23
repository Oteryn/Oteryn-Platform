-- The final write of a budgeted transaction queues this guard at the end of
-- PostgreSQL's deferred trigger queue. The guard consumes its own row.
CREATE TABLE mutation_commit_deadlines (
    transaction_id xid8 PRIMARY KEY DEFAULT pg_current_xact_id(),
    expires_at timestamptz NOT NULL
);

CREATE FUNCTION enforce_mutation_commit_deadline() RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    -- Finish any additional deferred work queued by preceding triggers.
    SET CONSTRAINTS ALL IMMEDIATE;
    DELETE FROM mutation_commit_deadlines WHERE transaction_id = NEW.transaction_id;
    IF clock_timestamp() >= NEW.expires_at THEN
        RAISE EXCEPTION 'mutation commit deadline exceeded' USING ERRCODE = '57014';
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER mutation_commit_deadline
AFTER INSERT ON mutation_commit_deadlines DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION enforce_mutation_commit_deadline();
