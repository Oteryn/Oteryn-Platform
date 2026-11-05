-- Private, promoted G-11 login projections; no bearer or device data.
CREATE TABLE pd_game_logins (
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    login_event_id uuid NOT NULL,
    character_id uuid,
    world_id uuid,
    projection_epoch numeric(20,0) NOT NULL CHECK (projection_epoch > 0),
    recovery_generation numeric(20,0) NOT NULL CHECK (recovery_generation > 0),
    projection_revision numeric(20,0),
    character_revision numeric(20,0),
    source_revision numeric(20,0) NOT NULL CHECK (source_revision > 0),
    entered_at bigint NOT NULL CHECK (entered_at BETWEEN 0 AND 253402300799999),
    left_at bigint CHECK (left_at BETWEEN entered_at AND 253402300799999),
    end_reason text CHECK (end_reason IN ('LOGOUT','DISCONNECT','REVOKED','UNKNOWN')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, login_event_id),
    CHECK ((left_at IS NULL) = (end_reason IS NULL)),
    CHECK ((character_id IS NULL AND world_id IS NULL AND projection_revision IS NULL AND character_revision IS NULL)
        OR (character_id IS NOT NULL AND world_id IS NOT NULL AND projection_revision IS NOT NULL AND character_revision IS NOT NULL AND projection_revision > 0 AND character_revision > 0))
);
CREATE INDEX pd_game_logins_history ON pd_game_logins(account_id, entered_at DESC, login_event_id);
