-- T72 ownership cuts. Additive only.
--
-- The initial ownership cut of a character is its committed CreateCharacter
-- receipt; the admission and read authorities find it by the receipt's
-- character id. Receipts are stored only after the canonical JSON check of the
-- characters command core, so every terminal receipt casts to jsonb.
CREATE INDEX character_commands_create_character
    ON character_commands ((receipt_json::jsonb->>'character_id'))
    WHERE command = 'CreateCharacter' AND state = 'TERMINAL';
