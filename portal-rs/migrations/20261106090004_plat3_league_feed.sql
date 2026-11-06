-- Additive follow-up: do not alter the already published T94 migration checksum.
CREATE TABLE league_feed_state (
    authority text PRIMARY KEY,
    epoch numeric(20,0) NOT NULL CHECK(epoch>0),
    recovery_generation numeric(20,0) NOT NULL CHECK(recovery_generation>0),
    ready boolean NOT NULL DEFAULT false,
    conflicted boolean NOT NULL DEFAULT false,
    event_complete bigint NOT NULL DEFAULT 0,
    season_complete bigint NOT NULL DEFAULT 0,
    manifest bytea,
    manifest_signature bytea,
    manifest_meta bytea
);
CREATE TABLE league_feed_inbox (
    sequence bigserial UNIQUE NOT NULL,
    request_id uuid PRIMARY KEY,
    path text NOT NULL,
    body bytea NOT NULL,
    meta bytea NOT NULL,
    signature bytea NOT NULL,
    fingerprint text NOT NULL,
    peer_certificate_sha256 text NOT NULL,
    received_at bigint NOT NULL,
    state text NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','done','refused')),
    UNIQUE(path,fingerprint)
);
CREATE INDEX league_feed_pending ON league_feed_inbox(sequence) WHERE state='pending';
CREATE TABLE league_feed_withdrawals (
    authority text NOT NULL, family text NOT NULL, world_id uuid NOT NULL,subject_id uuid NOT NULL,
    epoch numeric(20,0) NOT NULL, revision numeric(20,0) NOT NULL,
    withdrawn boolean NOT NULL, record_hash text NOT NULL,
    PRIMARY KEY(authority,family,world_id,subject_id)
);
CREATE TABLE league_feed_requests (
    request_id uuid PRIMARY KEY,
    fingerprint text NOT NULL,
    state text NOT NULL CHECK(state IN ('pending','done','refused','gone'))
);
ALTER TABLE league_feed_inbox ADD COLUMN expires_at bigint NOT NULL;
-- Minimal immutable question identity, including questions that never settle.
CREATE TABLE league_event_questions (
    authority text NOT NULL, world_id uuid NOT NULL, subject_id uuid NOT NULL,
    question_hash text NOT NULL,
    last_nonterminal text NOT NULL DEFAULT '' CHECK(last_nonterminal IN ('','SCHEDULED','LOCKED')),
    PRIMARY KEY(authority,world_id,subject_id)
);
