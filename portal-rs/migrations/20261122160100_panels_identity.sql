ALTER TABLE accounts ADD COLUMN profile_display_name text NOT NULL DEFAULT ''
    CHECK (char_length(profile_display_name) <= 64);
ALTER TABLE account_mfa_totp ADD COLUMN replacement_secret bytea,
    ADD COLUMN replacement_expires_at timestamptz,
    ADD CONSTRAINT account_mfa_replacement_pair CHECK (
        (replacement_secret IS NULL) = (replacement_expires_at IS NULL)
    );
