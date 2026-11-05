-- Staff-owned records only; identity and credential state stay in their owning module.
CREATE TABLE player_notes (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL,
    author uuid NOT NULL,
    body text NOT NULL CHECK (length(btrim(body)) BETWEEN 1 AND 4000),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX player_notes_account_created ON player_notes(account_id,created_at DESC,id DESC);
CREATE TABLE player_mfa_reset_requests (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL,
    requested_by uuid NOT NULL CHECK (requested_by <> account_id),
    evidence uuid NOT NULL,
    reason text NOT NULL CHECK (length(btrim(reason)) BETWEEN 1 AND 500),
    approved_by uuid CHECK (approved_by <> requested_by AND approved_by <> account_id),
    expires_at timestamptz NOT NULL DEFAULT now() + interval '30 minutes',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX player_mfa_reset_target ON player_mfa_reset_requests(account_id);
