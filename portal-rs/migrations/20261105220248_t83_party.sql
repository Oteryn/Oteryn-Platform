-- Portal-owned recruitment board; character identity is supplied by the ownership adapter.
CREATE TABLE pd_party_posts (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL,
    character_id uuid NOT NULL,
    character_name text NOT NULL CHECK (char_length(character_name) BETWEEN 1 AND 64),
    world_id uuid NOT NULL,
    purpose text NOT NULL CHECK (purpose IN ('hunt', 'boss', 'quest')),
    min_level integer NOT NULL CHECK (min_level BETWEEN 1 AND 100000),
    max_level integer NOT NULL CHECK (max_level BETWEEN min_level AND 100000),
    profession text NOT NULL CHECK (profession IN ('any','knight','paladin','sorcerer','druid','monk')),
    note text NOT NULL CHECK (char_length(note) <= 500),
    starts_at timestamptz NOT NULL,
    expires_at timestamptz NOT NULL,
    status text NOT NULL DEFAULT 'active' CHECK (status IN ('active','removed','expired')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (expires_at > starts_at),
    CHECK (expires_at <= created_at + interval '7 days')
);
CREATE INDEX pd_party_active_idx ON pd_party_posts(world_id, expires_at) WHERE status = 'active';
CREATE TABLE pd_party_reports (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    post_id uuid NOT NULL REFERENCES pd_party_posts(id),
    account_id uuid NOT NULL,
    reason text NOT NULL CHECK (char_length(reason) BETWEEN 1 AND 500),
    status text NOT NULL DEFAULT 'open' CHECK (status IN ('open','resolved')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(post_id, account_id)
);
