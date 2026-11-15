-- G-4 v2 character families: skills (public) and achievements, equipment and
-- outfit (owner-only). Additive only: two new tables, no existing row changes.
-- One current snapshot per (family, character); T24's public_projection_state
-- remains the shared epoch fence.
CREATE TABLE public_character_families_v2 (
    family text NOT NULL CHECK (family IN ('character_skills','character_achievements','character_equipment','character_outfit')),
    character_id uuid NOT NULL,
    source_authority text NOT NULL CHECK (octet_length(source_authority) BETWEEN 1 AND 128),
    epoch numeric(20,0) NOT NULL CHECK (epoch > 0),
    revision numeric(20,0) NOT NULL CHECK (revision > 0),
    observed bigint NOT NULL CHECK (observed >= 0),
    body jsonb NOT NULL CHECK (jsonb_typeof(body) = 'array' AND jsonb_array_length(body) <= 1),
    PRIMARY KEY (family, character_id)
);
CREATE TABLE public_character_families_v2_watermarks (
    family text PRIMARY KEY CHECK (family IN ('character_skills','character_achievements','character_equipment','character_outfit')),
    source_authority text NOT NULL CHECK (octet_length(source_authority) BETWEEN 1 AND 128),
    epoch numeric(20,0) NOT NULL CHECK (epoch > 0),
    complete_through bigint NOT NULL CHECK (complete_through >= 0)
);
