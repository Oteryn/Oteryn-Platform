CREATE TABLE staff_character_cases (
    id uuid PRIMARY KEY,
    ticket_id uuid NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    account_id uuid NOT NULL REFERENCES accounts(id),
    requester uuid NOT NULL REFERENCES accounts(id),
    reviewer uuid NOT NULL REFERENCES accounts(id),
    world_id uuid NOT NULL,
    operation text NOT NULL CHECK (operation IN ('create', 'rename', 'schedule', 'restore', 'repair')),
    intent jsonb NOT NULL CHECK (jsonb_typeof(intent) = 'object'),
    evidence_ref text NOT NULL CHECK (length(evidence_ref) BETWEEN 1 AND 160),
    revision bigint NOT NULL DEFAULT 1,
    decision text CHECK (decision IN ('accept', 'return')),
    decision_note text CHECK (length(decision_note) BETWEEN 1 AND 500),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    decided_at timestamptz,
    CHECK (requester <> reviewer AND account_id NOT IN (requester, reviewer)),
    CHECK ((revision = 1 AND decision IS NULL AND decision_note IS NULL AND decided_at IS NULL)
        OR (revision = 2 AND decision IS NOT NULL AND decision_note IS NOT NULL AND decided_at IS NOT NULL))
);

CREATE INDEX staff_character_cases_requester ON staff_character_cases(requester, id DESC);
CREATE INDEX staff_character_cases_reviewer ON staff_character_cases(reviewer, id DESC);
CREATE INDEX staff_character_cases_ticket ON staff_character_cases(ticket_id);

-- Extend the installed permission check without changing any existing grant.
DO $$
DECLARE
    current_check text;
BEGIN
    SELECT pg_get_expr(conbin, conrelid)
    INTO STRICT current_check
    FROM pg_constraint
    WHERE conrelid = 'role_permissions'::regclass
      AND conname = 'role_permissions_permission_check';

    ALTER TABLE role_permissions DROP CONSTRAINT role_permissions_permission_check;
    EXECUTE format(
        $check$
        ALTER TABLE role_permissions ADD CONSTRAINT role_permissions_permission_check
        CHECK ((%s) OR permission = 'support.character-cases.review')
        $check$,
        current_check
    );
END $$;
