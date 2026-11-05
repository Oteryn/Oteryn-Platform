-- Collection assets are private entertainment; no wallet or game rewards.
CREATE TABLE collection_cards (
    id uuid PRIMARY KEY,
    definition jsonb NOT NULL,
    sprite bytea NOT NULL CHECK (octet_length(sprite) BETWEEN 1 AND 1048576),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE collection_decks (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    character_id uuid NOT NULL,
    name text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 40),
    cards uuid[] NOT NULL CHECK (cardinality(cards) BETWEEN 1 AND 3),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX collection_decks_owner ON collection_decks(account_id);
CREATE TABLE collection_battles (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    character_id uuid NOT NULL,
    deck_id uuid REFERENCES collection_decks(id) ON DELETE SET NULL,
    state jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX collection_battles_owner ON collection_battles(account_id);
CREATE UNIQUE INDEX collection_one_active_battle ON collection_battles(account_id)
    WHERE state->>'outcome' = 'active';
