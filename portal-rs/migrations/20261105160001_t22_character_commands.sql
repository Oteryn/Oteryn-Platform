-- Account identifiers are Platform UUIDs; account authority is supplied by the caller.
CREATE TABLE character_commands (
    id uuid PRIMARY KEY,
    source_revision bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
    issuer_decision_id uuid NOT NULL,
    request_key text NOT NULL UNIQUE,
    command text NOT NULL CHECK (command IN ('CreateCharacter', 'TransferCharacterOwnership')),
    account_id uuid NOT NULL,
    target_account_id uuid,
    binding_json text NOT NULL,
    intent_json text NOT NULL,
    receipt_json text,
    state text NOT NULL DEFAULT 'PENDING' CHECK (state IN ('PENDING', 'TERMINAL')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK ((state = 'PENDING') = (receipt_json IS NULL))
);
CREATE INDEX character_commands_pending ON character_commands (source_revision, id) WHERE state = 'PENDING';
