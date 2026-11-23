CREATE TABLE world_incidents (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    world_id uuid NOT NULL REFERENCES world_status(id),
    opened_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    resolved_at timestamptz CHECK (resolved_at >= opened_at),
    summary_en text NOT NULL CHECK (char_length(btrim(summary_en)) BETWEEN 1 AND 240),
    summary_pl text NOT NULL CHECK (char_length(btrim(summary_pl)) BETWEEN 1 AND 240)
);

CREATE UNIQUE INDEX world_incidents_one_open ON world_incidents (world_id)
    WHERE resolved_at IS NULL;
CREATE INDEX world_incidents_history ON world_incidents (world_id, id DESC);
