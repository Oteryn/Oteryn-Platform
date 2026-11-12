-- Gateway login contract §19.3: one positive native play generation per account,
-- advanced by every committed fresh attempt while protocol 3 is configured. A
-- recovery handle is usable only while it carries the newest generation.
CREATE TABLE native_play_generations (
    account_id uuid PRIMARY KEY,
    generation bigint NOT NULL CHECK (generation > 0),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
