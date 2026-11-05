-- Platform preferences only. Game achievement facts remain owner-only.
CREATE TABLE account_badge_selections (
    account_id uuid PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    badge_keys text[] NOT NULL DEFAULT '{}',
    achievement_keys text[] NOT NULL DEFAULT '{}',
    profile_id uuid,
    profile_ownership text,
    loyalty_id uuid,
    loyalty_ownership text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (cardinality(badge_keys) <= 4),
    CHECK (cardinality(achievement_keys) <= 5),
    CHECK ((profile_id IS NULL) = (profile_ownership IS NULL)),
    CHECK ((loyalty_id IS NULL) = (loyalty_ownership IS NULL)),
    CHECK (profile_id IS NOT NULL OR cardinality(achievement_keys) = 0)
);
