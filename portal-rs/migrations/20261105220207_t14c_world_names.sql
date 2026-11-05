-- Presentation metadata never changes immutable gameplay definitions.
CREATE TABLE world_admin_names (
    kind text NOT NULL CHECK (octet_length(kind) BETWEEN 1 AND 512),
    key text NOT NULL CHECK (octet_length(key) BETWEEN 1 AND 512),
    pl text NOT NULL CHECK (octet_length(pl) BETWEEN 1 AND 512),
    en text NOT NULL CHECK (octet_length(en) BETWEEN 1 AND 512),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (kind, key)
);
