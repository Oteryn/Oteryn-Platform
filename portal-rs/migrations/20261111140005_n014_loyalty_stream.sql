-- A transactional head serializes publishers, including their commit order.
-- Sequence allocation alone would allow a late commit behind a returned cursor.
CREATE TABLE loyalty_stream_head (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    position bigint NOT NULL DEFAULT 0 CHECK (position >= 0)
);
INSERT INTO loyalty_stream_head(singleton) VALUES (true);

CREATE TABLE loyalty_stream_state (
    account_id uuid PRIMARY KEY REFERENCES premium_authority(account_id),
    spent_premium_days integer NOT NULL CHECK (spent_premium_days >= 0),
    loyalty_points integer NOT NULL CHECK (loyalty_points >= 0),
    platform_revision bigint NOT NULL CHECK (platform_revision > 0),
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE loyalty_stream_events (
    position bigint PRIMARY KEY CHECK (position > 0),
    account_id uuid NOT NULL REFERENCES premium_authority(account_id),
    spent_premium_days integer NOT NULL CHECK (spent_premium_days >= 0),
    loyalty_points integer NOT NULL CHECK (loyalty_points >= 0),
    platform_revision bigint NOT NULL CHECK (platform_revision > 0),
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (account_id, platform_revision)
);
CREATE TRIGGER loyalty_stream_events_immutable BEFORE UPDATE OR DELETE ON loyalty_stream_events
    FOR EACH ROW EXECUTE FUNCTION premium_immutable();
