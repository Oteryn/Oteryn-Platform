-- V2 deaths remain isolated from the closed v1 family schema.
CREATE TABLE public_deaths_v2 (
    subject text PRIMARY KEY,
    authority text NOT NULL,
    epoch numeric(20,0) NOT NULL CHECK (epoch > 0),
    revision numeric(20,0) NOT NULL CHECK (revision > 0),
    digest bytea NOT NULL,
    deaths jsonb NOT NULL
);
CREATE TABLE public_deaths_v2_watermark (
    id smallint PRIMARY KEY CHECK (id = 1),
    authority text NOT NULL,
    epoch numeric(20,0) NOT NULL CHECK (epoch > 0),
    complete_through bigint NOT NULL CHECK (complete_through >= 0)
);
-- Minimal immutable occurrence fences survive body retention and suppressions.
CREATE TABLE public_deaths_v2_occurrences (
    subject text NOT NULL,
    occurrence text NOT NULL,
    digest bytea NOT NULL,
    PRIMARY KEY (subject, occurrence)
);
-- Aggregate group budgets are stricter than per-certificate budgets and shared
-- by every process, with no certificate or payload identity persisted.
CREATE TABLE public_deaths_v2_rate_limits (
    route text PRIMARY KEY,
    window_start bigint NOT NULL,
    requests integer NOT NULL
);
