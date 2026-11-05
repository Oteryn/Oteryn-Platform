-- Nullable character ids preserve imported, unbound guild opt-outs.
CREATE TABLE character_profiles (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    character_id uuid,
    show_guild boolean NOT NULL DEFAULT true,
    is_main_character boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(account_id, character_id)
);
CREATE UNIQUE INDEX character_profiles_one_main ON character_profiles(account_id) WHERE is_main_character;
CREATE TABLE character_profile_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    event_type text NOT NULL CHECK (event_type IN ('character.profile_preferences_updated', 'character.main_character_selected')),
    created_at timestamptz NOT NULL DEFAULT now()
);
