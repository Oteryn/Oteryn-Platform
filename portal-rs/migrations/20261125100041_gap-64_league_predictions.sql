-- The active cursor references an already admitted publication. Historical
-- cursors remain readable without a reference and open no prediction forms.
ALTER TABLE league_feed_withdrawals ADD COLUMN prediction_publication uuid;
CREATE INDEX league_feed_prediction_publications
    ON league_feed_withdrawals(authority, epoch, recovery_generation, world_id, subject_id)
    WHERE prediction_publication IS NOT NULL;

CREATE TABLE league_predictions (
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    authority text NOT NULL,
    world_id uuid NOT NULL,
    event_id uuid NOT NULL,
    choice text NOT NULL CHECK (choice ~ '^[a-z0-9][a-z0-9._:/-]{0,127}$'),
    expires_at bigint NOT NULL,
    PRIMARY KEY (account_id, authority, world_id, event_id)
);

CREATE INDEX league_predictions_expiry ON league_predictions(expires_at);
