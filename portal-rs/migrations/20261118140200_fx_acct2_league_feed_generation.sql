-- Existing cursors can recover their generation from an exact subject match.
-- Keep unbound historical cursors nullable until the producer publishes again.
ALTER TABLE league_feed_withdrawals
    ADD COLUMN recovery_generation numeric(20,0),
    ADD CONSTRAINT league_feed_withdrawals_generation_positive
        CHECK(recovery_generation>0) NOT VALID;
