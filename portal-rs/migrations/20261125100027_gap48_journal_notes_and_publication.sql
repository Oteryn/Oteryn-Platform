CREATE TABLE pd_weekly_notes (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    week_start bigint NOT NULL CHECK (week_start >= 345600 AND (week_start - 345600) % 604800 = 0),
    body text NOT NULL CHECK (octet_length(body) BETWEEN 1 AND 4000),
    UNIQUE (account_id, week_start)
);

CREATE TABLE pd_places_publication_days (
    utc_day date PRIMARY KEY,
    digest bytea NOT NULL CHECK (octet_length(digest) = 32),
    withdrawn boolean NOT NULL DEFAULT false
);
