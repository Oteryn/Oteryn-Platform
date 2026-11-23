-- Legacy rows keep their UTC occurrence date. New rows carry their evidence day
-- explicitly so writes arriving after a seal can join the current open day.
ALTER TABLE audit_log ADD COLUMN evidence_day date;
CREATE INDEX audit_log_evidence_day_idx
    ON audit_log ((coalesce(evidence_day, (created_at AT TIME ZONE 'UTC')::date)), id);

CREATE TABLE audit_evidence_records (
    id uuid PRIMARY KEY REFERENCES audit_log(id),
    day date NOT NULL,
    digest bytea NOT NULL CHECK (octet_length(digest) = 32)
);
CREATE INDEX audit_evidence_records_day_idx ON audit_evidence_records (day, id);

CREATE TABLE audit_evidence_seals (
    day date PRIMARY KEY,
    head bytea NOT NULL CHECK (octet_length(head) = 32),
    entry_count integer NOT NULL CHECK (entry_count >= 0),
    sealed_at timestamptz NOT NULL DEFAULT clock_timestamp()
);

CREATE TRIGGER audit_evidence_records_append_only
    BEFORE UPDATE OR DELETE OR TRUNCATE ON audit_evidence_records
    FOR EACH STATEMENT EXECUTE FUNCTION reject_audit_log_mutation();
CREATE TRIGGER audit_evidence_seals_append_only
    BEFORE UPDATE OR DELETE OR TRUNCATE ON audit_evidence_seals
    FOR EACH STATEMENT EXECUTE FUNCTION reject_audit_log_mutation();

-- Keep the v1 encoding stable across schema changes and server session time zones.
CREATE FUNCTION audit_evidence_digest(entry audit_log) RETURNS bytea
    LANGUAGE sql STABLE SET TimeZone = 'UTC'
AS $$
    SELECT sha256(convert_to(jsonb_build_array(
        entry.id, entry.actor, entry.action, entry.target, entry.before,
        entry.after, entry.request_id, entry.created_at, entry.admin_operation_id,
        entry.operation, entry.risk_class, entry.admin_control_target, entry.reason,
        entry.approvals, entry.ip, entry.user_agent, entry.correlation_id,
        entry.outcome, entry.game_ledger_id, entry.evidence_day
    )::text, 'UTF8'));
$$;

INSERT INTO audit_evidence_records (id, day, digest)
SELECT entry.id, (entry.created_at AT TIME ZONE 'UTC')::date, audit_evidence_digest(entry)
FROM audit_log AS entry;

-- The matching exclusive lock in the exporter waits for pending writes and
-- protects the transition from an open day to an immutable seal.
CREATE FUNCTION assign_audit_evidence_day() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    open_day date;
BEGIN
    IF current_setting('transaction_isolation') IN ('read committed', 'read uncommitted')
       AND NEW.created_at <> transaction_timestamp() THEN
        NEW.evidence_day := least(
            (NEW.created_at AT TIME ZONE 'UTC')::date,
            (clock_timestamp() AT TIME ZONE 'UTC')::date
        );
        IF pg_try_advisory_xact_lock_shared(20261124, NEW.evidence_day - DATE '2000-01-01') THEN
            IF NOT EXISTS (SELECT 1 FROM audit_evidence_seals WHERE day = NEW.evidence_day) THEN
                RETURN NEW;
            END IF;
        END IF;
    END IF;
    LOOP
        open_day := (clock_timestamp() AT TIME ZONE 'UTC')::date;
        IF pg_try_advisory_xact_lock_shared(20261124, open_day - DATE '2000-01-01') THEN
            IF open_day = (clock_timestamp() AT TIME ZONE 'UTC')::date
               AND NOT EXISTS (SELECT 1 FROM audit_evidence_seals WHERE day = open_day) THEN
                NEW.evidence_day := open_day;
                RETURN NEW;
            END IF;
        END IF;
        IF open_day = (clock_timestamp() AT TIME ZONE 'UTC')::date THEN
            RAISE EXCEPTION 'no open audit evidence day' USING ERRCODE = '40001';
        END IF;
    END LOOP;
END;
$$;
CREATE TRIGGER audit_evidence_assign_day
    BEFORE INSERT ON audit_log
    FOR EACH ROW EXECUTE FUNCTION assign_audit_evidence_day();

-- AFTER INSERT captures the final row and also supports INSERT ... ON CONFLICT.
CREATE FUNCTION capture_audit_evidence() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO audit_evidence_records (id, day, digest)
    VALUES (NEW.id, NEW.evidence_day, audit_evidence_digest(NEW));
    RETURN NEW;
END;
$$;
CREATE TRIGGER audit_evidence_capture
    AFTER INSERT ON audit_log
    FOR EACH ROW EXECUTE FUNCTION capture_audit_evidence();
