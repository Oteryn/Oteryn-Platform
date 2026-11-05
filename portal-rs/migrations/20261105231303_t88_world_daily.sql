-- Disposable public content; permanent identity fences survive payload expiry.
CREATE TABLE today_snapshots (
    authority text NOT NULL,
    world_id uuid NOT NULL REFERENCES world_status(id),
    epoch numeric(20,0) NOT NULL CHECK (epoch > 0),
    revision numeric(20,0) NOT NULL CHECK (revision > 0),
    observed bigint NOT NULL CHECK (observed >= 0),
    content text,
    fingerprint bytea NOT NULL CHECK (octet_length(fingerprint) = 32),
    conflicted boolean NOT NULL DEFAULT false,
    received_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (authority, world_id)
);
CREATE TABLE today_watermarks (
    authority text PRIMARY KEY,
    epoch numeric(20,0) NOT NULL CHECK (epoch > 0),
    complete_through bigint NOT NULL CHECK (complete_through >= 0),
    observed bigint NOT NULL CHECK (observed >= complete_through)
);
