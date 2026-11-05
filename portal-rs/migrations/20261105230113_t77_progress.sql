-- Private projections only. No game state or gameplay effects are mutated.
CREATE TABLE pd_progress_streams (
    account_id uuid NOT NULL,
    family text NOT NULL,
    authority text NOT NULL,
    epoch text NOT NULL,
    recovery text NOT NULL,
    complete_through numeric(20,0) NOT NULL DEFAULT 0,
    baseline_complete boolean NOT NULL DEFAULT false,
    observed_at bigint NOT NULL DEFAULT 0,
    PRIMARY KEY (account_id, family)
);
CREATE TABLE pd_progress_records (
    account_id uuid NOT NULL,
    family text NOT NULL,
    source_revision numeric(20,0) NOT NULL CHECK (source_revision > 0),
    record_id uuid NOT NULL UNIQUE,
    character_id uuid,
    entity_key text NOT NULL,
    canonical bytea NOT NULL,
    occurred_at bigint NOT NULL,
    PRIMARY KEY (account_id, family, source_revision),
    FOREIGN KEY (account_id, family) REFERENCES pd_progress_streams ON DELETE CASCADE
);
CREATE TABLE pd_progress_goals (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL,
    character_id uuid NOT NULL,
    ownership_cut text NOT NULL,
    kind text NOT NULL CHECK (kind IN ('level','skill','bestiary')),
    entity_key text NOT NULL,
    target bigint NOT NULL CHECK (target > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
