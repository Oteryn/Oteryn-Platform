-- T72, journal contracts revision 3 (G-10, G-11 §8.2): content-free deletion
-- tombstones. Account deletion, character deletion and character transfer close
-- their scope; a replay or first delivery inside a closed scope is refused and
-- never reinserted. Only the SHA-256 of the canonical scope key is kept, for
-- 120 days from the later of deletion and purge. Additive: no existing table
-- or row changes.
CREATE TABLE pd_deleted_scopes (
    scope_digest bytea PRIMARY KEY CHECK (octet_length(scope_digest) = 32),
    deleted_ms bigint NOT NULL,
    expires_ms bigint NOT NULL,
    CHECK (deleted_ms <= expires_ms)
);
CREATE INDEX pd_deleted_scopes_expiry ON pd_deleted_scopes(expires_ms);
