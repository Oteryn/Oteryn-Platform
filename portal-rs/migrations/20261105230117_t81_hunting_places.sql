-- Private contribution snapshots, never a public session feed.
CREATE TABLE pd_places_preferences (
    account_id uuid PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    participating boolean NOT NULL DEFAULT false,
    privacy_revision bigint NOT NULL DEFAULT 1 CHECK (privacy_revision > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE pd_places_sessions (
    source_authority text NOT NULL,
    session_id uuid NOT NULL,
    account_id uuid NOT NULL REFERENCES pd_places_preferences(account_id) ON DELETE CASCADE,
    privacy_revision bigint NOT NULL CHECK (privacy_revision > 0),
    ended_at timestamptz NOT NULL,
    snapshot jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (source_authority, session_id)
);
CREATE INDEX pd_places_sessions_account ON pd_places_sessions(account_id);
CREATE INDEX pd_places_sessions_end ON pd_places_sessions(ended_at);
-- Digests below are not needed: immutable snapshot equality binds the business key.
-- Exclusion erases contributions but retains payload-free replay fences for 120 days.
CREATE TABLE pd_places_replay_fences (
    source_authority text NOT NULL,
    session_id uuid NOT NULL,
    expires_at timestamptz NOT NULL,
    PRIMARY KEY (source_authority, session_id)
);
