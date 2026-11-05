CREATE TABLE stats_character_daily (
    character_ref uuid NOT NULL,
    day date NOT NULL,
    level bigint NOT NULL CHECK (level > 0),
    experience numeric(20,0) CHECK (experience >= 0),
    skills jsonb CHECK (jsonb_typeof(skills) = 'object'),
    achievement_points bigint CHECK (achievement_points >= 0),
    minutes_online integer CHECK (minutes_online BETWEEN 0 AND 1440),
    name text NOT NULL,
    world uuid NOT NULL,
    guild uuid,
    observed_at bigint NOT NULL CHECK (observed_at >= 0),
    PRIMARY KEY (character_ref, day)
) PARTITION BY RANGE (day);
CREATE TABLE stats_character_events (
    character_ref uuid NOT NULL,
    day date NOT NULL,
    kind text NOT NULL CHECK (kind IN ('name_change','world_change','guild_join','guild_leave','bazaar_sale','death','level_up')),
    event_key text NOT NULL,
    details jsonb NOT NULL,
    PRIMARY KEY (character_ref, kind, event_key)
);
CREATE INDEX stats_character_events_history ON stats_character_events (character_ref, day);
CREATE TABLE stats_world_samples (
    world uuid NOT NULL,
    sampled_at timestamptz NOT NULL,
    players_online integer NOT NULL CHECK (players_online >= 0),
    PRIMARY KEY (world, sampled_at)
);
CREATE INDEX stats_world_samples_retention ON stats_world_samples (sampled_at);
CREATE TABLE stats_world_daily (
    world uuid NOT NULL,
    day date NOT NULL,
    samples bigint NOT NULL CHECK (samples > 0),
    players_sum bigint NOT NULL,
    players_min integer NOT NULL,
    players_max integer NOT NULL,
    PRIMARY KEY (world, day)
);
-- Per-subject cursors include revision and epoch: equal timestamps can contain new revisions.
CREATE TABLE stats_projection_cursors (
    family text NOT NULL,
    subject text NOT NULL,
    epoch numeric(20,0) NOT NULL,
    version numeric(20,0) NOT NULL,
    watermark bigint NOT NULL,
    PRIMARY KEY (family, subject)
);
