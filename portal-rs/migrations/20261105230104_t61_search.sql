-- Preferences store criteria and references only, never copies of public or private snapshots.
CREATE TABLE bazaar_watches (
    account uuid NOT NULL CHECK (account <> '00000000-0000-0000-0000-000000000000'),
    listing_id uuid NOT NULL REFERENCES bazaar_listings(id),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY(account, listing_id)
);
CREATE TABLE bazaar_saved_searches (
    account uuid NOT NULL CHECK (account <> '00000000-0000-0000-0000-000000000000'),
    name text NOT NULL CHECK (length(name) BETWEEN 1 AND 80),
    query text NOT NULL CHECK (octet_length(query) <= 16384),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY(account, name)
);
