-- Bootstrap-intent producer contract v4: the requested vocation is bound into the
-- intent. Additive and nullable: retained v1-v3 rows keep NULL and are not current
-- (their reads and retries fail closed); no vocation is backfilled.
ALTER TABLE character_bootstrap_intents ADD COLUMN requested_vocation text;
ALTER TABLE character_bootstrap_intents ADD CONSTRAINT character_bootstrap_intents_requested_vocation_check
    CHECK (requested_vocation IS NULL OR requested_vocation IN ('Knight', 'Paladin', 'Sorcerer', 'Druid', 'Monk'));
