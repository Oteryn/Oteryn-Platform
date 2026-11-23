ALTER TABLE accounts ADD COLUMN display_name text NOT NULL DEFAULT ''
    CHECK (char_length(display_name) <= 80);

CREATE INDEX accounts_email_prefix ON accounts (lower(email) text_pattern_ops) WHERE status<>'closed';
CREATE INDEX accounts_id_prefix ON accounts ((id::text) text_pattern_ops);
CREATE INDEX account_characters_name_prefix ON account_characters (lower(name) text_pattern_ops);
