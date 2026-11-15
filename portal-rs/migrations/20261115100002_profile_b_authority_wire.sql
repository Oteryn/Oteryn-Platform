-- Profile-B entitlement authority wire (oteryn-v2-profile-b-authority-wire-v1, §8, §10).
-- Additive only. The B1 authority row is separate from premium_authority (§10.2);
-- account ids have no foreign key, as for premium_authority.
CREATE TABLE profile_b_authority (
    account_id uuid PRIMARY KEY,
    authority_revision bigint NOT NULL DEFAULT 0 CHECK (authority_revision BETWEEN 0 AND 9007199254740991),
    last_issued_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- B2 journal (§8.4, §1.6): positions are allocated under one locked counter row
-- in the transaction of the lifecycle change, so a larger position never
-- becomes visible before a smaller one commits. History is never pruned.
CREATE TABLE profile_b_change_head (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    position bigint NOT NULL DEFAULT 0 CHECK (position >= 0)
);
INSERT INTO profile_b_change_head DEFAULT VALUES;

CREATE TABLE profile_b_changes (
    position bigint PRIMARY KEY CHECK (position >= 1),
    account_id uuid NOT NULL,
    product_id text NOT NULL CHECK (octet_length(product_id) BETWEEN 1 AND 64
        AND product_id ~ '^[a-z][a-z0-9_]{0,31}(\.[a-z][a-z0-9_]{0,31}){1,3}$'),
    product_version bigint NOT NULL CHECK (product_version BETWEEN 1 AND 9007199254740991),
    lifecycle_revision bigint NOT NULL CHECK (lifecycle_revision BETWEEN 1 AND 9007199254740991),
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- One hint per lifecycle revision; also the B1 fence lookup index.
    UNIQUE (account_id, product_id, product_version, lifecycle_revision)
);

CREATE FUNCTION profile_b_changes_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'profile-b change history is immutable';
END;
$$;
CREATE TRIGGER profile_b_changes_immutable BEFORE UPDATE OR DELETE ON profile_b_changes
    FOR EACH ROW EXECUTE FUNCTION profile_b_changes_immutable();

-- Owner decision P4: B2 also carries oteryn.premium_time v1. Every Premium
-- lifecycle change is one premium_ledger row; publish its hint in the same
-- transaction. Rows written before this migration publish nothing.
CREATE FUNCTION profile_b_publish_premium() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    next_position bigint;
BEGIN
    UPDATE profile_b_change_head SET position = position + 1 WHERE singleton
        RETURNING position INTO next_position;
    INSERT INTO profile_b_changes(position, account_id, product_id, product_version, lifecycle_revision)
        VALUES (next_position, NEW.account_id, 'oteryn.premium_time', 1, NEW.lifecycle_revision);
    RETURN NULL;
END;
$$;
CREATE TRIGGER premium_ledger_profile_b_publish AFTER INSERT ON premium_ledger
    FOR EACH ROW EXECUTE FUNCTION profile_b_publish_premium();
