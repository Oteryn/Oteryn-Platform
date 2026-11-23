CREATE INDEX character_commands_create_requests_by_account
    ON character_commands (account_id, source_revision DESC)
    WHERE command = 'CreateCharacter';
