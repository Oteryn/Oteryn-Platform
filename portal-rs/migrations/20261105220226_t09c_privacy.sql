-- Platform privacy ordering is independent of game projection generations.
ALTER TABLE accounts ADD COLUMN privacy_revision bigint NOT NULL DEFAULT 0 CHECK (privacy_revision >= 0);
CREATE TABLE account_privacy_events (
    account_id uuid NOT NULL REFERENCES accounts(id),
    revision bigint NOT NULL CHECK (revision > 0),
    event_type text NOT NULL DEFAULT 'identity.privacy_updated' CHECK (event_type = 'identity.privacy_updated'),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, revision)
);
