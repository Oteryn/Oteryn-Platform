-- Public registry metadata only. No routes, runtime policy or production worlds.
CREATE TABLE world_status (
    id uuid PRIMARY KEY CHECK (id::text ~ '^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$'),
    name text NOT NULL CHECK (length(btrim(name)) BETWEEN 1 AND 96),
    pvp_type text CHECK (length(btrim(pvp_type)) BETWEEN 1 AND 96),
    location text CHECK (length(btrim(location)) BETWEEN 1 AND 96),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
