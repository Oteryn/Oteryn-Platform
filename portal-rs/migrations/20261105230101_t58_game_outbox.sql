-- Staged G-7 intents are private, immutable and deliberately not dispatchable.
CREATE TABLE bazaar_game_issuer (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    source_revision numeric(20,0) NOT NULL DEFAULT 0 CHECK (source_revision BETWEEN 0 AND 18446744073709551615),
    projection_epoch numeric(20,0) NOT NULL DEFAULT 0 CHECK (projection_epoch BETWEEN 0 AND 18446744073709551615),
    recovery_generation numeric(20,0) NOT NULL DEFAULT 0 CHECK (recovery_generation BETWEEN 0 AND 18446744073709551615)
);
INSERT INTO bazaar_game_issuer (singleton) VALUES (true);
CREATE TABLE bazaar_game_outbox (
    operation_id uuid PRIMARY KEY,
    issuance_key text NOT NULL UNIQUE CHECK (length(issuance_key) BETWEEN 1 AND 191),
    listing_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    command text NOT NULL CHECK (command IN ('LockBazaarCharacter','BindBazaarCharacterTransfer','UnlockBazaarCharacter','EscrowBazaarItem','DeliverBazaarItem','ReturnBazaarItem','HoldBazaarGold','ReleaseBazaarGold','PayoutBazaarGold')),
    source_revision numeric(20,0) NOT NULL UNIQUE CHECK (source_revision BETWEEN 1 AND 18446744073709551615),
    intent text NOT NULL CHECK (octet_length(intent) BETWEEN 1 AND 4096),
    binding_digest text NOT NULL CHECK (binding_digest ~ '^[0-9a-f]{64}$'),
    state text NOT NULL DEFAULT 'STAGED' CHECK (state IN ('STAGED','REOPENED')),
    projection_epoch numeric(20,0) NOT NULL CHECK (projection_epoch BETWEEN 1 AND 18446744073709551615),
    recovery_generation numeric(20,0) NOT NULL CHECK (recovery_generation BETWEEN 1 AND 18446744073709551615),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX bazaar_game_outbox_listing ON bazaar_game_outbox (listing_id);
-- No receipt/facts schema is invented before the CHAR_CMD version is accepted.
CREATE TABLE bazaar_game_projections (
    source_authority text NOT NULL,
    kind text NOT NULL CHECK (kind IN ('POLICY','STATUS','WATERMARK')),
    subject uuid NOT NULL,
    projection_epoch numeric(20,0) NOT NULL CHECK (projection_epoch BETWEEN 1 AND 18446744073709551615),
    projection_revision numeric(20,0) NOT NULL CHECK (projection_revision BETWEEN 1 AND 18446744073709551615),
    content_digest text NOT NULL CHECK (content_digest ~ '^[0-9a-f]{64}$'),
    canonical_body text NOT NULL CHECK (octet_length(canonical_body) BETWEEN 1 AND 262144),
    source_observed_at numeric(20,0) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (source_authority,kind,subject)
);
CREATE TABLE bazaar_game_quarantine (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    source_authority text NOT NULL,
    kind text NOT NULL,
    subject uuid NOT NULL,
    incoming_digest text NOT NULL CHECK (incoming_digest ~ '^[0-9a-f]{64}$'),
    created_at timestamptz NOT NULL DEFAULT now()
);
