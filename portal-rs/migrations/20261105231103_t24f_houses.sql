-- Isolated v2 bodies; the shared T24 epoch fence applies to both generations.
CREATE TABLE public_houses_v2 (
    subject text PRIMARY KEY,
    source_authority text NOT NULL,
    epoch numeric(20,0) NOT NULL CHECK (epoch > 0),
    revision numeric(20,0) NOT NULL CHECK (revision > 0),
    observed bigint NOT NULL CHECK (observed >= 0),
    payload jsonb NOT NULL
);
CREATE TABLE public_houses_v2_watermark (
    id smallint PRIMARY KEY CHECK (id = 1),
    source_authority text NOT NULL,
    epoch numeric(20,0) NOT NULL CHECK (epoch > 0),
    complete_through bigint NOT NULL CHECK (complete_through >= 0)
);
