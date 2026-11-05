CREATE TABLE stats_watches (
    account_id uuid NOT NULL,
    target_kind text NOT NULL CHECK (target_kind IN ('character', 'guild')),
    target_id uuid NOT NULL,
    event_kind text NOT NULL CHECK (event_kind IN ('online', 'level_up', 'death', 'guild_change')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, target_kind, target_id, event_kind)
);
CREATE INDEX stats_watches_fanout ON stats_watches (event_kind, target_kind, target_id);
CREATE TABLE stats_watch_deliveries (
    account_id uuid NOT NULL,
    event_id uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, event_id)
);
CREATE TABLE stats_watch_budgets (
    account_id uuid PRIMARY KEY,
    window_start timestamptz NOT NULL DEFAULT now(),
    requests integer NOT NULL CHECK (requests > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
