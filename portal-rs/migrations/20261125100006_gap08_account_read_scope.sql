-- Add the Platform account-read grant without changing existing grant rows.
ALTER TABLE oauth_authorization_codes
    DROP CONSTRAINT oauth_authorization_codes_scope_check,
    ADD CONSTRAINT oauth_authorization_codes_scope_check
        CHECK (scope IN ('game:ticket', 'game:account-services', 'account:read'));

ALTER TABLE oauth_token_families
    DROP CONSTRAINT oauth_token_families_scope_check,
    ADD CONSTRAINT oauth_token_families_scope_check
        CHECK (scope IN ('game:ticket', 'game:account-services', 'account:read'));
