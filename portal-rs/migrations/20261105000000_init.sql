-- Baseline for the Oteryn portal schema. Module tables arrive in their own migrations.
CREATE TABLE portal_schema_baseline (
    id smallint PRIMARY KEY CHECK (id = 1),
    created_at timestamptz NOT NULL DEFAULT now()
);
INSERT INTO portal_schema_baseline (id) VALUES (1);
