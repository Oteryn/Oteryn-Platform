-- Independent service revisions under the shared account projection epoch fence.
ALTER TABLE character_commands DROP CONSTRAINT character_commands_command_check;
ALTER TABLE character_commands ADD CONSTRAINT character_commands_command_check CHECK
(command IN ('CreateCharacter','TransferCharacterOwnership','ScheduleCharacterDeletion','CancelCharacterDeletion','RenameCharacter'));
CREATE TABLE character_service_feed (
    id smallint PRIMARY KEY CHECK (id=1),
    epoch numeric(20,0), complete_through bigint, observed_at bigint,
    invalid boolean NOT NULL DEFAULT false,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (complete_through<=observed_at)
);
INSERT INTO character_service_feed(id) VALUES (1);
CREATE TABLE character_service_states (
    account_id uuid NOT NULL, character_id uuid NOT NULL,
    epoch numeric(20,0) NOT NULL, revision numeric(20,0) NOT NULL,
    character_revision text NOT NULL,
    snapshot_json text NOT NULL,
    invalid boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(account_id,character_id)
);
CREATE TABLE character_service_orders (
    operation_id uuid PRIMARY KEY REFERENCES character_commands(id),
    account_id uuid NOT NULL, character_id uuid NOT NULL,
    request_id uuid NOT NULL UNIQUE,
    action_json text NOT NULL,
    product_id text, product_version integer, amount bigint,
    reservation_id uuid, settlement_id uuid NOT NULL UNIQUE,
    financial_state text NOT NULL CHECK (financial_state IN ('FREE','RESERVED','SETTLED','RELEASED','RECOVERY_REQUIRED')),
    credit_from uuid REFERENCES character_service_orders(operation_id),
    credit_used_by uuid REFERENCES character_service_orders(operation_id),
    intent_digest bytea NOT NULL CHECK (octet_length(intent_digest)=32),
    created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK ((reservation_id IS NULL AND amount IS NULL) OR (reservation_id IS NOT NULL AND amount>0 AND product_id IS NOT NULL AND product_version>0))
);
CREATE INDEX character_service_subject ON character_service_orders(character_id);
