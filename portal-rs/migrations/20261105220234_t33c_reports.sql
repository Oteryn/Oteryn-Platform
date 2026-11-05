CREATE TABLE player_reports (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    reporter_account_id uuid NOT NULL REFERENCES accounts(id),
    request_key uuid NOT NULL,
    report_type text NOT NULL CHECK (report_type IN ('player', 'content', 'guild')),
    category text NOT NULL,
    target_reference text NOT NULL CHECK (length(btrim(target_reference)) BETWEEN 1 AND 160),
    evidence_summary text CHECK (length(evidence_summary) BETWEEN 1 AND 4000),
    status text NOT NULL DEFAULT 'submitted' CHECK (status IN ('submitted', 'reviewing', 'actioned', 'rejected', 'closed')),
    public_outcome text,
    moderator_notes text,
    assigned_to uuid REFERENCES accounts(id),
    lock_version bigint NOT NULL DEFAULT 1 CHECK (lock_version > 0),
    processed_at timestamptz,
    rejected_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (reporter_account_id, request_key),
    CHECK (
        (report_type = 'player' AND category IN ('harassment', 'cheating', 'name', 'conduct', 'other')) OR
        (report_type = 'content' AND category IN ('illegal_content', 'privacy', 'spam', 'misinformation', 'other')) OR
        (report_type = 'guild' AND category IN ('name', 'description', 'harassment', 'impersonation', 'other'))
    ),
    CHECK (status NOT IN ('actioned', 'rejected', 'closed') OR length(btrim(public_outcome)) > 0 AND public_outcome IS NOT NULL)
);
CREATE INDEX player_reports_owner_created ON player_reports (reporter_account_id, created_at DESC, id DESC);
CREATE INDEX player_reports_owner_pending ON player_reports (reporter_account_id) WHERE status IN ('submitted', 'reviewing');

-- Preserve rejection history when a processed report is subsequently closed.
CREATE FUNCTION player_reports_preserve_rejection() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND OLD.rejected_at IS NOT NULL THEN
        NEW.rejected_at := OLD.rejected_at;
    ELSIF NEW.status = 'rejected' THEN
        NEW.rejected_at := now();
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER player_reports_rejection BEFORE INSERT OR UPDATE ON player_reports
    FOR EACH ROW EXECUTE FUNCTION player_reports_preserve_rejection();

-- Persistent, account-scoped submission throttling shared by all portal processes.
CREATE TABLE player_report_submission_limits (
    account_id uuid PRIMARY KEY REFERENCES accounts(id),
    window_started_at timestamptz NOT NULL DEFAULT now(),
    attempts integer NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 3),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
