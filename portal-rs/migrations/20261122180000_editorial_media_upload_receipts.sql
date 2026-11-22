CREATE TABLE editorial_media_upload_receipts (
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    upload_key uuid NOT NULL,
    fingerprint bytea NOT NULL CHECK (octet_length(fingerprint) = 32),
    PRIMARY KEY (account_id, upload_key)
);
