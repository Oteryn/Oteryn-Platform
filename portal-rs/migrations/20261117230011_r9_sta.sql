-- Every existing positive bigint is exactly representable in numeric(20,0).
ALTER TABLE stats_character_daily
    ALTER COLUMN level TYPE numeric(20,0),
    ADD CHECK (level <= 18446744073709551615);
