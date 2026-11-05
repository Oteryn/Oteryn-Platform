-- T34 owns authoritative sanctions; this table owns only the appeal workflow.
CREATE TABLE support_appeal_workflows (
    sanction_id uuid PRIMARY KEY,
    owner_id uuid NOT NULL,
    acknowledged_at timestamptz,
    acknowledged boolean GENERATED ALWAYS AS (acknowledged_at IS NOT NULL) STORED,
    status text NOT NULL DEFAULT 'none' CHECK (status IN ('none', 'requested', 'reviewing', 'accepted', 'rejected')),
    message text CHECK (char_length(message) BETWEEN 1 AND 4000),
    outcome text CHECK (char_length(outcome) BETWEEN 1 AND 4000),
    version bigint NOT NULL DEFAULT 1 CHECK (version > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (status = 'none' OR message IS NOT NULL),
    CHECK (status NOT IN ('accepted', 'rejected') OR outcome IS NOT NULL)
);
CREATE INDEX support_appeal_workflows_owner_history ON support_appeal_workflows (owner_id, created_at DESC, sanction_id DESC);
