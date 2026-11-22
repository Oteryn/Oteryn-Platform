-- A cancel-safe, expiring preview of the account privacy locale.
ALTER TABLE accounts
    ADD COLUMN synthetic_privacy_locale text,
    ADD COLUMN synthetic_privacy_locale_expires_at timestamptz,
    ADD COLUMN synthetic_privacy_locale_token uuid,
    ADD CONSTRAINT accounts_synthetic_privacy_locale_coherent CHECK (
        (synthetic_privacy_locale IS NULL AND synthetic_privacy_locale_expires_at IS NULL
            AND synthetic_privacy_locale_token IS NULL)
        OR
        (synthetic_privacy_locale IS NOT NULL AND synthetic_privacy_locale IN ('pl', 'en')
            AND synthetic_privacy_locale_expires_at IS NOT NULL
            AND synthetic_privacy_locale_token IS NOT NULL)
    );
