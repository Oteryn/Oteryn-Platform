ALTER TABLE accounts ADD COLUMN email_change_blocked_until timestamptz NOT NULL DEFAULT '-infinity';
CREATE TABLE account_email_changes (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL REFERENCES accounts(id),
    kind text NOT NULL CHECK (kind IN ('primary','backup')),
    old_email text NOT NULL,
    new_email text NOT NULL CHECK (length(new_email) BETWEEN 3 AND 254),
    confirmation_hash bytea UNIQUE NOT NULL CHECK (octet_length(confirmation_hash)=32),
    revert_hash bytea UNIQUE NOT NULL CHECK (octet_length(revert_hash)=32),
    immediate boolean NOT NULL,
    confirmed_at timestamptz,
    apply_after timestamptz NOT NULL,
    expires_at timestamptz NOT NULL,
    revert_until timestamptz NOT NULL,
    applied_at timestamptz,
    cancelled_at timestamptz,
    reverted_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (old_email <> new_email)
);
CREATE UNIQUE INDEX account_email_change_pending ON account_email_changes(account_id,kind)
    WHERE applied_at IS NULL AND cancelled_at IS NULL AND reverted_at IS NULL;
CREATE TABLE account_backup_emails (
    account_id uuid PRIMARY KEY REFERENCES accounts(id),
    email text NOT NULL,
    verified_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
-- Delivery addresses are frozen at request time: the old address must remain reachable.
CREATE TABLE account_email_change_mail (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    change_id uuid NOT NULL REFERENCES account_email_changes(id),
    recipient text NOT NULL,
    subject text NOT NULL,
    body text NOT NULL,
    delivered_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
