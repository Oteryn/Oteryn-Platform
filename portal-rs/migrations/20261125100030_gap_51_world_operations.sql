-- Platform planning only. These grants confer no Game authority.
CREATE TABLE world_operation_planning_grants (
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    world_id uuid NOT NULL REFERENCES world_status(id) ON DELETE CASCADE,
    action text NOT NULL CHECK (action IN (
        'enter-maintenance', 'exit-maintenance', 'drain', 'restart',
        'event-start', 'event-stop', 'event-cancel', 'broadcast'
    )),
    can_review boolean NOT NULL DEFAULT false,
    PRIMARY KEY (account_id, world_id, action)
);

CREATE TABLE world_operation_plans (
    id uuid PRIMARY KEY,
    world_id uuid NOT NULL,
    action text NOT NULL CHECK (action IN (
        'enter-maintenance', 'exit-maintenance', 'drain', 'restart',
        'event-start', 'event-stop', 'event-cancel', 'broadcast'
    )),
    requester uuid NOT NULL,
    starts bigint NOT NULL,
    ends bigint NOT NULL,
    ticket text NOT NULL CHECK (octet_length(ticket) BETWEEN 1 AND 128),
    target uuid,
    event uuid,
    locale text CHECK (locale IN ('en', 'pl')),
    message text CHECK (char_length(message) BETWEEN 1 AND 500),
    reviewer uuid,
    approved boolean,
    reviewed_at bigint,
    CHECK (ends > starts AND ends - starts <= 86400),
    CHECK ((reviewer IS NULL) = (approved IS NULL)),
    CHECK ((reviewer IS NULL) = (reviewed_at IS NULL)),
    CHECK (reviewer <> requester AND reviewed_at < starts),
    CHECK (action <> 'restart' OR target IS NOT NULL),
    CHECK (target IS NULL OR action IN ('event-start', 'event-stop', 'event-cancel', 'broadcast', 'drain', 'restart')),
    CHECK ((action IN ('event-start', 'event-stop', 'event-cancel')) = (event IS NOT NULL)),
    CHECK ((action = 'broadcast') = (locale IS NOT NULL)),
    CHECK ((action = 'broadcast') = (message IS NOT NULL))
);

CREATE INDEX world_operation_plans_scope_idx ON world_operation_plans (world_id, action, id DESC);
