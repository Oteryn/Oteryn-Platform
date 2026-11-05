-- Private valuation preferences are not game facts or monetary balances.
CREATE TABLE pd_price_overrides (
    account_id uuid NOT NULL,
    world_id uuid NOT NULL,
    item_key text NOT NULL CHECK (octet_length(item_key) BETWEEN 1 AND 192),
    item_revision text NOT NULL CHECK (octet_length(item_revision) BETWEEN 1 AND 192),
    purpose text NOT NULL CHECK (purpose IN ('loot', 'supplies')),
    selection jsonb NOT NULL CHECK (jsonb_typeof(selection) = 'object'),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, world_id, item_key, item_revision, purpose)
);
