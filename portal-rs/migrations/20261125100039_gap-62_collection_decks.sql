CREATE TABLE collection_pvp_owned_decks (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 40),
    cards uuid[] NOT NULL CHECK (cardinality(cards) BETWEEN 1 AND 3)
);

CREATE INDEX collection_pvp_owned_decks_account ON collection_pvp_owned_decks(account_id);
CREATE INDEX collection_pvp_purchases_account_set ON collection_pvp_purchases(account_id, set_id);
