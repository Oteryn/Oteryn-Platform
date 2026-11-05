CREATE TABLE public_projection_state (
    id smallint PRIMARY KEY CHECK (id = 1),
    highest_epoch numeric(20,0)
);
INSERT INTO public_projection_state (id) VALUES (1);
CREATE TABLE public_snapshots (
    family text NOT NULL CHECK (family IN ('highscores','character_profile','character_deaths','guild','house','world_online')),
    subject text NOT NULL,
    is_watermark boolean NOT NULL DEFAULT false,
    epoch numeric(20,0) NOT NULL CHECK (epoch > 0),
    version numeric(20,0) NOT NULL CHECK (version > 0),
    watermark bigint NOT NULL CHECK (watermark >= 0),
    payload jsonb NOT NULL,
    previous_snapshot jsonb,
    PRIMARY KEY (family, subject, is_watermark)
);
