CREATE TABLE admin_incident_cases (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    investigator uuid NOT NULL REFERENCES accounts(id),
    world_id text NOT NULL CHECK (world_id ~ '^[A-Za-z0-9_.-]{1,128}$'),
    node_id text CHECK (node_id ~ '^[A-Za-z0-9_.-]{1,128}$'),
    character_id text CHECK (character_id ~ '^[A-Za-z0-9_.-]{1,128}$'),
    account_ref uuid CHECK (account_ref <> '00000000-0000-0000-0000-000000000000'),
    -- Case creation checks the alarm against the append-only audit log.
    alarm_id uuid UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX admin_incident_cases_investigator_idx
    ON admin_incident_cases (investigator, id DESC);

CREATE INDEX admin_control_requests_incident_ref_idx
    ON admin_control_requests ((reason->>'ticket_ref'))
    WHERE kind = 'command' AND reason->>'ticket_ref' IS NOT NULL;

CREATE INDEX audit_log_incident_assignments_idx
    ON audit_log (target, created_at)
    WHERE action IN ('incident.created', 'incident.assigned');
