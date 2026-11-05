-- Disposable T73 projections. T72 owns canonical bytes, stream watermarks,
-- rare-drop reconciliation and the authoritative privacy/transfer fences.
CREATE TABLE pd_hunt_sessions (
    account_id uuid NOT NULL,
    session_id uuid NOT NULL,
    stream_generation uuid NOT NULL,
    character_revision numeric(20,0) NOT NULL CHECK (character_revision > 0),
    character_id uuid NOT NULL,
    started_at bigint NOT NULL CHECK (started_at > 0),
    ended_at bigint NOT NULL CHECK (ended_at >= started_at),
    expires_at bigint NOT NULL CHECK (expires_at > ended_at AND expires_at <= ended_at + 7776000),
    has_party boolean NOT NULL,
    facts jsonb NOT NULL CHECK (jsonb_typeof(facts) = 'object'),
    PRIMARY KEY (account_id, session_id)
);
CREATE INDEX pd_hunt_sessions_owner_date ON pd_hunt_sessions (account_id, ended_at DESC, session_id);
-- No row or a false decision always denies. T72 must update/revoke these
-- decisions transactionally with privacy, transfer and restore reconciliation.
CREATE TABLE pd_hunt_access (
    account_id uuid NOT NULL,
    session_id uuid NOT NULL,
    stream_generation uuid NOT NULL,
    character_revision numeric(20,0) NOT NULL CHECK (character_revision > 0),
    complete boolean NOT NULL DEFAULT false,
    allowed boolean NOT NULL DEFAULT false,
    PRIMARY KEY (account_id, session_id),
    FOREIGN KEY (account_id, session_id) REFERENCES pd_hunt_sessions (account_id, session_id) ON DELETE CASCADE
);
