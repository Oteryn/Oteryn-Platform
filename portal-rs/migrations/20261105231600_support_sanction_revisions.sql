-- Separate from session generations and legacy sanctions.lock_version.
CREATE TABLE support_sanction_counters (
    account_id uuid PRIMARY KEY REFERENCES accounts(id),
    revision bigint NOT NULL DEFAULT 0 CHECK (revision BETWEEN 0 AND 9007199254740991)
);
CREATE TABLE support_sanction_identities (
    sanction_id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id),
    kind text NOT NULL CHECK (kind IN ('ban','namelock','warning','notation')),
    character_id uuid,
    CHECK (kind <> 'namelock' OR character_id IS NOT NULL),
    CHECK (kind NOT IN ('ban','warning') OR character_id IS NULL),
    UNIQUE (sanction_id, account_id)
);
CREATE TABLE support_sanction_revisions (
    account_id uuid NOT NULL REFERENCES accounts(id),
    revision bigint NOT NULL CHECK (revision BETWEEN 1 AND 9007199254740991),
    sanction_id uuid NOT NULL,
    intent jsonb NOT NULL,
    active boolean NOT NULL,
    decision_at timestamptz NOT NULL,
    expires_at timestamptz,
    PRIMARY KEY (account_id, revision),
    FOREIGN KEY (sanction_id, account_id) REFERENCES support_sanction_identities(sanction_id, account_id),
    CHECK (expires_at IS NULL OR expires_at > decision_at)
);
CREATE INDEX support_sanction_latest ON support_sanction_revisions(account_id, sanction_id, revision DESC);
