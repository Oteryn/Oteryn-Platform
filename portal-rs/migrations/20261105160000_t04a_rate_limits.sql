-- Shared counters never store raw addresses, identities or credentials.
CREATE TABLE web_rate_limits (
    limiter_name text NOT NULL,
    key_hash bytea NOT NULL CHECK (octet_length(key_hash) = 32),
    window_start timestamptz NOT NULL,
    count bigint NOT NULL CHECK (count > 0),
    PRIMARY KEY (limiter_name, key_hash, window_start)
);
CREATE INDEX web_rate_limits_expiry ON web_rate_limits (window_start);
