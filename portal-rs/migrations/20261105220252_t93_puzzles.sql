-- Daily puzzle definitions are immutable after publication. All rewards are local badges.
CREATE TABLE puzzles_daily (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    day date NOT NULL,
    kind text NOT NULL CHECK (kind IN ('monster', 'atlas', 'quiz')),
    world_version uuid NOT NULL REFERENCES world_versions(id),
    definition jsonb NOT NULL,
    image bytea,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (day, kind),
    CHECK ((kind = 'quiz' AND image IS NULL) OR
           (kind IN ('monster', 'atlas') AND octet_length(image) BETWEEN 1 AND 1048576))
);
CREATE TABLE puzzles_answers (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL REFERENCES accounts(id),
    puzzle_id uuid NOT NULL REFERENCES puzzles_daily(id),
    choice smallint NOT NULL CHECK (choice BETWEEN 0 AND 5),
    correct boolean NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (account_id, puzzle_id)
);
CREATE TABLE puzzles_streaks (
    account_id uuid PRIMARY KEY REFERENCES accounts(id),
    last_completed date,
    current_streak integer NOT NULL DEFAULT 0 CHECK (current_streak >= 0),
    best_streak integer NOT NULL DEFAULT 0 CHECK (best_streak >= current_streak),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE FUNCTION puzzles_daily_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'published daily puzzles are immutable';
END;
$$;
CREATE TRIGGER puzzles_daily_immutable BEFORE UPDATE OR DELETE ON puzzles_daily
    FOR EACH ROW EXECUTE FUNCTION puzzles_daily_immutable();
