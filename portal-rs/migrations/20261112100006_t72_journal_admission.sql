-- T72 journal admission (G-10 items rev 3, G-11 progress rev 4). Additive only.
--
-- Initial ownership without a transfer receipt needs an accepted baseline that
-- proves the equivalent receipt cut (G-10 *Types and common envelope*). No route
-- writes this table: until the baseline exchange is accepted (G-10 open question
-- 6) only a trusted adapter may insert a row, and a character without one and
-- without a committed transfer receipt stays unavailable.
CREATE TABLE pd_ownership_baselines (
    character_id uuid PRIMARY KEY,
    account_id uuid NOT NULL,
    world_id uuid NOT NULL,
    character_revision numeric(20,0) NOT NULL CHECK (character_revision > 0),
    from_projection_revision numeric(20,0) NOT NULL CHECK (from_projection_revision > 0),
    to_projection_revision numeric(20,0) NOT NULL CHECK (to_projection_revision > 0),
    projection_epoch numeric(20,0) NOT NULL CHECK (projection_epoch > 0),
    recovery_generation numeric(20,0) NOT NULL CHECK (recovery_generation > 0),
    accepted_at timestamptz NOT NULL DEFAULT now()
);
-- The admission lease reads ownership by character: the projection row and the
-- transfer commands of one character, each through an index.
CREATE INDEX account_characters_character ON account_characters(character_id);
CREATE INDEX character_commands_transfer_character
    ON character_commands ((binding_json::jsonb->>'character_id'))
    WHERE command = 'TransferCharacterOwnership';
-- The restore generation of the last accepted G-11 watermark, the account-level
-- recovery fence of the login consumer. NULL until a watermark is accepted.
ALTER TABLE pd_streams ADD COLUMN watermark_recovery numeric(20,0);
