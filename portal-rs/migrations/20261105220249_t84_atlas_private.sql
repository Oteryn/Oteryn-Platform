CREATE TABLE atlas_private_markers (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    x integer NOT NULL CHECK (x BETWEEN 0 AND 65535),
    y integer NOT NULL CHECK (y BETWEEN 0 AND 65535),
    floor smallint NOT NULL CHECK (floor BETWEEN 0 AND 255),
    label text NOT NULL CHECK (length(label) BETWEEN 1 AND 80),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX atlas_private_markers_owner ON atlas_private_markers(account_id);
-- Narrow T33c handoff: map reports stay local until the support API is available.
CREATE TABLE atlas_map_reports (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    x integer NOT NULL CHECK (x BETWEEN 0 AND 65535),
    y integer NOT NULL CHECK (y BETWEEN 0 AND 65535),
    floor smallint NOT NULL CHECK (floor BETWEEN 0 AND 255),
    description text NOT NULL CHECK (length(description) BETWEEN 10 AND 2000),
    status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'resolved', 'rejected')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX atlas_map_reports_owner_time ON atlas_map_reports(account_id, created_at);
