-- Registration remains unavailable until an operator enables it and provisions fresh ranges.
CREATE TABLE registration_abuse_policy (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    enabled boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
INSERT INTO registration_abuse_policy (singleton) VALUES (true);
-- A range is authoritative only when its complete contents were imported atomically.
CREATE TABLE registration_password_ranges (
    prefix text PRIMARY KEY CHECK (prefix ~ '^[0-9A-F]{5}$'),
    suffixes text[] NOT NULL,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE registration_work (
    token_hash bytea PRIMARY KEY CHECK (octet_length(token_hash) = 32),
    form_hash bytea NOT NULL CHECK (octet_length(form_hash) = 32),
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX registration_work_expiry ON registration_work (expires_at);
CREATE INDEX registration_work_form ON registration_work (form_hash);
