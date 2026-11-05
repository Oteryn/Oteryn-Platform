-- Platform-owned annotations only; gameplay facts remain owned by the journal source.
CREATE TABLE pd_report_notes (
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    target_kind text NOT NULL CHECK (target_kind IN ('session', 'character')),
    target_id uuid NOT NULL,
    body text NOT NULL CHECK (octet_length(body) BETWEEN 1 AND 4000),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, target_kind, target_id)
);
CREATE TABLE pd_report_preferences (
    account_id uuid PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    email_enabled boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE pd_report_deliveries (
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    week_start bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, week_start)
);
