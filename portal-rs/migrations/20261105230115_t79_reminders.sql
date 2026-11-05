-- Private reminder intents contain no addresses, credentials or gameplay payloads.
CREATE TABLE pd_reminders (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL,
    character_id uuid,
    kind text NOT NULL CHECK (kind IN ('rent','premium','boss','stamina','bazaar')),
    entity_key text NOT NULL CHECK (length(entity_key) BETWEEN 1 AND 512),
    channel text NOT NULL CHECK (channel IN ('email','web_push')),
    due_at_ms bigint NOT NULL CHECK (due_at_ms >= 0),
    cut jsonb,
    state text NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','delivered','cancelled')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (account_id, kind, entity_key, channel, due_at_ms),
    CHECK ((character_id IS NULL) = (cut IS NULL))
);
CREATE INDEX pd_reminders_pending ON pd_reminders (due_at_ms) WHERE state = 'pending';
