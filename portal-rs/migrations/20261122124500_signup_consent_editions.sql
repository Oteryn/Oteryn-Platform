-- Legacy and optional decisions had no attributable edition. Preserve them as recorded.
ALTER TABLE account_consents
    ADD COLUMN legal_page_id uuid,
    ADD COLUMN legal_locale text,
    ADD CONSTRAINT account_consents_legal_edition_shape CHECK (
        (legal_page_id IS NULL AND legal_locale IS NULL)
        OR (legal_page_id IS NOT NULL AND legal_locale IS NOT NULL
            AND legal_locale IN ('en', 'pl') AND document_kind IN ('terms', 'privacy'))
    ),
    ADD CONSTRAINT account_consents_legal_edition_fk
        FOREIGN KEY (legal_page_id, legal_locale, document_version)
        REFERENCES page_legal_versions (page_id, locale, version);
