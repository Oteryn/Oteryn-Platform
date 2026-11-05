-- Additive, inert ledger. No Game schema or local gameplay effects.
-- The singleton lock is held through commit for issuer-wide revision ordering.
CREATE TABLE hireling_command_sequence (
 singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
 revision numeric(20,0) NOT NULL DEFAULT 0 CHECK (revision BETWEEN 0 AND 18446744073709551615)
);
INSERT INTO hireling_command_sequence(singleton) VALUES(true);
-- Retained while unresolved. Expiry never proves a terminal rejection.
CREATE TABLE hireling_command_intents (
 operation_id uuid PRIMARY KEY CHECK (substring(operation_id::text,15,1)='7'),
 account_id uuid NOT NULL,
 idempotency_key uuid NOT NULL CHECK (substring(idempotency_key::text,15,1)='7'),
 hireling_id uuid NOT NULL,
 source_authority text NOT NULL CHECK (source_authority ~ '^[A-Za-z0-9._:/-]{1,128}$'),
 source_revision numeric(20,0) NOT NULL UNIQUE CHECK (source_revision BETWEEN 1 AND 18446744073709551615),
 action_bytes text NOT NULL CHECK (octet_length(action_bytes) BETWEEN 1 AND 4096),
 intent_bytes text NOT NULL CHECK (octet_length(intent_bytes) BETWEEN 1 AND 4096),
 binding_digest text NOT NULL CHECK (binding_digest ~ '^[0-9a-f]{64}$'),
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(account_id,idempotency_key)
);
CREATE INDEX hireling_command_intents_account_time ON hireling_command_intents(account_id,created_at);
CREATE INDEX hireling_command_intents_source ON hireling_command_intents(source_authority);
CREATE FUNCTION hireling_command_intent_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 RAISE EXCEPTION 'hireling command intents are immutable';
END;
$$;
CREATE TRIGGER hireling_command_intent_immutable BEFORE UPDATE OR DELETE ON hireling_command_intents
 FOR EACH ROW EXECUTE FUNCTION hireling_command_intent_immutable();
